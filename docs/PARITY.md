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

## Spec 008 — memory & context parity

Rust adds an advisory context-management layer on the send side. Python Hermes
at the time of this slice has no first-class, persisted "summary turn"; these
are Rust-side behaviors, tracked in ADR 0003:

- **Advisory context accounting.** A `char/4` token estimator feeds `/info`
  (`estimated context ~N tokens`) and a non-blocking warning when the estimate
  exceeds the configured limit. This is advisory — it never blocks a request.
- **Sliding window (send-side only).** When the active window limit is set
  (`provider.context_length` > `model.context_length` >
  `compression.target_max_tokens`, per config precedence), `turns_to_send`
  returns a trimmed *copy*. `self.turns` and `state.db` always keep the full
  history, so `/messages` shows every turn even after many requests.
- **Pinned turns.** `/pin <n>` keeps a turn inside the window regardless of its
  age; pins are in-memory per-session (never written to `state.db`) and are
  cleared by `/new` and `/resume`. A pinned set that alone exceeds the budget is
  still sent (never dropped), surfacing a warning instead.
- **Compression wiring.** `compression.enabled: true` + `compression.target_max_tokens`
  turns the window on via the same context-limit channel; the default is off,
  so configs written before this field behave unchanged. `/info` shows
  `compression: on/off`.
- **Summarization (display-only).** `/info` summarizes the turns the window
  would drop. The summary is heuristic, redacted/sanitized, and **never
  injected into the model context**; ADR 0003 fixes the future
  `Turn::Summary` representation so no fake `User` turn is ever introduced.

## Spec 009 — planning, reflection & recovery parity

Rust adds a guided multi-turn loop (goal → plan → execute → reflect → recover →
done) layered on top of the Spec 002 agentic loop. Python Hermes at the time of
this slice has no equivalent goal/plan/reflection engine; all of this is
Rust-side behavior. The guidance lives in **in-memory runner state plus
ephemeral instructions** — it is never a new persisted role and never a fake
`User` turn:

- **Goal extraction & tracking (`/goal`).** Off by default. When enabled, the
  first user prompt of an agentic session is recorded (char-safe, capped) as the
  active goal with a lifecycle (`NotStarted → InProgress → Achieved/Blocked`).
  State is in-memory and advisory; it is never written to `state.db`.
- **Plan-then-execute (`/plan`).** Off by default. Planned mode sends one
  ephemeral instruction round (no user turn, no system-prompt activation) that
  asks the model for a `[[plan]]…[[/plan]]` step list; the parsed plan is kept in
  memory and re-supplied to the model during execution via the ephemeral
  instruction channel (ADR 0004). A plan shares the iteration budget (≤ 10).
- **Self-reflection gate (`/reflect`).** Off by default. After each tool result a
  deterministic heuristic classifies on-plan / off-plan / blocked and applies it
  to the goal lifecycle: `Success → on-plan`, `Denied → blocked` (never
  retried), and retryable `Error`/`Timeout` → off-plan (recover) up to an
  anti-loop cap. An active, in-progress goal is marked `Achieved` only when the
  guided (reflection-on) loop finishes normally with a tool-free answer.
- **Error recovery via parameter mutation.** When a tool fails retryably,
  `RetryTracker` records the FNV-1a fingerprint of the argument set and the
  runner rejects an *identical* repeat before re-executing it, annotating the
  tool result with an "already tried" note so the model picks different
  parameters (ADR 0005). Distinct failures are bounded (`MAX_RETRIES = 3`); when
  exhausted the step is `Blocked` and the loop early-stops via the new
  `AgenticResult::Blocked` variant (distinct from `MaxIterations`). `Denied`
  tools are never recorded/retried.
- **Off by default / zero regression.** With `/goal`, `/plan`, and `/reflect`
  all off, the tool loop is byte-for-byte the Spec 002 reactive agentic loop.

## Spec 011 — MCP client parity

Rust connects to Model Context Protocol (MCP) servers as a client so it can use
the ecosystem of existing MCP servers (GitHub, PostgreSQL, search, etc.) without
writing each tool by hand. Python Hermes at the time of this slice has no MCP
client, so this is Rust-side behavior:

- **Config.** `mcp_servers: { name: { command, args, env, confirm } }` in
  `config.yaml`. Empty by default → nothing spawns (zero regression).
