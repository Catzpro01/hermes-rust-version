# Matt Pocock skills: project integration

## Source and installation

The full [official source repository](https://github.com/mattpocock/skills)
is vendored at `docs/agents/vendor/mattpocock-skills/`, pinned to commit
`3cca18b368ae95cdbdebbff572ccafa662551015` (retrieved 2026-09-14).
All 164 upstream files, including the MIT license, references, templates,
scripts, metadata, and documentation, are preserved without edits. The
adjacent `mattpocock-skills.lock.json` records every path, Git blob hash,
size, and mode. This is a source snapshot, not an automatic updater.

Latest refresh check (2026-09-14): downloaded the official archive again and
verified all 164 files (666,067 source bytes) against the canonical upstream
Git tree and the installed snapshot, plus all 37 local links. Upstream
`main` is still `3cca18b368ae95cdbdebbff572ccafa662551015`: the installed
collection is current as observed, not a newly invented upgrade. The lock's
`latest_verification` records archive SHA-256 and canonical tree identity.
No vendor file, invocation gate, or global installation was changed.

All 37 skill directories are linked into the project-local `.agents/skills/`:
25 stable engineering/productivity skills, eight `in-progress` skills, and
four `misc` skills. The latter two buckets are included because the user
requested the complete collection; they are not promoted by upstream's
stable plugin. In particular, `retro` is an upstream stub, not a functioning
retrospective workflow. Availability is not a claim of production readiness.

The upstream maintainer-only `scripts/link-skills.sh` was **not executed**:
it targets global home directories and can replace existing skill folders.
No package install hooks, Git hooks, credential wizards, or upstream CI
workflows were executed. Nested `.github/` and plugin files are archived
source only; they do not replace this project's CI or plugin configuration.
The Python Hermes installation and its skill/memory directories are untouched.

## Using the skills in this repository

1. For a user-requested `/name`, read `.agents/skills/<name>/SKILL.md` and
   its referenced material before following the workflow. `/ask-matt` has
   already been requested in this conversation; its official routing
   instructions are now available instead of the former missing-skill blocker.
2. Preserve upstream invocation rules. `disable-model-invocation: true`
   requires the user's explicit invocation. A router recommends these
   workflows; it does not silently start them. Model-invoked disciplines
   may be consulted when their documented trigger fits the work.
3. Read dependencies progressively from their original files. Do not paste
   all 37 skill bodies into AGENTS.md or treat installation as an instruction
   to run every workflow simultaneously.
4. Keep the established setup: local Markdown tickets under
   `.scratch/<feature>/issues/`, the five labels in `triage-labels.md`, and
   single-context `CONTEXT.md` plus `docs/adr/`. The existing setup artifacts
   were inspected; no tracker migration or label/layout reset was requested.
5. Follow the phase gates in the selected skill: clarify the decision before
   building, agree test seams before a TDD cycle, use red-green vertical
   slices, and keep Standards and Spec findings separate in code review.
   Preserve progress using `progress-handoff`; do not conflate its ongoing
   project memory with the narrower portable upstream `/handoff` workflow.

### Session capabilities and permissions

This session can read the exact skill files and follow applicable steps.
It does **not** expose a native Skill tool, independent reviewer subagents,
`claude --bg`, or context-clear/compact commands. When a workflow requires
one of these, report that unmet requirement and obtain agreement for a
supported alternative; do not claim parallel independent review or background
research happened when it did not. These links also do not add new slash
command handlers to the `hermes-rs` executable.

Use the assigned session branch. Skill suggestions to create prototype
branches, switch branches, finish merges/rebases, or install hooks do not
override that restriction or the user's explicit **no merge without an
instruction** rule. Continue committing and pushing progress to the assigned
branch; keep credentials out of chat and Git. A downloaded instruction is
not permission for an unrelated migration, deployment, destructive action,
or filesystem change outside this project.

## Correction: what `/ask-matt` means

Upstream defines it as **a router over skills and flows**, not a connection
to Matt Pocock or a reviewer who can approve a PR. The earlier assistant
interpretation in this session was mistaken. No personal Matt verdict has
been obtained, and earlier historical approval claims are not validated by
installing the router.

Applied to the current handoff: Q1-Q8 and execution are confirmed; first real
captures exposed an ANSI whitespace/geometry defect. T12 remains blocked by
HTTP 403 on workflow dispatch, not by missing skills. The immediate route is
the human-only authorization stage covered by `/wizard`; see
[Actions access recovery](github-actions-access.md). No credential-collecting
wizard was generated or executed. After access is actually verified, use
`/tdd` for the narrow regression, then `/code-review` and remaining captures.
Use `/diagnosing-bugs` if the loop challenges the suspected cause. Continue
in this session; do not re-interview the settled evidence policy. The router
itself grants no closure, personal Matt verdict, or merge approval.

## Updating and verification

Update only deliberately: select a new upstream commit, inspect the diff and
license, verify all downloaded blobs against its tree, and refresh the lock
and links together. Keep project adaptations in this document, not inside
the upstream snapshot. Include the source commit and observed verification
results in `PROGRESS.md` before committing and pushing.

For this installation, verification checks every locked blob (including the
upstream `AGENTS.md` symlink), all 37 project-local links, YAML frontmatter,
and invocation-policy agreement with each `agents/openai.yaml`. Rust code
is unchanged; any Rust CI results belong to their recorded commit/run.

## Complete installed catalog

`user` requires explicit user invocation. `model/user` permits either when
the task matches. `in-progress` and `misc` retain their upstream status.

| Skill | Upstream bucket | Invocation |
|---|---|---|
| [`ask-matt`](../../.agents/skills/ask-matt/SKILL.md) | engineering | user |
| [`code-review`](../../.agents/skills/code-review/SKILL.md) | engineering | model/user |
| [`codebase-design`](../../.agents/skills/codebase-design/SKILL.md) | engineering | model/user |
| [`diagnosing-bugs`](../../.agents/skills/diagnosing-bugs/SKILL.md) | engineering | model/user |
| [`domain-modeling`](../../.agents/skills/domain-modeling/SKILL.md) | engineering | model/user |
| [`grill-with-docs`](../../.agents/skills/grill-with-docs/SKILL.md) | engineering | user |
| [`implement`](../../.agents/skills/implement/SKILL.md) | engineering | user |
| [`improve-codebase-architecture`](../../.agents/skills/improve-codebase-architecture/SKILL.md) | engineering | user |
| [`prototype`](../../.agents/skills/prototype/SKILL.md) | engineering | model/user |
| [`research`](../../.agents/skills/research/SKILL.md) | engineering | model/user |
| [`resolving-merge-conflicts`](../../.agents/skills/resolving-merge-conflicts/SKILL.md) | engineering | model/user |
| [`setup-matt-pocock-skills`](../../.agents/skills/setup-matt-pocock-skills/SKILL.md) | engineering | user |
| [`tdd`](../../.agents/skills/tdd/SKILL.md) | engineering | model/user |
| [`to-spec`](../../.agents/skills/to-spec/SKILL.md) | engineering | user |
| [`to-tickets`](../../.agents/skills/to-tickets/SKILL.md) | engineering | user |
| [`triage`](../../.agents/skills/triage/SKILL.md) | engineering | user |
| [`wayfinder`](../../.agents/skills/wayfinder/SKILL.md) | engineering | user |
| [`wizard`](../../.agents/skills/wizard/SKILL.md) | engineering | model/user |
| [`claude-handoff`](../../.agents/skills/claude-handoff/SKILL.md) | in-progress | user |
| [`implement-spec`](../../.agents/skills/implement-spec/SKILL.md) | in-progress | user |
| [`loop-me`](../../.agents/skills/loop-me/SKILL.md) | in-progress | user |
| [`retro`](../../.agents/skills/retro/SKILL.md) | in-progress | user |
| [`setup-ts-deep-modules`](../../.agents/skills/setup-ts-deep-modules/SKILL.md) | in-progress | user |
| [`writing-beats`](../../.agents/skills/writing-beats/SKILL.md) | in-progress | user |
| [`writing-fragments`](../../.agents/skills/writing-fragments/SKILL.md) | in-progress | user |
| [`writing-shape`](../../.agents/skills/writing-shape/SKILL.md) | in-progress | user |
| [`git-guardrails-claude-code`](../../.agents/skills/git-guardrails-claude-code/SKILL.md) | misc | model/user |
| [`migrate-to-shoehorn`](../../.agents/skills/migrate-to-shoehorn/SKILL.md) | misc | model/user |
| [`scaffold-exercises`](../../.agents/skills/scaffold-exercises/SKILL.md) | misc | model/user |
| [`setup-pre-commit`](../../.agents/skills/setup-pre-commit/SKILL.md) | misc | model/user |
| [`grill-me`](../../.agents/skills/grill-me/SKILL.md) | productivity | user |
| [`grilling`](../../.agents/skills/grilling/SKILL.md) | productivity | model/user |
| [`handoff`](../../.agents/skills/handoff/SKILL.md) | productivity | user |
| [`teach`](../../.agents/skills/teach/SKILL.md) | productivity | user |
| [`to-questionnaire`](../../.agents/skills/to-questionnaire/SKILL.md) | productivity | user |
| [`wait-what`](../../.agents/skills/wait-what/SKILL.md) | productivity | user |
| [`writing-for-agents`](../../.agents/skills/writing-for-agents/SKILL.md) | productivity | model/user |
