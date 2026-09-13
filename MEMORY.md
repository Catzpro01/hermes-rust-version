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

- `.agents/skills/ask-matt/SKILL.md` and `docs/agents/matt-pocock-skills.md`

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
- Active `/grill-with-docs` interview: `.scratch/hermes-rs-total-parity/grilling.md`.
  Q1-Q8 are settled (all A): real captures + recordings + metadata, normal
  and risky-state coverage, only existing documented adaptations, unchanged
  originals with separately logged dynamic normalization, and the agreed
  100/80/94/95-column terminal matrix. Q7 confines fixes to the five UI areas
  and related regressions; Q8 requires explicit user acceptance of the final
  report for closure. User confirmed `setuju lanjutkan`: scoped execution is
  authorized; closure and merge are not. Active execution ticket is
  `.scratch/hermes-rs-total-parity/issues/T12-visual-evidence.md`.
  Official Python source is verified in an isolated cache. Four paired banner
  cases are retained under docs/hermes-ui-spec/017/evidence/banner-814c235/.
  They expose a real ANSI whitespace/column-collapse defect; none is a parity
  pass. Python side is a public display-component call, not full CLI startup.
  T12 is IN PROGRESS, not waiting for another permission-setting loop.
  Authorized branch-scoped pushes validate an unapplied welcome.rs-only
  candidate under `.scratch/hermes-rs-total-parity/runner-candidate/`.
  Official runner rustup replaces third-party wrapper actions; remaining
  GitHub-owned actions are SHA-pinned, tokens read-only, artifacts 90 days.
  Server settings and manual dispatch permissions remain unverified/403;
  these are not required for this push route. No policy bypass or merge.
- Actual correct-fixture RED: run 34782931817 on 209d04e passed fmt/check
  then observed the named regression fail. The earlier test accidentally
  reused the wrong model/tool fixture for width 100/80; that flawed baseline
  is superseded, not hidden. References/assertions remain unchanged.
  GREEN run 34783039592 on 3f3d996 passed fmt/check, named regression,
  clippy/full tests and candidate capture. Verified and applied the exact
  3469-byte Rust delta (SHA-256 5ae5e9b3430ad1a32e44cded4cd25b55cf6d6a81800edc613a6f82df4216a79e).
  It removes only the unstyled-space skip and adds the public-writer test.
  Candidate consumed/disabled (phase none). Actual source a2a3d08 has green
  CI 34783196808 and clean-source capture 34783196812. Eight paired PNGs,
  raw/cast recordings and checked metadata are in evidence/banner-a2a3d08/.
  rustc/Cargo 1.98.1 recorded. Space-collapse fixed, NOT full visual parity:
  columns 100/80 match (51/41), but 94/95 Rust=42 vs Python=48/49.
  Bold leaks onto right title border; dim and separator colors also differ.
  Next: public-writer RED/GREEN on long-session layout and style transitions,
  then recapture. See REPORT.md/comparison.json; no original was normalized.
- User requested `/wizard` → `/tdd` → `/code-review` → recapture. Wizard
  was delivered/static checked, not run interactively; no need repeat it.
  `/code-review` still needs a user-supplied fixed point and unavailable
  independent subagents; do not invent completed review. Q1-Q8 remain settled.
  Latest `/ask-matt` routing: Continue here; `/tdd` first for long-session
  94/95-column layout, then separate style slices; `/diagnosing-bugs` if the
  loop/correction resists. The agreed public seam and Q1-Q8 need no re-interview.
  No new regression/fix was executed in that routing turn. Keep focus on Hermes.
- Fresh official toolkit download verified on 2026-09-14: upstream main still
  `3cca18b368ae95cdbdebbff572ccafa662551015`, all 164 files/37 links match.
  Latest is the already-installed version, not a new release. Verification
  added to the existing lock; no vendor edits or global installer execution.
- The missing `/ask-matt` skill blocker is resolved. On the user's download
  request, all 37 official Matt Pocock skills were installed project-locally
  from pinned upstream `3cca18b368ae95cdbdebbff572ccafa662551015`.
  Read `docs/agents/matt-pocock-skills.md` and the original skill on use.
- Correction to earlier session guidance: `/ask-matt` is a skill/flow router,
  not a way to contact Matt or obtain his approval. No personal verdict has
  been received. Its recommendation does not close Spec 017 §J.7.
- All skills are available, not all automatically invoked. Respect user-only
  triggers, experimental/stub status, and the documented missing Skill-tool /
  independent-subagent capabilities. Do not fabricate completion of those steps.
- Verified source checkpoint `3e0e8d9`: CI run `34776992554` succeeded with
  fmt/clippy success, 576 Rust tests, and five CI regression tests. Formatting
  and false error annotations are fixed. Later documentation checkpoints
  must still report their own check status rather than inheriting green.
- Remaining closure work: the real §J.7 evidence set selected in Q1 and
  explicit sign-off. See T11; code CI success does not close visual parity.
  Do not invent reviewer approval or revert to the unselected evidence policy.
- Historical PR #6: 576 tests passed, clippy success; the original workflow
  masked fmt errors. This is not proof that the current commit passes.
- This session's assigned branch is `arena/01a09c1e-hermes-rust-version`.
  A successor must use its own session's assigned branch and must not switch
  to a different branch based only on this historical pointer.

## Current TDD execution — 2026-09-14

User explicitly invoked the sequence. Long-session layout: RED 34783950104
(cc458e9), GREEN 34784076114 (33209fd), all required checks passed. Exact
4075-byte tested delta (ffe59885824b13a0c6d73f54eff9b12691b5da93e93aca91e37b0f0425b244cd)
applied to source. Next candidate is test-only RED for title-to-border bold,
using public SGR decoding (not buffer internals); request.json is authoritative.
No review/recapture completion or merge. After style slices, ask for review
fixed point and an honest way to handle unavailable independent subagents.