- **Transport.** JSON-RPC 2.0 over newline-delimited stdio to a spawned child
  process. The client is built over a small transport seam so protocol logic is
  unit-tested without a real child; production uses the child's stdin/stdout.
- **Lifecycle.** Startup spawns each configured server, runs `initialize` /
  `notifications/initialized`, discovers tools via `tools/list`, and registers
  each as a Hermes `Tool` named `{server}__{tool}` in the same `ToolRegistry`
  used by the Spec 002/009 agentic loop. Execution forwards `tools/call` to the
  child; results are flattened back to `ToolResponse`. Child processes are
  killed on drop when the session ends.
- **Security (Spec 011b hardening).** Server config is trusted input; `env`
  secrets are redacted on every display path and `${VAR}` placeholders are
  expanded from the environment at spawn (an unset var errors naming the var).
  Confirmation **defaults to ON** (secure-by-default): a server runs its tools
  through the Spec 002 confirmation gate unless it sets `confirm: false`
  explicitly (a decline is `Denied`, never retried). Inbound JSON-RPC messages
  over 10 MB are rejected (message-size limit), and per-call timeouts bound a
  hanging server. `/mcp` and `/mcp restart <name>` list/inspect and restart
  servers (see `docs/SECURITY.md`).

## Spec 012 — TUI dashboard parity

Hermes-RS adds an **opt-in Ratatui terminal dashboard** (`--tui`) that the
Python Hermes at the time of this slice does not have. This is an *extra*
Rust-side capability rather than a parity gap: the default Rust entry point
remains the readline REPL (identical to before), and `--tui` is an alternative
front end over the **same** single agentic engine, the same `state.db`, and the
same provider/config as the REPL — it never forks a second loop.

- **Core observer.** A UI-free `AgentEvent` observer on `ConversationRunner`
  (`crates/hermes-core/src/conversation/events.rs`) streams chunks, tool
  start/done, iteration, status and token ticks. It is additive and
  default-`None` (REPL/tests unaffected), and is a domain contract any future
  front end can consume.
- **Sanitization boundary.** Raw `AgentEvent`s are scrubbed (ANSI/control
  stripped + credentials redacted) at the CLI boundary in
  `crates/hermes-cli/src/tui/worker.rs` before becoming display `TuiEvent`s;
  the renderer never sanitizes.
- **Panels.** Streaming transcript, tool log, status header (token meter,
  provider, session, goal/plan/reflection), scrollable transcript, single-line
  input with history. Headless E2E via `TestBackend` covers a full simulated
  session, credential/ANSI sanitization, and Ctrl-C → exit-130 mapping.

Python-side equivalent of a terminal dashboard is out of scope for this Rust
rewrite.

## Spec 013 — UI parity (visual)

Goal: the Hermes-RS default look & feel ("gold and kawaii") is visually
identical to the Python Hermes default skin. Verified 2026-09-05 by
side-by-side comparison: live `pty` captures of `hermes-rs` (REPL at 50/60/80/
100/110 columns, TUI at 100 columns; raw bytes incl. SGR escapes) checked
element-by-element against (a) the verbatim Python captures in
`docs/HERMES_UI_SPEC.md` (T01, taken on this VM) and (b) the Python sources
under `~/.hermes/hermes-agent/` (`cli.py`, `hermes_cli/banner.py`). The Python
installation was read-only throughout (`smoke_python_hermes_untouched`).

