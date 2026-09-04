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

## Compatibility policy

Do not modify or delete the existing Python Hermes installation. Hermes-RS uses the existing `~/.hermes` layout only through explicit adapters and starts with read-only compatibility where possible.
