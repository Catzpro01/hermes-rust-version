# Hermes-RS Agent Instructions

Read `CONTEXT.md` before changing domain terms or architecture. Read relevant ADRs under `docs/adr/` before making an irreversible design choice.

## Agent skills

### Issue tracker

Issues are local Markdown files under `.scratch/<feature>/issues/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Use `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

This is a single-context repository with root `CONTEXT.md` and ADRs in `docs/adr/`. See `docs/agents/domain.md`.

### Progress, skill inheritance, and memory

Follow [`docs/agents/skills/progress-handoff/SKILL.md`](docs/agents/skills/progress-handoff/SKILL.md).
At the start of work, read `MEMORY.md`, the latest `PROGRESS.md` entry,
and the active ticket as well as this file and `CONTEXT.md`. Load the
relevant skills and ADRs; pass their paths, decisions, verification results,
and blockers to the next agent instead of relying on chat-only memory.

User-confirmed workflow (2026-09-14):

- Always commit meaningful work together with its progress notes, and push
  to GitHub on the branch assigned to the session. If blocked, record and
  report the failure; never claim an unpushed commit is on GitHub.
- Fix errors found in the current scope, rerun the relevant checks, and
  distinguish a historical CI result from a new verification run.
- **Never merge branches or pull requests, or enable auto-merge, without
  an explicit user instruction. Commit/push permission is not merge permission.**
- Keep reusable decisions in `MEMORY.md`, chronological evidence in
  `PROGRESS.md`, and feature details in the local issue tracker. Do not
  store credentials or private conversation transcripts in these files.

## Engineering rules

- Preserve the Python Hermes installation; do not delete or mutate it as part of Hermes-RS work.
- Prefer small vertical slices with tests before broad rewrites.
- Keep provider and tool integrations behind explicit interfaces.
- Never commit API keys, passwords, private keys, or `.env` files.
- Run `cargo fmt --all` and `cargo check` before committing Rust changes.