| Element | Python default skin (ground truth) | Hermes-RS (live pty capture 2026-09-05) | Status |
|---|---|---|---|
| Figlet logo `HERMES_AGENT_LOGO` (shown ≥95 cols) | 6 rows, 101/101/98/98/98/98 cells; rows 1-2 `bold #FFD700`, 3-4 `#FFBF00`, 5-6 `#CD7F32` (`cli.py` `HERMES_AGENT_LOGO`) | Row-for-row byte-identical text + identical SGR tiers (110-col capture); pinned by `logo_is_verbatim_bytes` | 100% (edge: at exactly 100 cols the 101-cell panel buffer clips the last cell of rows 1-2; ≥101 cols renders full — verified at 110) |
| Caduceus hero art | 15 × 30-cell rows, gradient `#CD7F32` / `#FFBF00` / `#FFD700` / `#B8860B` (`banner.py` `HERMES_CADUCEUS`) | 15/15 rows identical in text and color; pinned by `caduceus_is_verbatim_bytes` | 100% |
| Banner panel border | `banner_border` `#CD7F32` | border emitted as `38;2;205;127;50` | 100% |
| Prompt symbol | `❯ ` with empty style — inherits the terminal (`'prompt': ''`) | `PROMPT_SYMBOL "❯ "` plain, inherits the terminal | 100% |
| Response box (streaming) | `╭─ ⚕ Hermes ──╮` / `╰──╯` bold accent `#FFD700`; body `banner_text` `#FFF8DC`; no side borders; width = longest line, clamped to terminal | Byte-identical frames (`1;38;2;255;215;0` header/footer, `38;2;255;248;220` body), same width rule (T04 tests) | 100% |
| Reasoning box | `┌─ Reasoning ─┐` dim + italic, always before the response box | Byte-pinned in T04 end-to-end tests (the `fake` provider has no reasoning trigger, so no live capture; zero-change closure constraint) | 100% (test-verified) |
| Tool-activity line | `  ┊ ◇ {header}` dim | `  ┊ ◇ {name}` with SGR `2;3` (dim + italic) | 100% |
| Spinner | braille `dots`, 120 ms, `\r  {frame} {message} ({elapsed:.1}s)` | Same frame set, rate and format (injectable-clock unit tests); fast `fake` responses (<120 ms) clear the spinner before a frame is drawn, so captures show the cleared line | 100% (test-verified) |
| Status bar — **all visual customizers: 100% compatible** | 3 width tiers (<52 / <76 / ≥76 cols); ` · ` vs ` │ ` separators; context gauge `used/total` + 10-block bar `[████░░░░░░]`; thresholds Dim (unknown) / Good `#8FBC8F` <50 / Warn `#FFD700` ≥50 / Bad `#FF8C00` >80 / Critical `#FF6B6B` ≥95; Python banker's rounding; full-width navy `#1a1a2e` line, padded or trimmed with `...`; ` ─ {title}` right badge suppressed <24 cols | Tiers, separators, gauge + bar, color thresholds (incl. banker's rounding via `py_round`), full-width `48;2;26;26;46` line — raw SGR confirmed in the 50/60/80/100-col captures; 22 unit tests (T05) | 100% |
| Goodbye line | `Goodbye! ⚕` (`cli.py:18000`) | `Goodbye! ⚕` (80-col capture) | 100% |
| Brand strings | "Hermes Agent …" | "Hermes-RS …" (T02 decision, approved) | Intentional difference |
| User-message echo | `─`×40 accent separator + `●` bold-text line in the scrollback | Prompt echo `❯ <text>`; no scrollback line | Intentional structural difference (line-REPL echo model) |
| Agentic iteration marker | not printed | dim `[iter N/10]` line | Rust-only informational |
| CLI surface (`--help` / `--version`) | argparse help (~90 subcommands), `Hermes Agent vX …` | Subset help + `hermes-rs 0.1.0` | Intentional (Spec 013 scope = in-app look & feel; the display-mode flag `--tui` exists in both) |

Status-bar visual customizers are marked **100% compatible**: tier
thresholds, separators, context-style thresholds, gauge rendering, the title
badge and the full-width pad/trim behavior all match the Python defaults.
Runtime data the Rust engine does not have (compression counter, background
tasks, YOLO, focus, session title) is hidden — exactly as Python hides absent
segments — and the renderer is data-driven, so each segment appears
automatically when the data exists.

Capture artifacts (VM `/tmp/t06_out/`, 2026-09-05): `t06_rs_repl_{50,60,80,100,110}.bin`
(raw pty bytes, SGR included), `t06_rs_tui_100.bin`, plus `--help`/`--version`
stdout; Python reference outputs at `/tmp/t06_py_help.txt` and
`/tmp/t06_py_version.txt` (venv interpreter, read-only).

## Differences ⚠️

| Feature | Python | Rust | Impact |
|---|---|---|---|
| Session ID format | UUID v4 | UUID v7 | Low — schema-compatible and time-sortable |
| FTS index | Enabled | Not yet | Low — search is not implemented |
| Provider catalog | Many built-ins + plugins | Config-declared + built-in `fake` | Medium — Rust has no dynamic plugin loading |
| Tool execution | Python sandbox | Native shell integration | High — different security model |
| TUI dashboard | Rich/curses terminal output only (no dedicated dashboard) | Opt-in Ratatui `--tui` dashboard + readline REPL | Rust-only capability (Spec 012) |

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
