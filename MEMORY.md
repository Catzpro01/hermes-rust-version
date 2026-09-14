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

### Latest checkpoint (supersedes older chronological state below)

- Latest counter correction a8d5e8c: no-match footer now `0/2 sessions` for
  two original rows; delete-hint behavior preserved. User `/tdd` + continued
  the recommended public frame_lines seam. RED34826295910/f232480 (3 exact
  failures) → GREEN34826518574/bdd3cb0 (fmt/check/clippy/full suite). Exact
  2008-byte patch SHA40ce7a929dbba8376ec5abf3181d32e2ec18d6ea959f4b6bcf27b6f45c28c66f
  applied unchanged before committed-source capture34826757031 + CI34826756998
  GREEN. New `evidence/picker-counter-a8d5e8c/REPORT.md`: six images inspected,
  counters6/6 and hints6/6 PASS. Four normal/filter PNGs byte-identical to c07
  packet; only Rust cell row4 changes in the two no-match frames.
- Current remaining scope: footer placement/palette, other wizard/completion
  discrepancies and T12 field/full-CLI coverage. Hint AND no-match counter are
  now fixed; older “counter NOT fixed” entries below describe earlier state.
- `scripts/check_picker_counter.py` validates independent Python footer literals
  plus prior hint checks. Existing runner accepts only two named regressions;
  counter proposals are isolated under diagnostics/picker/counter. Selection
  lives in request.json test field; preserve it for counter capture. Ten workflow
  policy tests. Capture metadata includes rustc/Cargo/capture-script hashes.


- Latest runtime correction c07f0c5: picker delete hint now follows actual
  filter-empty/nonempty-selection eligibility. RED34791212317 (3 exact failures);
  first GREEN34791352664 failed full validation due a stale expected hint found
  in source (full log unavailable); corrected GREEN34791539009, exact3397-byte
  patch c866c9cb5f555527e151ab0e16efffde2a84cfc9af5790bff48e88493bef0e15 applied.
  Source capture34791652216 + CI34791652229 GREEN; six images directly inspected
  in `evidence/picker-hint-c07f0c5/REPORT.md`, original-symptom replay6/6 PASS.
  Other T12/T13 findings remain. No-match0/0 vs0/2 counter NOT fixed.
- Diagnosis tools: `scripts/check_picker_filter_hint.py` (pyte0.8.2/wcwidth0.8.3)
  replays either packet; `scripts/capture_picker_diagnostic.py` captures six states.
  Scoped workflow `.github/workflows/picker-diagnostic.yml` uses request phases
  red/green/capture and bounds proposals to session_picker.rs. 9 workflow tests.
- Cache paths may disappear between environments; install isolated deps again
  if needed. Chromium NSS/NSPR come from pinned Sparticuz al2023.tar.br, not apt.
  Git metadata may start at base while worktree preserves previous delivery: only
  align to assigned remote branch after verifying every tracked blob; never lose
  user edits or force-push. Signed artifact/log URLs still fail EOF; checksum
  annotation transport works. Correct package is hermes-rs (see evidence/ERRATA.md).


- Final four-area packet: `docs/hermes-ui-spec/017/evidence/ui-3b39bd7/REPORT.md`.
  48/48 actual paired PNGs directly inspected, including all narrow images.
  96 raw/cast checks, 48 PNG hashes, all5128 Python source hashes verified.
- Runtime checkpoint3b39bd7: capture34788156824 and CI34788156828 GREEN.
  Python1c3c9dd capture reused explicitly; same driver/pinned reference.
- Corrected exit-drain recorder via real60003-byte RED/GREEN; service launches
  now denied alongside networking. Missing gateway deps supplied only in cache.
  Preserve old ui-1e3abe7 rejected and ui-1c3c9dd intermediate packets unchanged.
- Summary example caller newline validated34787994035/e40bf5d and applied
  exactly (755 bytes, SHA639bdad8cd9c6ec12d59a34c60b89f6c45538f73f47f8bef5ed869cac3990728).
  Production UI unchanged. Final0/3-tool summaries match geometry/styles with
  branding; skills/MCP stay0. Empty picker matches, other picker/wizard/menu
  presentations differ. Do not fix production from the rejected summaries.
