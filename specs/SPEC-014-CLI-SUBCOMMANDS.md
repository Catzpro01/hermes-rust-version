# SPEC-014: CLI Subcommands Parity (Matt Pocock Methodology)

**Target Crate**: `crates/hermes-cli`
**Status**: DONE (T01–T08 landed; review Matt pending)
**Author**: Matt (Strategic Lead)

## 1. Goal & Context
Python Hermes Agent allows non-interactive shell access via `hermes <subcommand>`.
In Rust `hermes-rs`, `src/main.rs` defines the `Commands` enum, and `src/subcommands.rs` handles execution.
All `Commands` variants are implemented (T01–T08); the T01 `coming soon` placeholder was removed in T08. Tickets and evidence: `.scratch/hermes-rs-cli-subcommands/`.

## 2. Invariants & Security Boundaries
1. Subcommands open `state.db` read-only.
2. Output must be colored on interactive TTY and plain text when piped (`io::stdout().is_terminal()`).
3. Credential safety: raw API keys must NEVER be printed to stdout/stderr (use `hermes_core::provider::redact`).
4. All tickets must pass `cargo check` and `cargo test --package hermes-cli`.

## 3. Ticket Breakdown (Matt Pocock /to-tickets)
- **T01-SESSIONS**: Implement `Commands::Sessions` (list chat sessions from `state.db`).
- **T02-INSPECT**: Implement `Commands::Inspect { id }` and `Commands::Messages { id }` (display metadata and messages).
- **T03-TOOLCALLS**: Implement `Commands::ToolCalls { id }` (filter and display tool calls in kebab-case).
- **T04-SEARCH**: Implement `Commands::Search { query }` (FTS5 search with redaction filter).
- **T05-INFO-MCP**: Implement `Commands::Info` and `Commands::Mcp` status.
- **T06-VERSION-HELP**: `hermes version` / `-V` / `--version` print the banner `VERSION_LABEL` + install facts (before config load); `--help` lists every implemented subcommand.
- **T07-CLOSURE**: Remove placeholders, sync docs (`docs/cli_subcommands.md`, `docs/ROADMAP.md`, `docs/PARITY.md`), E2E closure proof.

## 4. Status
All tickets landed. Implementation: `crates/hermes-cli/src/subcommands.rs`; closure proof: `crates/hermes-cli/tests/subcommands_e2e.rs` (26 tests). Detailed per-ticket evidence lives in `.scratch/hermes-rs-cli-subcommands/issues/01…08` (that board numbers tickets 01–08).
