# hermes-core: provider module architecture

Module path: `crates/hermes-core/src/provider/`

The provider module is the **provider-neutral asynchronous chat contract** for Hermes. It turns "talk to an LLM endpoint" into a small stack of composable layers: a config-driven registry, a fallback chain with health tracking, a retrying HTTP provider, an SSE normalizer, and a streaming tool-tag parser. Downstream consumers (the conversation runner, the REPL) hold a single `Box<dyn Provider>` and never need to know how many providers sit behind it.

## Module map

| File | Lines | Role |
|------|------:|------|
| `mod.rs` | 202 | Core contract: `Provider` trait, `ProviderError`, retryability, `tool_aware_stream` |
| `registry.rs` | 562 | Config-driven provider selection (lazy factories, `select`, `select_with_fallback`) |
| `fallback.rs` | 497 | `FallbackProvider`: ordered chain with health-gated hop skipping |
| `http.rs` | 506 | `HttpProvider`: OpenAI-compatible wire client with bounded retry/backoff |
| `sse.rs` | 115 | SSE `data:` framing + wire-mode-neutral event normalization |
| `health.rs` | 156 | `HealthTracker`: in-memory per-provider cooldown (lightweight circuit breaker) |
| `fake.rs` | 48 | `FakeProvider`: deterministic offline provider for tests/local dev |
| `redact.rs` | 21 | Credential redaction for diagnostic messages |

## Core contract (`mod.rs`)

### `Provider` trait

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(&self, turns: &[Turn]) -> Result<EventStream, ProviderError>;
    async fn chat_with_cancel(&self, turns: &[Turn], cancel: CancellationToken)
        -> Result<EventStream, ProviderError> { self.chat(turns).await }
    async fn chat_with_instruction(&self, turns: &[Turn], instruction: Option<&str>,
        cancel: CancellationToken) -> Result<EventStream, ProviderError>
        { self.chat_with_cancel(turns, cancel).await }
}
```

- **`EventStream`** = `Pin<Box<dyn Stream<Item = Result<Event, ProviderError>> + Send>>`.
- **`chat_with_cancel`** — cancellation is a first-class concern: implementations honor the token both before the request and mid-stream.
- **`chat_with_instruction`** (Spec 009, Ticket 02) — ephemeral instruction channel for plan generation. The instruction is never persisted, never a user turn, never canonical history. The default implementation ignores it (backward compatible); `HttpProvider` overrides it (prepended as a `system` message in chat mode, or an `[Instruction]` header in completions mode).
- A `Provider` impl for `Box<T>` makes dynamic dispatch transparent.

### `ProviderError` and retryability

| Variant | Retryable? | Notes |
|---------|-----------|-------|
| `Message(String)` | no | generic transport/diagnostic error |
| `Http { status, message }` | if status ∈ `RETRYABLE_HTTP_STATUS` | explicit set: **429, 500, 502, 503, 504** (pinned by test) |
| `Timeout` | yes | distinct from `Message` so it can be classified |
| `Cancelled` | no | user cancellation is never retried |
| `Fallback { tried }` | no | terminal for the request; each hop already ran its own retries |

## The provider stack (layered view)

```
config.yaml / CLI flags
        │
        ▼
