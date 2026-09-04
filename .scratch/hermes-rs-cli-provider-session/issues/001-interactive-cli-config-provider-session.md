---
title: Hermes-RS interactive CLI with Hermes compatibility
status: ready-for-agent
label: ready-for-agent
priority: high
feature: hermes-rs-cli-provider-session
---

# Hermes-RS interactive CLI with Hermes compatibility

## Problem Statement

The project needs a Rust implementation that can begin replacing Hermes Agent without discarding the existing Hermes configuration and session conventions. The current Rust workspace is only an architecture bootstrap: it has provider-neutral domain types and a CLI placeholder, but it cannot load Hermes configuration, converse with an LLM, stream an answer, or persist a session.

## Solution

Implement the first usable Hermes-RS vertical slice as an interactive CLI conversation runner. It reads the existing Hermes home configuration through a compatibility boundary, resolves the selected provider, accepts a prompt, emits response events as they arrive, and persists the conversation session. The CLI remains a thin terminal adapter over one high-level agent-runtime seam. The implementation must not mutate or delete the existing Python Hermes installation or silently overwrite existing Hermes data.

## User Stories

1. As a Hermes user, I want to start Hermes-RS from a terminal, so that I can interact with the Rust rewrite using a familiar entry point.
2. As a Hermes user, I want Hermes-RS to discover the existing Hermes home directory, so that I do not need to duplicate my configuration.
3. As a Hermes user, I want an explicit Hermes home override, so that I can test against a separate configuration and avoid changing production data.
4. As a Hermes user, I want the CLI to load supported provider settings from the existing configuration, so that the provider choice behaves consistently with Hermes.
5. As a Hermes user, I want an explicit provider/model selection, so that I can choose a backend without changing source code.
6. As a Hermes user, I want provider configuration errors to explain the missing or invalid setting, so that I can fix setup without guessing.
7. As a Hermes user, I want to enter a prompt interactively, so that I can have a normal conversation.
8. As a Hermes user, I want the response to stream as it is produced, so that I receive useful output without waiting for the entire completion.
9. As a Hermes user, I want the final response to be readable in a terminal, so that streamed output does not corrupt the prompt or session transcript.
10. As a Hermes user, I want provider errors to be presented without leaking API keys, so that failures are useful and safe to share.
11. As a Hermes user, I want a failed request to leave the prior session intact, so that transient provider failures do not destroy conversation history.
12. As a Hermes user, I want each user and assistant turn persisted, so that I can resume the conversation later.
13. As a Hermes user, I want session writes to be atomic, so that an interrupted process does not leave invalid session data.
14. As a Hermes user, I want a new session to be distinguishable from an existing session, so that I can start clean work without overwriting prior history.
15. As a Hermes user, I want a resume operation to restore prior messages, so that context survives process restarts.
16. As a Hermes user, I want the session format to be documented, so that future migrations can preserve my data.
17. As a Hermes user, I want Ctrl-C to stop the current turn cleanly, so that I can interrupt a slow provider without corrupting the session.
18. As a Hermes user, I want end-of-input to exit cleanly, so that the CLI behaves correctly in scripts and SSH sessions.
19. As a Hermes user, I want the CLI to return a nonzero exit status on configuration or provider failure, so that automation can detect failure.
20. As a Hermes user, I want the CLI to avoid writing secrets to logs, stdout, stderr, or session files, so that credentials remain protected.
21. As a developer, I want provider behavior hidden behind a provider-neutral interface, so that additional providers can be added without changing the agent runtime.
22. As a developer, I want the runtime to emit typed conversation events, so that the CLI, future gateways, and tests can consume the same behavior.
23. As a developer, I want a fake provider for tests, so that runtime behavior can be tested without network calls or API credentials.
24. As a developer, I want compatibility reads to be isolated, so that the Rust rewrite can evolve without changing the Python Hermes code.
25. As a developer, I want the first slice to compile and run on the Ubuntu VM, so that the project has a repeatable development target.
26. As an operator, I want the Rust binary to work over SSH without a graphical environment, so that the VM can host it headlessly.
27. As an operator, I want the CLI to report its selected home, provider name, and session identifier without exposing secrets, so that operation is diagnosable.
28. As a maintainer, I want behavior that differs from upstream Hermes to be recorded, so that parity gaps are intentional and trackable.

## Implementation Decisions

