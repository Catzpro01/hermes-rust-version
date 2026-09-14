# Hermes-RS — agent entry point

Read these files before editing:

1. [Rules](agent-support/RULES.md) — project constraints and safe checkpointing.
2. [Current handoff](agent-support/handoff/CURRENT.md) — active work and next action.
3. [Start next session](agent-support/handoff/START-NEXT-SESSION.md) — bootstrap/checks.
4. [Working memory](agent-support/memory/MEMORY.md) and the latest entries of
   [progress](agent-support/memory/PROGRESS.md).
5. [Context](agent-support/context/CONTEXT.md), relevant `docs/adr/`, and the active
   local issue under `.scratch/` before domain or architecture changes.

## Non-negotiable defaults

- Use only the branch assigned to the active session. Do not force-push or discard work.
- No branch/PR merge or auto-merge without explicit user instruction AND platform permission.
- Preserve Python Hermes and its data; never commit secrets or real credentials.
- Work in small verified slices; commit and push meaningful progress with notes.
- Rust changes require fmt/check; if unavailable locally, use verified official
  runner results and the exact tested patch, never invent a local PASS.
- Read requested skill originals in `.agents/skills/<name>/SKILL.md`. Preserve
  invocation gates. Do not claim unavailable subagents, personal reviews or tools.

Canonical agent-only material lives in `agent-support/`. Legacy entry paths
`MEMORY.md`, `PROGRESS.md`, `CONTEXT.md`, `docs/agents`, and `.agents/skills`
are compatibility symlinks, not duplicate sources. See [folder guide](agent-support/README.md).
