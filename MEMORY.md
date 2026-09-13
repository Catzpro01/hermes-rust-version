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
  T12 is BLOCKED: workflow dispatch returned HTTP 403 Resource not accessible
  by integration. Ask user to reconnect/check GitHub in Arena. RED candidate
  is prepared but NOT executed/applied; no Rust fix is claimed. After access
  returns, remote fmt/check + RED/GREEN verification must precede Rust commit.
  Use the report, T12 and tooling README for exact evidence/next commands.
- Latest `/ask-matt` routing: continue in this session; use `/tdd` for the
  narrow ANSI geometry regression once Actions access is restored, then
  `/code-review` and the remaining T12 captures. Q1-Q8 need no re-interview.
  `/diagnosing-bugs` is the fallback if the feedback loop does not confirm
  the suspected cause. Routing is not execution, access restoration, or sign-off.
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
