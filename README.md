# Hermes-RS

Rust rewrite of Hermes Agent, developed as a compatibility-first workspace.

## Initial goals

- Preserve the `~/.hermes` data and skills conventions.
- Add provider-neutral LLM adapters.
- Implement tool execution with explicit permissions.
- Support memory, skills, MCP, scheduler, gateway, and dashboard incrementally.
- Keep the CLI usable on a headless Ubuntu VM.

## Workspace

- `crates/hermes-core`: domain types and stable interfaces.
- `crates/hermes-cli`: initial CLI entry point.

## Build and run

```bash
cargo check
cargo run -p hermes-rs
```

## First run

On a terminal, a bare `hermes-rs` with no `~/.hermes` yet onboards the same way
the Python agent does: it prints `No existing configuration found — running
first-time setup.`, runs the `setup` wizard, creates the home and then drops
straight into the REPL (ESC skips the wizard and starts the REPL on the offline
`fake` provider). Equivalent to `hermes-rs setup` followed by `hermes-rs`.

Piped or scripted runs never prompt and never write: they exit 1 with an error
naming both escapes (`hermes-rs setup`, `HERMES_HOME`). Inspection subcommands
(`info`, `sessions`, `model`, `tools`, `mcp`, …) report the same error instead
of creating a home during a read.

## Compatibility policy

Do not modify or delete the existing Python Hermes installation. Hermes-RS uses the existing `~/.hermes` layout only through explicit adapters and starts with read-only compatibility where possible.
