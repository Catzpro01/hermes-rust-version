# Hermes-RS Roadmap

Status of the staged rewrite described in [`CONTEXT.md`](../CONTEXT.md).

## Phase status

| Phase | Spec | Scope | Status |
|---|---|---|---|
| 1 — Foundation | 001 | CLI, config, session, streaming | Done |
| 1 — Foundation | 002 | Tool calling, agentic loop | Done |
| 2 — Inspection & search | 003 | Session/message inspection CLI | Done |
| 2 — Inspection & search | 004 | FTS5 full-text search | Done |
| 3 — Multi-model & routing | 005 | Multi-provider runtime routing | Done |
| 3 — Multi-model & routing | 006 | Model fallback and load balancing | Done |
| 4 — Advanced agent | 007 | Tool execution sandbox | Not started |
| 4 — Advanced agent | 008 | Memory and context management | Done |
| 4 — Advanced agent | 009 | Multi-turn planning and reflection | Done |
| 5 — Ecosystem | 010 | Plugin/extension system (WASM) | Not started |
| 5 — Ecosystem | 011 | MCP client | Done |
| 5 — Ecosystem | 012 | TUI dashboard (ratatui) | Done |

## Spec 004 closure

The last open ticket under Spec 004 was the end-to-end proof that `/search`
never emits a raw credential. That proof exists in
`crates/hermes-cli/tests/search_credential_safety.rs`
(`search_does_not_leak_credentials`) and covers all four required properties:
a canonical message carrying a credential, a `/search` invocation, stdout free
of the raw value and of ANSI escapes, and unchanged canonical tables.
Landed in `2852e6d`; the query display was redacted in `1239e8b`.

A follow-up review found a related defect in the same boundary:
`redact_pattern` matched `sk-` as a bare substring, so the short-key threshold
could not be lowered without masking ordinary words such as `ask-anything`.
`58ca282` adds a token-boundary check and lowers the thresholds
(`sk-proj-` 12, `sk-` 8), with regression tests for short keys,
punctuation-delimited keys, and false positives.

## Spec 005 closure

Spec 005 ("Multi-provider runtime routing") is complete. Providers are now
resolved from `config.yaml` rather than a literal name match, credentials come
from each provider's own `key_env`, `HttpProvider` routes its endpoint and
payload by a strict `api_mode` enum, and the REPL switches providers mid-session
with `/provider <name>`.

The five tickets and their landing commits:

| Ticket | Scope | Commit |
|---|---|---|
| 01 | Config-driven provider registry | `d9aeec2` |
| 02 | Per-provider `key_env` resolution + STRIDE | `5e25da3` |
| 03 | `api_mode` endpoint/payload routing | `f646f65` |
| 04 | `/provider <name>` mid-session switch | `f15e61d` |
| 05 | Parity, docs, E2E closure proof | latest |

Closure proof (Spec 005): `crates/hermes-cli/tests/provider_routing_e2e.rs`
drives the real binary against two wiremock-backed providers. It starts on one
provider, switches mid-session to the other, and asserts both responses land in
the same `state.db` session in order, that neither provider's credential
appears on any output path, and that a failed switch keeps the active provider
(and its credential) unchanged.

## Spec 006 closure

Spec 006 ("Model fallback and load balancing") is complete. Providers absorb
transient upstream failures with a bounded exponential backoff, fall back to an
ordered `model.fallback_chain` of providers when the active one fails, and
track per-provider health in memory so a recently-failing endpoint is not
hammered repeatedly in one session. An advisory-only context-length estimator
(`char/4`) is available for later memory/context work. A `BEGIN IMMEDIATE`
transaction in `SessionStore::save_turn` also removed a class of
rollback-journal lock-ordering deadlocks that made the concurrent-write test
flaky under load.

The seven tickets and their landing commits:

| Ticket | Scope | Commit |
|---|---|---|
| 01 | Error taxonomy & client timeout | `9ad81d8` |
| 02 | Bounded exponential-backoff retry | `e4dfa6a` |
| 03 | Ordered provider fallback chain | `e4c4d5f` |
| 04 | Advisory context-length estimation | `d79518c` |
| 05 | In-memory per-provider cooldown | `f2b8d9c` |
| 06 | SQLite IO hardening (`BEGIN IMMEDIATE`) | `88c812e` |
| 07 | Parity, docs, E2E closure proof | this commit |

Closure proof (Spec 006): `crates/hermes-core/tests/provider_fallback_integration.rs`
drives a config-declared `model.fallback_chain` through the registry into a live
`FallbackProvider`. With provider A persistently down, the request falls back to
provider B over the wire, B's response is the one served, and two-server
assertions prove A's key never reaches B and vice-versa. Companion tests cover a
down-provider being skipped during cooldown and recovered afterward, aggregate
`ProviderError::Fallback` naming every hop tried, and B's answer being the only
assistant text persisted to `state.db`.

## Spec 008 closure