- The highest test seam is a provider-neutral agent runtime/conversation runner. It accepts configuration, a provider implementation, a session store, and user input; it emits typed response events and a final session result.
- The terminal CLI is an adapter only. It is responsible for argument parsing, input/output rendering, signal handling, and process exit status, not provider-specific logic.
- Hermes compatibility starts read-only. The runtime may read the Hermes home, configuration, personality/context inputs, and session conventions through an adapter, but must not mutate the Python installation or rewrite existing files implicitly.
- Configuration precedence follows the existing Hermes behavior where it can be verified: explicit CLI/environment overrides take precedence over the Hermes home configuration, and missing values produce actionable errors.
- Provider adapters expose a common request/stream interface. The first implementation should establish the interface and one real HTTP-compatible provider path while keeping provider-specific authentication, request mapping, response decoding, and error translation inside the adapter.
- The provider interface must support incremental text events, completion, provider error, and cancellation. Empty or malformed provider chunks must not terminate a valid stream unexpectedly.
- Messages use the existing provider-neutral role/content model and preserve ordering. Tool calls are not part of this first slice but the event model must not prevent their later addition.
- Session persistence is explicit and versioned. New sessions must not overwrite existing sessions unless the user explicitly requests resume or replacement behavior.
- Session writes use a temporary file plus atomic rename, and serialization failures leave the prior valid session untouched.
- Secrets are sourced from environment/config mechanisms but are excluded from serialized messages, diagnostic output, and error strings. Redaction is a required compatibility and security behavior.
- The compatibility layer must make the Hermes home location testable, including a temporary home used by integration tests.
- The implementation must preserve the existing Python Hermes installation as a primary reference and must not change its files as part of normal Rust execution.
- The Rust workspace remains a Cargo workspace with a reusable core crate and a thin CLI crate. Future gateway and dashboard adapters should reuse the runtime rather than duplicate conversation logic.
- The session and configuration schema should be designed for forward-compatible versioning, but full migration of every Hermes artifact is deferred.

## Testing Decisions

- Tests verify externally observable behavior at the agent-runtime seam rather than private helper functions or concrete implementation details.
- Use a deterministic fake provider that emits known chunks, completion, provider failure, and cancellation events.
- Test that a prompt produces ordered user/assistant events and a persisted session with the expected content.
- Test that streamed chunks are delivered incrementally and that the final session contains the reconstructed assistant response.
- Test configuration loading with an isolated temporary Hermes home, explicit overrides, missing provider configuration, malformed configuration, and unknown provider selection.
- Test session creation, resume, ordering, version metadata, atomic replacement, and preservation of the previous file after a failed write.
- Test secret redaction in provider errors, diagnostics, and persisted session metadata.
- Test cancellation and end-of-input behavior through the runtime contract; CLI-level tests should cover exit status and rendered output only where practical.
- Add a smoke test that builds and runs the CLI with a fake provider or offline mode, so that CI does not depend on a real provider key.
- Add a manual verification procedure against a disposable Hermes home. The procedure must confirm that the existing `/home/fern/.hermes` data remains unchanged.
- Existing project prior art is the current provider-neutral `Message` and `AgentConfig` model; tests should extend the public behavior around these seams rather than expose new internal structures unnecessarily.

## Out of Scope

- Full parity with every Hermes provider in the first slice.
- Tool execution, approval UI, sandboxing, terminal backends, and browser/computer-use tools.
- Skills loading, slash commands, automatic skill creation, and skill self-improvement.
- Long-term memory indexing, FTS5 search, user modeling, and learning loops.
- MCP client/server integration.
- Subagents, delegation, trajectory generation, and parallel workstreams.
- Messaging gateways such as Telegram, Discord, Slack, WhatsApp, Signal, or email.
- Cron scheduling, background gateway services, and dashboard UI.
- Full migration or write compatibility for every Python Hermes file.
- Changes to or removal of the existing Python Hermes installation.
- Persisting or transferring API keys into the Obsidian vault or repository.
- Production packaging, release signing, distribution binaries, and cross-platform installers.

## Further Notes

The upstream Python Hermes Agent checkout at `/home/fern/.hermes/hermes-agent` is the primary behavioral reference. Parity findings should be added as ADRs when they affect architecture. The next flow is `/to-tickets`, which should split this spec into vertical, independently verifiable tickets with blocking edges.
