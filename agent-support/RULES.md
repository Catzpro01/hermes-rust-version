# Hermes-RS Agent Instructions

Read `CONTEXT.md` before changing domain terms or architecture. Read relevant ADRs under `docs/adr/` before making an irreversible design choice.

## Agent skills

### Matt Pocock toolkit

The official toolkit is installed at `.agents/skills/<name>/SKILL.md`.
On a matching slash request, read the original skill and its references;
use model-invoked disciplines when their trigger fits. Preserve user-only
invocation gates. `/ask-matt` is a workflow router, not a personal reviewer.
Read [`agent-support/guidance/matt-pocock-skills.md`](guidance/matt-pocock-skills.md)
for the full catalog, pinned source, existing setup, and capability limits.
Project permission and branch rules below remain binding.

### Issue tracker

Issues are local Markdown files under `.scratch/<feature>/issues/`. See `agent-support/guidance/issue-tracker.md`.

### Triage labels

Use `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and `wontfix`. See `agent-support/guidance/triage-labels.md`.

### Domain docs

This is a single-context repository with root `CONTEXT.md` and ADRs in `docs/adr/`. See `agent-support/guidance/domain.md`.

### Progress, skill inheritance, and memory

Follow [`agent-support/guidance/skills/progress-handoff/SKILL.md`](guidance/skills/progress-handoff/SKILL.md).
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


## Continuity and automation

- Start with `agent-support/handoff/CURRENT.md` and the next-session guide.
- The platform's active branch constraint overrides an inherited branch name.
  Never switch, force-push, or reset unverified work to chase a remembered SHA.
- Save each small meaningful checkpoint: update memory/progress, review the diff,
  stage explicit paths, commit. The optional installed post-commit hook pushes
  ONLY to its explicitly bound current Arena branch. It never stages or commits.
- A failed auto-push leaves the commit local: report it and retry after diagnosis;
  do not claim GitHub persistence from a successful commit alone.
- Hook configuration is local and does not transfer in a clone. New sessions
  must verify the assigned branch and install explicitly using the documented tool.
- Requests to save on main do not override platform restrictions. This session
  cannot write main; integration remains separate from Spec017 acceptance.
- Preserve original evidence and vendored skill bytes. Summaries are navigation,
  not a replacement for the authoritative report, ticket or original skill.
- Never archive credentials, raw private chat, or inaccessible internal context.
  Persist relevant user-visible decisions, actions, results and constraints.