Spec 008 ("Memory and context management") is complete. A `char/4` advisory
estimator, a send-side sliding window, per-session pinned turns, config-driven
compression wiring, and display-only summarization of dropped turns are all
landed. The window trims only the copy handed to the provider — `self.turns`
and `state.db` always keep the full history. ADR 0003 records the agreed
`Turn::Summary` representation so the future summary-injection feature never
introduces a fake `User` turn.

The six tickets and their landing commits:

| Ticket | Scope | Commit |
|---|---|---|
| 01 | Advisory context-length accounting + `/info` | `56f0122` |
| 02 | Send-side sliding window | `7d96fd3` |
| 03 | Heuristic summarization of dropped turns (display-only) | `a7fef42` |
| 04 | Pinned messages (`/pin`, `/unpin`, `/pinned`) | `da05b73` |
| 05 | Compression config wiring into the context window | `f21753d` |
| 06 | Parity, docs, E2E closure proof | this commit |

Closure proof (Spec 008): `spec008_long_conversation_compresses_send_but_keeps_canonical_and_protects_pin`
in `crates/hermes-core/tests/conversation_session_integration.rs` drives a
120-turn history over the configured limit and asserts the send window fits the
budget, the newest and a pinned turn are always sent, the dropped-turn summary
is never injected (the provider receives only verbatim history turns), and
`state.db`/`/messages` still expose every canonical turn while read/resume
leaves the file byte-identical.

## Spec 009 closure

Spec 009 ("Multi-turn planning and reflection") is complete. Building on the
Spec 002 agentic loop, the runner now supports a guided
goal → plan → execute → reflect → recover → done flow. Every piece is
**off by default** and driven by in-memory runner state plus ephemeral
instructions — never a new persisted role and never a fake `User` turn — so the
reactive loop stays a zero-regression Spec 002 path.

The five tickets and their landing commits:

| Ticket | Scope | Commit |
|---|---|---|
| 01 | Goal extraction & tracking (`/goal`) | `7395878` |
| 02 | Plan-then-execute (`/plan`) + ephemeral instruction channel (ADR 0004) | `5d4e896` |
| 03 | Deterministic self-reflection gate (`/reflect`) | `6a38fa1` |
| 04 | Error recovery via parameter mutation, never retry a denied tool (ADR 0005) | `9d514ef` |
| 05 | Parity, docs, E2E closure proof | this commit |

Closure proof (Spec 009): `crates/hermes-core/tests/planning_reflection_e2e.rs`
drives a scripted, deterministic provider through the full guided pipeline and
asserts the goal is extracted, a plan is generated and kept in memory, a
retryable tool failure is recovered by mutating parameters (the identical repeat
is **not** re-executed), a second tool step runs, and the run finishes `Done`
with the goal `Achieved` — all within the iteration budget, with exactly one
`User` turn (no fabricated role). Companion tests pin the negative invariants: a
`Denied` tool immediately `Blocked` and never retried, and reactive mode
(`/plan`/`/reflect`/`/goal` off) behaving exactly as the Spec 002 loop. The goal
is marked `Achieved` on a guided, tool-free completion only when reflection is
on (reactive goal tracking never auto-closes a goal).

## Spec 011 closure

Spec 011 ("Model Context Protocol client") is complete. Hermes-RS connects to
MCP servers as a client over JSON-RPC 2.0 stdio, discovers each server's tools,
and registers them in the existing `ToolRegistry` under `{server}__{tool}` names
so the Spec 002/009 agentic loop can call them like built-in tools. Everything
is off by default (empty `mcp_servers:`), keeping configs written before Spec
011 unchanged; server `env` secrets are redacted; `confirm: true` routes a
server's tools through the confirmation gate.

The five tickets and their landing commits:

| Ticket | Scope | Commit |
|---|---|---|
| 01 | Config `mcp_servers` + model + env redaction + STRIDE | `e69c4dd` |
| 02 | JSON-RPC 2.0 stdio transport + `initialize` handshake | `da588d9` |
| 03/04 | Spawn child + `tools/list` → `{server}__{tool}` wrappers + `tools/call` execution + confirm gate + graceful shutdown | `5ee3d4d` |
| 05 | REPL registration + live E2E closure + PARITY/SECURITY/ROADMAP | this commit |

Closure proof (Spec 011): `crates/hermes-core/tests/mcp_e2e.rs` spawns a
controllable child-process MCP server (`mcp_test_server`), performs the real
handshake, discovers two tools, executes `echo` through the Hermes tool registry,
maps a failing tool to an error, proves the `confirm: true` + deny path returns
`Denied`, and drives the tool through `ConversationRunner::chat_agentic` so the
MCP result lands as a normal `Turn::Tool`. A companion test asserts the zero-MCP
default path adds no tools.

## Spec 011b — MCP hardening & REPL commands

A follow-up hardening pass closed four gaps identified in review (the original
Spec 011 approval over-claimed `/mcp` commands, a default-confirm policy, a
message-size limit, and `${VAR}` env expansion):

