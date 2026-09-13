---
name: progress-handoff
description: Preserve project skills, decisions, verification evidence, and progress across agent sessions; commit and push without unauthorized merges.
---

# Progress and context inheritance

This is a repository-development skill, not an installed Hermes runtime
skill. It follows the existing `docs/agents/` and local-ticket conventions;
it never writes to the user's Python Hermes home.

## Restore context before work

1. Read `AGENTS.md` and `CONTEXT.md`.
2. Read `MEMORY.md`, the latest entry in `PROGRESS.md`, and the active ticket.
3. Load the applicable skills: [issue tracker](../../issue-tracker.md),
   [triage labels](../../triage-labels.md), and [domain](../../domain.md).
   Read the relevant ADRs before architecture or permission changes.
4. Inspect `git status`, current branch, and recent commits. Work only on
   the branch assigned to the session. Do not switch branches to continue
   a predecessor's work.
5. Treat remembered test counts as historical until tied to the exact
   tested commit/run. Do not inherit an approval that has not been recorded.

## Execute and verify

- Prefer small, tested changes. Fix in-scope errors instead of hiding them.
- Record the command, outcome, and whether verification was local or remote.
- Run `cargo fmt --all` and `cargo check` before committing Rust changes;
  also run relevant tests and clippy. If the environment blocks these,
  report the blocker rather than inventing a pass or disabling a gate.
- Preserve the Python installation and never include secrets in logs,
  handoffs, commits, or public GitHub content.

## Save and hand off

1. Update the active ticket with completed work and remaining criteria.
2. Append a dated `PROGRESS.md` entry: request, inherited context/skills,
   changes, verification, blockers, and exact next actions.
3. Update `MEMORY.md` only for durable decisions or the active-work pointer.
   Keep older evidence in the progress log/tickets, not duplicated as current.
4. Review the diff, stage only intended files, and commit code/docs together
   with progress. Push only the assigned branch, without force.
5. Verify the remote commit and inspect its CI checks via `gh`. Fix failures
   where possible and save a follow-up progress entry. If credentials or
   network access fail, retain the local commit and clearly report the error.
6. A commit cannot record its own hash or a future push outcome. Record
   already-observed results in the next progress entry and report the final
   hash, push, and check status to the user; never predeclare CI green.

**Never run a branch/PR merge or enable auto-merge unless the user explicitly
asks.** A request to save progress, push, fix CI, or open a PR does not grant
merge permission.

## Minimum successor handoff

- Current assigned branch and active ticket.
- Required skills and ADR paths already consulted.
- Decisions that must remain unchanged (especially permission boundaries).
- Exact evidence and known blockers; unfinished work must remain open.
- Next safe command/action, without requiring access to prior chat.
