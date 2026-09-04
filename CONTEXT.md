# Hermes-RS Context

## Purpose

Hermes-RS is a Rust rewrite of Hermes Agent, developed compatibility-first against the existing Python Hermes installation.

## Shared language

- **Agent**: the runtime that receives messages, chooses actions, invokes tools, and returns results.
- **Provider**: an adapter for an LLM backend. The target is multi-provider support.
- **Skill**: an instruction pack and optional resources that shape a repeatable agent workflow.
- **Memory**: durable user/project/session information, initially compatible with `~/.hermes` where practical.
- **Tool**: an explicitly permissioned capability the agent can invoke.
- **Gateway**: a channel adapter for messaging platforms.
- **Compatibility-first**: preserve existing data and behavior through adapters; do not modify or delete the Python Hermes installation during the rewrite.

## Current boundary

- `hermes-core` owns provider-neutral domain types and stable interfaces.
- `hermes-rs` owns the CLI entry point.
- The existing Python Hermes installation remains at `/home/fern/.hermes/hermes-agent`.
- Hermes-RS source is `/home/fern/hermes-rs`.

## Open decisions

- Which provider abstraction and streaming model to standardize.
- Which `~/.hermes` files are read-only compatible first.
- Tool permission and sandbox policy.
- Memory indexing and retrieval strategy.
- Gateway and dashboard boundaries.

## Product direction

Hermes-RS should reproduce the behavior of the original Hermes Agent rather than inventing a smaller alternative. The rewrite is staged, with the first vertical slice being the interactive CLI: load Hermes configuration, select a provider, send a prompt, stream the response, and persist the session.

## Parity sequence

1. CLI conversation loop and session persistence.
2. Provider adapters and streaming normalization.
3. Tool execution and permission/approval boundaries.
4. Skills loading and slash commands.
5. Memory and context files.
6. MCP, subagents, scheduler, gateways, and dashboard.