- `scripts/audit_ui_evidence.py` read-only audit + three behavior tests; actual
  tampered-PNG RED then digest-verification GREEN. CI runs them.
- T12/T13 remain IN PROGRESS. Wizard inventory is section-level, not every
  nested field; completion Python side is native component-host, not full CLI.
  Do not waive these boundaries or known V1/V2/V3 differences. No closure,
  independent/personal Matt approval, Spec018 or merge.


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

User explicitly invoked layout → separate styles → review → recapture.
All four slices have observed RED/GREEN and verified tested deltas applied:
layout 34783950104/34784076114; bold 34784219747/34784316150;
dim 34784445723/34784546009; punctuation 34784685139/34784783352.
See docs/hermes-ui-spec/017/evidence/tdd-followup.md for exact provenance.
Actual Rust changes only welcome.rs; old images remain unmodified FAIL evidence.
Actual source 3e7c89590b22881898b0f0b37e882a5d415d072b passed CI
34784967731. Run 34784967746 verified that final capture/build/exports are
skipped (not a capture success). Candidate removed and request set to phase
none, capture false to respect review-before-recapture.
Next: obtain review fixed point and user choice between explicitly limited
direct review or human-review packet; independent subagents are unavailable.
Do not invent independent review or silently skip to final capture. No merge.

## Review boundary resolved — 2026-09-14

User chose baseline 059cd65 (all ANSI fixes) and delegated best review mode.
Selected/performed direct limited Standards/Spec review, NOT independent
subagents. Report: evidence/review-059cd65.md. One duplication heuristic was
fixed via exact tested patch: review GREEN 34785303446/c2aca92, 2564 bytes,
5b8d335cdb45cf9ab24397afce10aa5f06e1a624c6287d8f2221e42ee811ba49.
Only test decoder reuse; runtime/expected values unchanged. Candidate consumed,
none/true now requests final committed-source recapture. Verify its CI, source
hash and empty Rust diff, then compare all four fresh pairs. Other UI evidence
and explicit user closure acceptance still outstanding. No independent verdict.

## 2026-09-14: Banner centering resolved; actual recapture inspected

Actual source 5a8e12c97dd760bf05f411a5d59585a97f0b5ad6 passed CI 34785921216
and capture 34785921221. New immutable evidence: banner-5a8e12c/ under
`docs/hermes-ui-spec/017/evidence/`. All eight PNGs directly inspected and all
eight raw/event/cast round trips verified; source hash and empty Rust diff
verified. Bundle 81053 bytes, SHA-256
705a6aee68b0a346e8f696a06bbe5b4f2af562220b7dca3ad4e95afbe24bbf93;
transport job 103801264070. Tools columns 51/41/48/49 and Session 4/17/21/21
match. Non-title glyph positions and same-glyph attributes match; background,
inverse/underline also checked on blank cells. Status is fixture match with
documented branding, not raw/pixel identity or overall acceptance.

b56c9a3 remained FAIL_SESSION_CENTER_94. New exact public-writer RED
34785680910 / GREEN 34785789939 resolved it without changing expectations.
Limited direct review addendum recorded. No independent approval. Candidate
consumed; none/false avoids redundant banner capture on documentation push.

Next: real paired wizard/picker/completion/summary evidence. Preliminary
setup/curses imports succeeded in isolated temporary homes, scrubbed env,
network blocked; NOT UI captures. `hermes_cli.completion` import also succeeded
but is shell script generation, NOT REPL candidate UI. Locate actual REPL seam.
Picker reference: hermes_cli/sessions_cmd.py wrapper and main.py
_session_browse_picker. Wizard: setup.py run_setup_wizard; Rust src/wizard/,
with tests/wizard_e2e.rs. Inventory every implemented step before capture.
Preserve Python data; no new features, independent verdict, closure or merge.
