# Hermes-RS Roadmap

Status of the staged rewrite described in [`CONTEXT.md`](../CONTEXT.md).

## Phase status

| Phase | Spec | Scope | Status |
|---|---|---|---|
| 1 — Foundation | 001 | CLI, config, session, streaming | Done |
| 1 — Foundation | 002 | Tool calling, agentic loop | Done |
| 2 — Inspection & search | 003 | Session/message inspection CLI | Done |
| 2 — Inspection & search | 004 | FTS5 full-text search | Done |
| 3 — Multi-model & routing | 005 | Multi-provider runtime routing | Not started |
| 3 — Multi-model & routing | 006 | Model fallback and load balancing | Not started |
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

## Verification

Last full run: `cargo test --workspace` — 14 suites, 89 passed, 0 failed.

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