| # | Scope |
|---|---|
| 01 | **Default confirmation ON (secure-by-default).** `McpServerConfig.confirm` now defaults to `true` unless a config sets `confirm: false` explicitly, so a configured MCP server cannot run its tools silently. |
| 02 | **Message-size limit.** Inbound JSON-RPC lines over 10 MB are rejected (`McpError::Protocol`), guarding against an oversized server response (DoS). |
| 03 | **`${VAR}` env expansion.** MCP `env` values expand `${NAME}` / `$NAME` from the process environment at spawn; an unset variable errors naming the variable (never its value), and expanded values are never logged. |
| 04 | **`/mcp` REPL commands.** `/mcp` / `/mcp list` shows each server's status and tool count; `/mcp restart <name>` shuts the child down, unregisters its tools, and respawns + re-discovers. `ToolRegistry::unregister`/`names` back the restart. |

Landing commit: the Spec 011b pass (Ticket 05 → parity). Spec 011/011b tooling is
now fully covered (281 tests).

## Spec 012 — TUI dashboard (ratatui) closure

Spec 012 adds an **opt-in Ratatui terminal dashboard** (`--tui`) as an
alternative front end to the readline REPL. It reuses the same single agentic
engine and data stores; it never forks a second agentic loop. The default path
(no `--tui`) is byte-for-byte unchanged.

Architecture:

- **Core observer (UI-free).** `ConversationRunner` gained an optional
  `AgentEvent` observer (`events.rs`) — an additive, default-`None` sink that
  emits streaming chunks, tool start/done, iteration, status and token ticks at
  6+ points inside `chat_agentic`. REPL and existing callers never set it, so
  behavior is unchanged (zero regression); the TUI subscribes. `AgentEvent` is a
  domain contract any future front end (web, MCP dashboard) can consume.
- **Worker → renderer via a bounded, drop-oldest queue** (capacity 256). The
  producer never blocks; on overflow a stale frame is dropped rather than
  deadlocking the turn. Renderer→worker user input is unbounded (never lossy).
- **Sanitization at the CLI boundary.** Raw `AgentEvent`s are scrubbed
  (ANSI/control stripped + credentials redacted) in `worker::agent_event_to_tui`
  *before* becoming display `TuiEvent`s; the renderer never sanitizes. A
  headless E2E injects a credential + ANSI into a core event and asserts neither
  reaches the `TestBackend` buffer.
- **Renderer shell** runs on a blocking thread; the worker (which holds a
  non-`Send` `SessionStore`) runs inline and is `select!`-ed against it.
  `RawGuard` (a `Drop` guard) restores the terminal on every exit path
  (q, Ctrl-C, error, unwind).
- **Panels.** Streaming transcript (live chunks finalized by `Done`), a tool
  log (`▶`/`✓` with redacted args), a two-line status header (session, provider,
  token meter, iteration, goal/plan/reflection), scrollable transcript
  (PgUp/PgDn), and a single-line input editor with cursor movement + history
  (↑/↓). Tool-log replay from `state.db` populates the panel on session resume.

Landing commits (Tickets 01–04): `1b74109`, `679e972`, `041d7a8`, `bfbe108`;
closure (docs + headless E2E) is this commit. Headless E2E lives in
`crates/hermes-cli/src/tui/e2e.rs` (`TestBackend` full-session simulation,
sanitization, Ctrl-C → exit-130 mapping). `312` tests green, clippy clean.

**Phase 5 status note (accurate):** Phase 5 spans Spec 010, 011 and 012. Spec
011 (MCP) and Spec 012 (TUI) are **Done**. Spec 010 (plugin/extension system,
WASM) and Phase-4 Spec 007 (tool-execution sandbox) remain **Not started** and
are the only non-`Done` specs in the roadmap. A blanket "Phase 5 100% DONE"
claim is therefore only correct if Spec 010 is descoped from the milestone.

## Verification

Last full run: `cargo test --workspace` — 312 passed, 0 failed (multiple
consecutive full runs stable); `clippy --workspace --all-targets -D warnings`
clean.

## Invariants

- `state.db` is the only canonical storage.
- SIGINT exits with code 130.
- No silent execution: write and shell tools require explicit confirmation.
- `--unsafe` is an explicit opt-in.
- Sanitization happens only at the render boundary.
- New execution or network surface requires a STRIDE threat model.
- Credentials are redacted on every log, error, and output path.
- A cancelled turn is never persisted partially.

## Notes

- Ticket files live under `.scratch/<feature>/issues/` (e.g.
  `.scratch/hermes-rs-planning-reflection/issues/` for Spec 009), not
  `.scratch/issues/`.
- Fixture databases are per crate: `crates/hermes-cli/tests/fixtures/` and
  `crates/hermes-core/tests/fixtures/`.
- `target/debug` currently occupies roughly 11 GB on a 19 GB disk. The
  `rusqlite` `bundled-full` feature compiles SQLite from C on every clean
  build; consider a shared `CARGO_TARGET_DIR` or periodic pruning.
