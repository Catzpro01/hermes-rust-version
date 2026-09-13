# Project working memory

Repository-tracked memory for coding agents. Read alongside `AGENTS.md` and
`CONTEXT.md`; this is not the Hermes runtime memory store or a copy of the
user's private Python installation.

## Durable user instructions — confirmed 2026-09-14

- Always commit progress notes with meaningful work and push to GitHub on
  the branch assigned to the active session.
- Inherit skills and memory: read the tracked context and previous progress
  before continuing, and leave a usable handoff for the next agent.
- Fix errors found in scope and verify the fixes. Report environmental
  blockers honestly; never turn a failing check into a silent success.
- **Do not merge branches/PRs or enable auto-merge unless the user explicitly
  instructs it.** Commit/push approval is separate from merge approval.
- Preserve Python Hermes and its data. Never commit secrets.

## Skill entry points

- `docs/agents/skills/progress-handoff/SKILL.md`
- `docs/agents/issue-tracker.md`
- `docs/agents/triage-labels.md`
- `docs/agents/domain.md`
- Architecture and permission decisions: `CONTEXT.md`, `docs/adr/`.

## Active handoff

- Work: review Spec 017 closure, correct CI gates/evidence, and preserve
  progress on GitHub. No new Spec 018 implementation is authorized by this work.
- Ticket: `.scratch/hermes-rs-total-parity/issues/T11-closure-review.md`.
- Evidence and chronological updates: `PROGRESS.md`.
- Spec 017 implementation and closure sign-off are different: §J.7 still
  needs the complete visual evidence set or explicit approval of an
  alternative. Do not invent reviewer approval.
- Historical PR #6: 576 tests passed, clippy success; the original workflow
  masked fmt errors. This is not proof that the current commit passes.
- This session's assigned branch is `arena/01a09c1e-hermes-rust-version`.
  A successor must use its own session's assigned branch and must not switch
  to a different branch based only on this historical pointer.