┌─────────────────────┐   lazy factories; no I/O at registration
│  ProviderRegistry   │   (misconfigured provider can't block startup)
└─────────────────────┘
        │  select_with_fallback (cli > config model.provider > "fake")
        ▼
┌─────────────────────┐   ordered hops; health-gated skipping
│  FallbackProvider   │   whole-turn retry on next hop; never partial
└─────────────────────┘        output across hops
        │  per hop
        ▼
┌─────────────────────┐   bounded exponential backoff (3× default,
│    HttpProvider     │   200ms base, 2s cap) on pre-stream retryable errors
└─────────────────────┘
        │  2xx response
        ▼
┌─────────────────────┐   "data:" line framing + partial-line remainder;
│       sse.rs        │   delta.content | text → Event::Chunk; [DONE] → Done
└─────────────────────┘
        │
        ▼
┌─────────────────────┐   buffers chunks; splits <tool_call …> XML out of
│ tool_aware_stream   │   the text stream → Event::ToolCall (mode-agnostic)
└─────────────────────┘
        │
        ▼
   EventStream (Event::Started / Chunk / ToolCall / Done)
```

## Registry: config-driven selection (`registry.rs`)

- **Lazy factories.** `ProviderRegistry` stores `name → Box<dyn Fn() -> Result<Box<dyn Provider>, RegistryError>>`. `from_config` performs **no I/O and reads no env vars**; anything that can fail per-provider is deferred to `build`. One misconfigured provider in `config.yaml` therefore cannot prevent startup or take down the provider in use.
- **Built-in `fake`.** `FAKE_PROVIDER = "fake"` is always registered unless a config entry shadows that name (shadowing wins; the config entry then needs real credentials).
- **Selection precedence** (`select`): `--provider` CLI flag > `model.provider` from config > built-in `fake`. `base_url_override` (`--api-url`) feeds the model-level fallback path.
- **Model-level legacy fallback.** If the chosen name is not under `providers:`, `model_level_fallback` tries the pre-`providers` layout: `model.base_url` (default `https://api.openai.com/`) plus a key from `OPENAI_API_KEY` → `HERMES_API_KEY` → `model.api_key`. If nothing usable, an unrequested selection resolves to `fake`; an explicitly requested unknown provider is an `UnknownProvider` error naming the available ones.
- **Fallback chain construction** (`select_with_fallback`):
  - Only engages when the active provider is a registered `providers:` entry.
  - **Strict startup validation**: every name in `model.fallback_chain` must be registered — a typo is a startup error, not a silent skip.
  - The primary's construction failure propagates; a **fallback hop whose construction fails is dropped** so a misconfigured backup never takes down a working primary.
  - Single surviving hop → the plain provider is returned **unwrapped** (no fallback indirection for a single-provider session); otherwise `FallbackProvider::new(hops)`.
- **Model choice determinism**: `models` is a `HashMap`; the registry sorts names and picks the first, so construction is deterministic.
- **`api_mode`**: strict tagged enum on `ProviderConfig` — an unknown value is rejected **at config parse time** (schema error echoing the bad value), not at build/request time. Absence defaults to `chat_completions` (backward compatible).
- **Errors**: `UnknownProvider { name, available }`, `NoneSelected`, `Construction { name, reason }`. Messages **name variables, never values** (credential-confusion guard, Spec 005 / STRIDE).

### Credential resolution (`resolve_api_key`)

1. `key_env` pinned (declared, non-empty) and the variable holds a non-empty value → use it.
2. `key_env` pinned but the variable is unset/empty → **error naming the variable**. It must *not* silently fall back to `model.api_key` — doing so would risk sending one provider's credential to another provider's endpoint (cross-provider credential leakage / Spoofing).
3. `key_env` absent → global `model.api_key` → error if neither is available.

## Fallback chain (`fallback.rs`)

`FallbackProvider` wraps an ordered hop list `(name, Box<dyn Provider>)` plus a shared `Arc<HealthTracker>`:

- **Semantics**: each hop is a full provider that already exhausted its own retries. On pre-stream failure the **whole turn is retried from its start** on the next hop — the same `turns` slice goes to every hop; partial output is never carried across hops.
- **Hop skipping** (Ticket 05): a hop cooling down from a recent failure is skipped and does *not* count as "tried" in the aggregate error.
- **Cancellation never falls through**: if the token fires before/during any hop, `Cancelled` is returned immediately and is never recorded as a failure.
- **Health updates**: a failed hop is recorded cooling; a successful hop clears its prior failure (immediate re-eligibility).
- **Aggregate failure**: `ProviderError::Fallback { tried }` names every provider actually attempted.
- **Transparency**: the REPL/runner hold one `Box<dyn Provider>`; `FallbackProvider` implements the trait, so the chain is invisible downstream. `chat_with_instruction` forwards the ephemeral instruction to every hop.

## HTTP provider (`http.rs`)

`HttpProvider { client, base_url, api_key: SecretString, model, api_mode, retry }` — an OpenAI-compatible client speaking either wire format:

- **API modes** (decided once at construction via `with_api_mode`):
  - `chat_completions` (default) → `POST {base}/v1/chat/completions`, body `{model, stream: true, messages}`; token text arrives in `delta.content`.
  - `completions` (legacy) → `POST {base}/v1/completions`, body `{model, stream: true, prompt}`; token text arrives in `text`. The prompt is a deterministic linear transcript (`User: …\nAssistant: …\nTool result (name): …\n`) ending with an unclosed `Assistant:` cue; an ephemeral instruction becomes an `[Instruction] …` header.
  - Both modes yield the **same provider-neutral `Event` sequence**, so `tool_aware_stream` and everything downstream is mode-agnostic.
- **Ephemeral instructions** (`chat_with_instruction`): prepended as a `system` message in chat mode.
- **Request attempt (`attempt`)**: send under `tokio::select!` against the cancel token; transport timeout → `ProviderError::Timeout` (retryable); other transport errors → `Message(redact(…))`; non-2xx → `Http { status, redacted body }`. Errors are classified **before any stream is consumed**, so retry/fallback react cleanly.
- **Bounded retry (`send_with_retry`)**: retries only pre-stream retryable errors; `RetryPolicy { max_attempts: 3, base_delay: 200ms, max_delay: 2000ms }` (defaults pinned by test). Backoff is `min(base × 2^(attempt−1), max)` computed in `u128` with the exponent clamped to 20 (no overflow). The backoff sleep runs inside `select!` against the cancel token — SIGINT mid-backoff exits `Cancelled`, it never waits out the timer.
- **Stream consumption (`chat_with_cancel_raw`)**: yields `Event::Started`, then feeds response bytes through `sse::parse_chunk` with a remainder buffer for partial lines; returns on `Event::Done`; flushes any remainder. The `chat`/`chat_with_cancel`/`chat_with_instruction` impls wrap this in `tool_aware_stream`.
- **Client timeout**: the default reqwest client carries a 30-second per-request timeout (DoS vector; surfaces as retryable `Timeout`). `with_client` allows injecting a custom client (tests).
- **Cancellation through `Box<dyn Provider>`**: `chat_with_cancel` is explicitly overridden because the trait default (`self.chat()`) would build a fresh token and silently ignore the caller's cancellation under dynamic dispatch.

## SSE normalization (`sse.rs`)

- `parse_data`: `data: [DONE]` → `Event::Done`; JSON payload → first choice; content from `delta.content` (chat) or `text` (completions); a `finish_reason` with no content also maps to `Done`; malformed JSON → `Message("malformed SSE payload: …")`.
- `parse_chunk`: line-oriented framing over arbitrary byte chunks; keeps an incomplete-line remainder for the next call; only `data:` lines with non-empty payloads produce events.
- This is the single place that knows the wire format — everything above it is wire-mode-neutral.

## Health tracker (`health.rs`)

`HealthTracker` is a lightweight, **in-memory** circuit breaker:

- `DEFAULT_COOLDOWN = 60s`; `new(cooldown)` for custom values.
- `record_failure(name)` starts a cooldown window; `record_success(name)` clears it immediately (recovered provider re-enters rotation at once); `is_cooling_down(name)` checks the window against `Instant`.
- State lives for the process lifetime only — **never persisted to `state.db`** (which stays the sole canonical store). Failures are recorded by provider name, never by credential or value. `Send + Sync` via a `std::sync::Mutex`; consulted only on hop failure/skip, so it is not a hot path. `Cancelled` is never recorded as a failure.

## Tool-tag streaming (`mod.rs::tool_aware_stream`)

A stream transformer that converts raw token text containing Hermes XML tool tags into typed events:

- Buffers `Event::Chunk` text; text before a `<tool_call` marker is yielded immediately; a complete `<tool_call …>…</tool_call>` block is parsed by `parse_tool_events` and yielded as `Event::ToolCall` (parse failure → the raw XML is emitted as a chunk, never swallowed).
- Handles markers **split across chunk boundaries**: if the buffer ends with a partial `<tool_call` prefix, that suffix is held back for the next chunk.
- Non-`Chunk` events pass through untouched. Applied by `HttpProvider` (all three chat methods) and `FakeProvider`, so every stream — fake or real — is tool-aware.

## Fake provider (`fake.rs`)

Deterministic offline provider for tests and local development (no config, no credentials):

| Last user input | Stream produced |
|---|---|
| `"tool"` | a `<tool_call id="fake-1">read_file: {"path":"Cargo.toml"}</tool_call>` chunk (exercises the tool path) |
| `"error"` | `Started` then `Err(Message("simulated"))` (exercises mid-stream error) |
| any turn with a `Tool` turn | `"tool completed"` |
| anything else | `"echo: {input}"` |

## Cross-cutting concerns

| Concern | Mechanism |
|---|---|
| **Cancellation** | `CancellationToken` honored in three places per request: pre-send, during backoff sleep, and per SSE byte. `Cancelled` is never retried, never falls through the fallback chain, never recorded as a health failure, and maps to exit code 130 at the CLI boundary. |
| **Resilience (3 layers)** | 1) per-hop bounded backoff retry inside `HttpProvider`; 2) whole-turn hop fallback in `FallbackProvider`; 3) health-gated hop skipping (`HealthTracker`) so a struggling endpoint isn't hammered within a session. |
| **Credentials** | `SecretString` everywhere; `key_env` pinning with the credential-confusion guard; error messages name variables, never values; `redact()` strips the key from all transport/upstream error text (`***REDACTED***`). |
| **Wire neutrality** | `api_mode` fixes endpoint/body/token-field per provider; `sse.rs` normalizes both modes into one `Event` sequence; downstream is mode-agnostic. |
| **Backward compatibility** | `fake` default; missing `config.yaml` allowed (offline slice); model-level config fallback for pre-`providers` files; absent `api_mode` → `chat_completions`; trait defaults keep old providers working. |

