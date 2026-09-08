# SPEC-014: CLI Subcommands Parity (Matt Pocock Methodology)

**Target Crate**: `crates/hermes-cli`
**Status**: IN PROGRESS
**Author**: Matt (Strategic Lead)

## 1. Goal & Context
Python Hermes Agent allows non-interactive shell access via `hermes <subcommand>`.
In Rust `hermes-rs`, `src/main.rs` defines the `Commands` enum, and `src/subcommands.rs` handles execution.
Currently, `Commands::Model` is implemented, while all other commands return `coming soon: <name> (Spec 014)`.

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
