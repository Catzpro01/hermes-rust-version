# Hermes-RS Parity Report

## Compatible ✅

| Feature | Status | Notes |
|---|---|---|
| Hermes home resolution | ✅ | `HERMES_HOME` > explicit CLI path > `~/.hermes` |
| `config.yaml` parsing | ✅ | Read-only, compatible schema |
| SQLite `state.db` | ✅ | Hermes schema, WAL mode, concurrent writes |
| Session create/resume/list | ✅ | UUID v7 (Python uses UUID v4) |
| OpenAI-compatible provider | ✅ | `/v1/chat/completions`, Bearer auth, redaction |
| Streaming response | ✅ | SSE chunks rendered as they arrive |
| Cancellation | ✅ | Ctrl-C cancels active HTTP stream; partial turn is discarded |
| Config-declared providers | ✅ | `providers:` in `config.yaml`, selected by `--provider` / `model.provider` |
| Per-provider credential env | ✅ | `key_env` names the env var a provider uses (see Spec 005 STRIDE) |
| Mid-session provider switch | ✅ | `/provider <name>` preserves session; failures roll back |
| Wire-mode routing | ✅ | `api_mode: chat_completions | completions` selects endpoint/payload |

## Spec 005 — provider routing parity

Rust routing now matches the config-declared, per-provider model the installed
Python Hermes uses for user-defined `providers:` entries:

- **Selection.** Both resolve the active provider from config (CLI override
  wins over `model.provider`, falling back to a default). Rust falls back to
  the built-in offline `fake` when nothing is configured.
- **Credential.** Python providers read their key from per-provider env vars;
  Rust reads `key_env` with an explicit fallback chain
  (`key_env` → `model.api_key` → error). A pinned-but-empty `key_env` errors
  rather than silently borrowing another key, preventing cross-provider
  credential leakage.
- **Mid-session switch.** Python persists a session across provider/model
  changes; Rust `/provider` swaps the backing provider on `ConversationRunner`
  without touching `self.turns`, and only at a turn boundary. A provider whose
  construction fails is rolled back, leaving the active one untouched.
- **Wire mode.** `api_mode` selects between the chat-completions and the legacy
  completions endpoints; streaming is normalized to the same provider-neutral
  event sequence so a switch is transparent to callers.

## Spec 006 — retry, fallback & health parity

Rust now absorbs transient upstream failures and automatically routes around a
failing provider, layered on top of the Spec 005 provider model:

- **Bounded retry.** `HttpProvider` retries transient pre-stream errors (429
  and 5xx, per `ProviderError::is_retryable`) with a bounded exponential
  backoff (`RetryPolicy` default: 3 attempts, 200 ms base, 2 s cap). Retry only
  happens before a stream starts, so no partial turn is ever retried.
- **Fallback chain.** `model.fallback_chain: [b, c]` declares the ordered
  providers to try after the active one. `FallbackProvider` is a `Provider`
  wrapper, so `ConversationRunner` and the REPL keep seeing a single provider.
  A hop is retried per its own policy first; only after it exhausts (or errors
  permanently) does the chain move on with the same `turns` from the start. An
  unknown chain name is rejected at startup; a hop that fails to *build* is
  skipped cleanly as long as its name was declared. Startup resolves via
  `select_with_fallback`; the manual `/provider <name>` switch stays
  single-provider, so an explicit user choice bypasses fallback.
- **Per-hop credential isolation.** Each hop is built by the registry and uses
  its own `key_env`/`model.api_key`, so provider A's key never reaches
  provider B's endpoint. This is proven by two-server wiremock tests asserting
  each endpoint only ever sees its own `Authorization` header.
- **Health / cooldown.** An in-memory `HealthTracker` records a provider that
  fails (after its retries) as cooling down for a bounded window (default
  60 s). `FallbackProvider` skips a cooling-down hop, so a struggling endpoint
  is not hammered repeatedly in one session. `Cancelled` is never recorded as a
  failure, and manual `/provider` bypasses cooldown. State is process-lifetime
  only — never written to `state.db`.

Python Hermes has no equivalent automatic retry/fallback/health layer in the
Rust parity slice; these are Rust-side resilience behaviors. If the active
request ultimately fails on every hop, `ProviderError::Fallback` names the
providers that were tried.

## Differences ⚠️

| Feature | Python | Rust | Impact |
|---|---|---|---|
| Session ID format | UUID v4 | UUID v7 | Low — schema-compatible and time-sortable |
| FTS index | Enabled | Not yet | Low — search is not implemented |
| Provider catalog | Many built-ins + plugins | Config-declared + built-in `fake` | Medium — Rust has no dynamic plugin loading |
| Tool execution | Python sandbox | Native shell integration | High — different security model |
| TUI rendering | Rich/curses | Plain stdout | Low — cosmetic |

## Known Gaps 🚧

- Function calling / tool use (planned Spec 002)
- FTS5 search on message content
- Dynamic plugin/provider loading
- Conversation branching and edit

## Testing

```bash
cargo test --test cli_e2e
cargo test --test smoke
./scripts/smoke-test.sh
```

## Verification notes

The checked-in Hermes-compatible fixture reports `sessions.id` as `TEXT`, with UUID values stored as 36-character strings. The live Python Hermes home on the validation VM did not contain `state.db`, so no claim is made about an existing live Python session database.

The config audit covers the currently supported Hermes-RS schema: `model.default`, `model.provider`, `model.base_url`, and `model.api_key`. Unknown fields remain ignored by serde for forward compatibility.

## Operational SIGINT verification

The Unix integration test `sigint_stream` synchronizes on the first SSE chunk before sending `SIGINT`. It verifies exit code 130 and zero persisted assistant messages after cancellation.

## Spec 002 tool status

| Capability | Status | Notes |
|---|---|---|
| XML tool-call parsing | ✅ | Streaming tag-aware parser |
| Agentic loop | ✅ | Cancellable, maximum 10 iterations |
| SQLite tool-call records | ✅ | Tool name, arguments, result, status |
| Read-only filesystem tools | ✅ | cwd jail and output limits |
| Confirmed writes | ✅ | Atomic write and default-deny confirmation |
| Readonly shell policy | ✅ | Blocklist, timeout, cancellation |
| Security documentation | ✅ | See `docs/SECURITY.md` |
