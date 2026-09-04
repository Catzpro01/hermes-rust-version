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
| 4 — Advanced agent | 008 | Memory and context management | Not started |
| 4 — Advanced agent | 009 | Multi-turn planning and reflection | Not started |
| 5 — Ecosystem | 010 | Plugin/extension system (WASM) | Not started |
| 5 — Ecosystem | 011 | MCP server | Not started |
| 5 — Ecosystem | 012 | TUI dashboard (ratatui) | Not started |

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

## Verification

Last full run: `cargo test --workspace` — 166 passed, 0 failed (multiple
consecutive full runs stable; the concurrency test also ran 12× sequentially);
`clippy --workspace --all-targets -D warnings` clean.

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

- Ticket files live under
  `.scratch/hermes-rs-cli-provider-session/issues/`, not `.scratch/issues/`.
- Fixture databases are per crate: `crates/hermes-cli/tests/fixtures/` and
  `crates/hermes-core/tests/fixtures/`.
- `target/debug` currently occupies roughly 11 GB on a 19 GB disk. The
  `rusqlite` `bundled-full` feature compiles SQLite from C on every clean
  build; consider a shared `CARGO_TARGET_DIR` or periodic pruning.