## Tests (per file)

- `mod.rs` — retryable-status set is exact (429/500/502/503/504; 501 and other 4xx excluded); `Timeout` retryable, `Cancelled`/`Message`/`Fallback` not.
- `registry.rs` — offline `fake`; registration without credentials (deferred build); unknown-provider errors listing available names; CLI > config > fake precedence; config shadowing of `fake`; pinned-but-empty `key_env` errors naming the var and **never** mentioning the fallback value; absent `key_env` uses `model.api_key`; unknown `api_mode` rejected at parse; `completions` parses to the tagged enum; deterministic model choice; `select_with_fallback` offline single-hop, strict rejection of unknown fallback names.
- `http.rs` — completions transcript rendering (linear, `Assistant:` cue, instruction header only when present); system instruction prepended only when present; 30s client timeout pinned; default retry policy pinned; backoff doubles then caps (u128 safe at large exponents).
- `sse.rs` — chat `delta.content` + `[DONE]`; completions `text` + `finish_reason`; partial-line remainder across chunks; malformed payload rejected.
- `fallback.rs` — primary success never calls later hops; primary failure moves on; permanent non-retryable errors also fall through (fallback is between different endpoints); cancellation short-circuits; health skip/record/clear behavior; aggregate `Fallback { tried }` naming.
- `health.rs` — default cooldown pinned (60s); failure marks cooling until the window elapses (explicit instants, no sleeping); success clears immediately; providers tracked independently.
- `redact.rs` — key value replaced, original absent from output.
- `fake.rs` — deterministic streams (via `Provider` contract tests in the runner suite).
