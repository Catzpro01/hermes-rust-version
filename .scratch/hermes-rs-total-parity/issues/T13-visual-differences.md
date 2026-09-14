# T13 — Resolve retained UI discrepancies and coverage boundaries

- Status: IN PROGRESS (delete hint and no-match counter fixed/recaptured; layout/palette and coverage remain)
- Label: `ready-for-agent`
- Basis: Spec017 §J.7 / Q1–Q8; no new waiver, feature implementation or merge.
- Evidence: `docs/hermes-ui-spec/017/evidence/ui-3b39bd7/REPORT.md`.

1. V1: use real wizard frames to regress palette/geometry/static labels/hints.
   Retain explicit OAuth/backend/registry deferrals; do not expand them into a
   generic inquire exemption. Check section/field inventory against Q1; capture
   any required common model fields rather than pretending they were shown.
2. V2: regress picker footer row, filter/no-match counters/hints and selected/
   delete colors. Preserve documented eight-char IDs/relative Active/status
   adaptation. Compare current pinned Python, not stale “6/no status” prose.
3. V3: distinguish candidate behavior from presentation; Python component-host
   vs real Rust inline cycle does not prove a Rust dropdown or full Python CLI
   wiring. Expand host coverage if required; do not substitute registry tests.
4. Keep all original packets; real RED → minimal scoped correction → official
   remote fmt/check/clippy/full tests before applying Rust → fresh affected
   source captures → direct Standards/Spec review. No local Rust available.
5. Only request explicit user closure acceptance after evidence/remaining
   differences actually satisfy the agreed scope. Do not merge.

## `/ask-matt` routing after delivery

Next skill: `/implement` on this ticket, using `/tdd` internally. First resolve
T12's required coverage gaps without reopening settled Q1–Q8; then correct one
captured discrepancy at a time (picker footer/filter hints/counters provide a
small concrete first slice). Official remote Rust checks precede patch application;
affected captures and Standards/Spec review precede any closure request.
This recommendation is not a personal review, acceptance, or a runtime change.

## Diagnosed / fixed slice at c07f0c5

- [x] Misleading `d delete` with active filter/no results. Missing eligibility
  predicate in footer was confirmed by minimal real-frame RED; corrected GREEN
  then exact patch application. Six committed-source PTY cases PASS, six paired
  images inspected; capture34791652216 + CI34791652229 GREEN.
- [ ] No-match counter, footer placement, selection/header/delete colors.
- [ ] Wizard/completion discrepancies and T12 field/full-CLI coverage boundaries.

Report: `docs/hermes-ui-spec/017/evidence/picker-hint-c07f0c5/REPORT.md`.
Original proposals, failed first GREEN and old images remain. This closes only
the hint symptom, not V2 as a whole, T13, Spec017 or user acceptance.

### Next routed slice after `/diagnosing-bugs`

Use `/tdd` on the remaining no-match counter: the two-session fixture should
show Python's `0/2 sessions`, not Rust's `0/0 sessions (filtered from 2)`.
First establish the exact RED; retain delete-hint and normal/filter controls.
Official remote validation → exact patch application → committed-source capture
and direct review. This routing is not a new fix or overall acceptance.

## Current checklist after counter TDD at a8d5e8c

This supersedes the historical pending-counter entries above.

- [x] Delete-hint eligibility: preserved; six actual PTY checks PASS.
- [x] No-match counter: literal Python `0/2 sessions` reached at both widths.
  Public frame RED three times → official full GREEN → exact2008-byte patch
  applied → source capture34826757031 and CI34826756998 GREEN. Six paired PNGs
  directly inspected; four normal/filter PNGs byte-identical to prior packet.
- [x] Footer placement: preserved; six PTY checks pass.
- [x] Filter help header: palette slot6 + bold reached and pixel-verified at
  both widths (RED34858664487 → GREEN34858865863 → capture34859140850/CI34859140646).
  The strict gate rejects the bright variant, so the slot is verified.
  Packet: `docs/hermes-ui-spec/017/evidence/picker-filter-header-9cc5cb4/REPORT.md`.
- [ ] Column header colour/indent, selection styling, delete-prompt colour.
  - [x] Cycle 2 (column layout) delivered: kursor column, blank row 3, header at
    width-54 in palette8 without bold and body columns at width-57 as the pinned
    100/80 reference shows (RED 34860668321 → GREEN 34861285022 → capture
    34861588181 / CI 34861587979). The first GREEN proposal was rejected by
    clippy (8/7 arguments) and kept as `rejected-first-attempt.patch`.
    Packet: `docs/hermes-ui-spec/017/evidence/picker-column-layout-b732d22/REPORT.md`.
  - [x] Cycle 3 (prompt/message rows) delivered: the delete-confirm prompt is
    palette1 + bold and the no-match message carries the **dim** attribute
    (`ESC[0;2m`, unreadable by pyte 0.8.2, so verified from the retained byte
    stream plus rendered pixels). RED 34864078749 → GREEN 34864360124 → capture
    34864672852 → CI 34864669012/34864672933. The tested 2216-byte patch is
    `message-style/green.patch`; packet
    `docs/hermes-ui-spec/017/evidence/picker-message-style-fd674dc/REPORT.md`.
    All 28 pinned pixel regions are equal, so the dim finding from cycle 2 is
    closed.
  - [x] Cycle 4 (selection row) delivered: the whole cursor row uses palette
    slot 2 + bold with no reverse video (RED 34866264371 → GREEN 34866921566 →
    capture 34868306211 → CI GREEN). Packet
    `docs/hermes-ui-spec/017/evidence/picker-selection-b4cb408/REPORT.md`; 34/34
    pinned pixel regions equal, and `attempts-result.txt` records the dim-scanner
    bug, the PTY start-up flake fix and the one unexplained GREEN failure.
  - [ ] Cycle 5 (status-tag ink) in flight: the new live gate
    `picker_status_ink` pins the tag ink from the upstream mapping read out of the
    pinned source (`docs/hermes-ui-spec/017/evidence/upstream-status-attr/`): the
    tag is drawn at `3 + name_width + 2`, five cells wide, never bold, and only on
    rows that are not the cursor — `done`/complete green slot 2, `intr` yellow
    slot 3 (corroborated by the retained capture), `err` red slot 1, `empty`
    palette 8. RED requested against the unchanged Rust source; the reference
    passes and the Rust capture misses the ink on `done`.
  - [ ] Cycle 6 candidates: resize/long-list/clear-filter behaviour.
- [ ] Wizard/completion discrepancies and T12 field/full-Python-CLI evidence.

Report: `docs/hermes-ui-spec/017/evidence/picker-counter-a8d5e8c/REPORT.md`.
No new adaptation, whole-picker PASS, Spec017 closure, or merge.

### Footer-position TDD delivered at0c0704d

Actual CLI normal100x30 RED34829070839 (row5 !=30, three failures) → official
GREEN34829279773 → exact1199-byte patch applied → committed-source capture
34829549105 and ordinary CI34829549118 SUCCESS. Primary PTY regression is now
required in ordinary CI. Ten paired images directly inspected: footer row30 at
both widths, delete prompt still bottom, empty row1. Counter6/hint6 controls PASS.
Audit validates20 raw/cast roundtrips,10 PNGs, unchanged Python records/cells and
renderer settings. Only old/new footer rows change; delete only drops stale row5;
both empty PNGs byte-identical to original. No normalization or new Python run.
See `docs/hermes-ui-spec/017/evidence/picker-position-0c0704d/REPORT.md`.
Palette/header/selection/delete styling, other body geometry and wizard/completion
coverage differences remain open. Resize/long-list/clear-filter behavior is not
proved by this fixed-size slice. No whole-picker PASS, final acceptance or merge.

### `/ask-matt` after footer-position delivery — routing only

Read installed official ask-matt skill and PHASE-BOUNDARIES, inherited memory,
T13 and retained0c0704d evidence. Recommended next: `/tdd` for footer foreground
color at the already used actual CLI/PTY seam, one primary normal100x30 test
before any production proposal. This is a concrete observed behavior, so no
new grilling/spec/triage or broad architecture work is needed.

Evidence detail: all six retained normal/filter/no-match cases have Python
footer foreground palette index8 (xterm cell fg8/fm16777216), dim=false; Rust
uses default foreground (fg-1/fm0), dim=false. Earlier report wording "dim footer"
is descriptive appearance, NOT an assertion of the ANSI dim attribute. Derive
expectations from independent reference terminal cells, not an invented SGR2
requirement or literal escape spelling. This inspection is not a fresh live RED.

Proposed route: confirm primary seam/behavior → actual CLI RED → minimal color
correction → official fmt/check/clippy/full tests before exact Rust application
→ affected committed-source captures at100/80 → Standards/Spec review and direct
paired-image inspection. Retain row30, counters/hints and delete/empty controls;
ensure footer style does not leak. Header/selection/delete styles and remaining
body geometry are separate slices; T12 wizard nested fields/full-CLI and T13
completion gaps remain blockers. No resize/feature expansion or new adaptation.

Ask-matt is a skill router, not a personal Matt review. No new test, production
patch, delegated review, closure, or merge is authorized/executed by this routing.
Same repository/session can continue; no portable handoff needed. Independent
review tools remain unavailable; do not relabel limited direct review as such.

### Footer color M5 delivered atae220ff — milestones visible

Source capture34833465322 and ordinary CI34833465347 SUCCESS. Color6/geometry10/
counter6/hint6 PASS; both real CLI position and color tests required in CI.
Ten paired PNGs directly opened/inspected. New packet
`docs/hermes-ui-spec/017/evidence/picker-color-ae220ff/REPORT.md` retains raw/casts,
receipts, RED trace, tested patch, audit/checksums and reproducible verifiers.
Rust bundle99678 bytes SHAb82134515d7b59fe40346a38ef136fb6c69c63861c9fbab3aad261f999022db1.

Only footer foreground on row30 changes; all other cells unchanged. Four delete/
empty PNGs byte-identical to0c0704d. Python records reused unchanged, not a new
capture/source audit. Initial audit mode-equality probe failed: Python palette16
vs Rust palette256, both indexed8/dim=false. Correct semantic check retains this
encoding distinction (no raw/color normalization), rejects truecolor, and verifies
six original footer/lower-half regions pixel-identical with pinned browser decoder.
20 raw/cast roundtrips,10 PNG hashes,29 supporting tests PASS. Limited direct
Standards review:0 new hard violations,1 nonblocking duplication observation;
Spec color slice passes, broader picker/wizard/completion gaps remain open.

User-facing `docs/hermes-ui-spec/017/MILESTONES.md` now marks M1–M5 complete for
color ONLY; four picker correction milestones verified. No invented whole-project
percentage, final acceptance, resize/long-list/clear-filter claims, or merge.

### Normal header H5 delivered at7fef514

Source capture34837424495 and ordinary CI34837424464 SUCCESS. Ten paired PNGs
opened/inspected. Header4, footer color6/geometry10/counter6/hint6 PASS. New packet
`docs/hermes-ui-spec/017/evidence/picker-header-7fef514/REPORT.md` includes original
raw/casts, stage receipts/patch, audit/checksums and reproduction scripts.
Rust bundle102020 bytes SHAf41d50ce422ee795a1fc7b1c5981b739a808389f97c4309e36df05970842b58f.

Only normal/delete header row1 foreground/bold changes. All other cells unchanged;
filter/no-match/empty6 paired PNGs byte-identical toae220ff. Python records/cells
reused unchanged, not a new capture or5128-file source audit. Python palette16 /
Rust palette256, both indexed3+bold/dim=false; encoding difference retained.
Four complete header-row pixel regions identical with pinned renderer.20 raw/cast
roundtrips,10 PNG hashes,34 supporting tests PASS. Source driver/helper hashes
checked against committed blobs. No raw/image normalization.

Milestones H1–H5 complete; now five picker correction milestones verified.
CI requires live position, footer color and normal-header tests. Limited direct
Standards/Spec review:0 new hard violations,1 nonblocking duplicated-runner/setup
observation; filter/column headers, selection/delete/no-match styles, other layout,
wizard/completion and T12 coverage remain open. No resize/clear-filter/long-list
claims, new adaptation, whole-picker acceptance, closure or merge.
  - [x] Cycle 5 (status-tag ink) patch authored: `status_ink()` + `status_tag_span()`
    + the per-row tag redraw in `session_picker.rs`, 5567 bytes
    (sha256 `a073bedc…`), verified with `git apply --check` against the committed
    source. RED run 34870302745 failed three times as expected (assertion only,
    no setup error). GREEN bound to the patch digest.
  - [ ] Cycle 5 GREEN attempt 1 (run 34870642732) failed honestly: the patched
    binary panicked with `attempt to subtract with overflow` at the blank
    separator row (`n - 3` on `n == 2`), so every case exited 101 at stage 0 and
    the gate reported a capture error, not an assertion failure. The panic was
    located by decoding the retained trace annotation
    (`sha256 bc0b233a…`, 18703 bytes) — the harness's raw bytes carry the Rust
    panic message. The patch now guards `n < 3` and is re-requested as
    `60687c1c…` (5840 bytes).
  - [ ] Lesson for the CI story: `ci.yml`'s picker step uses
    `continue-on-error: true`, so a real live-gate failure shows up as
    `step.conclusion == success` while the step's `outcome` is `failure`, and the
    job still fails at the final aggregate step. Cycle 9's "unexplained" GREEN
    failure `34866468131` was exactly this: the committed source failed a live
    gate, not a flake. Read the `picker terminal` annotation, not the step list.
  - [ ] Cycle 5 GREEN attempt 2 (run 34871081677): the fix works — the live gate
    captured all six cases and passed three times in a row — but the new unit
    test compared `format_row(..., 20, ...)` cells against the 100-column tag
    offsets, so step 24 (Full GREEN validation) failed on that assertion alone.
    Test corrected to use the matching widths; patch re-requested.
  - [x] Cycle 5 GREEN: run 34871381451 succeeded with `green actual CLI
    regression verified three times` (all six live cases captured, full
    validation OK). Exported `tested.patch` = 6109 bytes,
    sha256 `73d0322e…` — the same diff as the bound `e88347c8…` patch plus the
    rustfmt line wrapping the runner applies; it is now applied to the tree.
    (The earlier commit message quoted `3b7f…`; the correct bound digest is
    `e88347c8…`.)
  - [x] Cycle 5 (status-tag ink) delivered: fix `1781404`, capture `19bbbd5`,
    packet `docs/hermes-ui-spec/017/evidence/picker-status-ink-19bbbd5/`
    (79 files) with audit `STATUS_TAG_INK_FIXED_ALL_CONTROL_REGIONS_EQUAL` —
    10 checkers PASS on the committed source, 34/34 control regions equal,
    4 declared tag-span differences, 20 changed cells (row 5 only).
  - [ ] Cycle 6 candidates (in order): (a) the picker repaints the whole frame
    every poll timeout while the reference redraws only after a key
    (`_curses_browse` blocks in `stdscr.getch()`) — this is also what makes the
    PTY gates flaky; (b) resize/long-list/clear-filter behaviour; (c) the
    documented `Active`/`ID` adaptations; (d) a fixture that exercises the
    `interrupted`/`error`/`empty` inks in a live capture (today only
    `complete`/`done` is captured).
  - [ ] Cycle 6 (a) in flight: the picker redraws the whole frame on every poll
    timeout while the pinned reference draws the frame and then blocks in
    `stdscr.getch()` (one draw per key, nothing while idle). The new live gate
    `picker_redraw_on_input` splits each record at the scenario keystrokes and
    counts frames per input window (allowance 1 before the first keystroke, one
    per typed key afterwards). The retained Python reference passes 10/10 on the
    cycle-10 bundle; the committed Rust capture fails 8/10 (e.g. 5 frames with no
    input at all, 25 frames in the no-match case) while the empty-store case is a
    control. This deviation is also the cause of the PTY start-up flake, so fixing
    it makes every live gate deterministic.
  - [x] Cycle 6 (a) RED: run 34873144309 failed exactly three times with the
    assertion and no setup error (trace `65cf8b28…`, 279052 bytes). Patch bound:
    2801 bytes `925b963e…` — a `dirty` flag makes the redraw happen only when a
    key or a resize changed the screen; the 100 ms poll loop stays for signal
    responsiveness. The runner's `cargo fmt --all` reindents the new block, so
    the tested patch is expected to differ from the bound one, as in cycle 10.
  - [x] Cycle 6 (a) GREEN: run 34873480643 succeeded, and the exported tested
    patch is byte-identical to the bound request patch (`925b963e…`, 2801 bytes)
    — rustfmt had nothing to change. The live gate passed three times and the
    retained trace fell from 279052 bytes (RED) to 45749 bytes (GREEN), the
    direct sign that the picker stopped repainting while idle.
  - [x] Cycle 6 (a) delivered: fix `bbd943c`, capture `b2db435`, packet
    `docs/hermes-ui-spec/017/evidence/picker-redraw-on-input-b2db435/` (78 files)
    with audit `REDRAW_ON_INPUT_FIXED_FRAME_CONTENT_UNCHANGED_ALL_REGIONS_EQUAL` —
    eleven checkers PASS, the same gate rejects 8/10 cases of the previous packet,
    frame content byte-identical, and the PTY start-up flake stops reproducing.
  - [ ] Cycle 7 candidates: (a) resize/long-list/clear-filter behaviour;
    (b) the documented Active/ID adaptations; (c) a fixture that exercises the
    interrupted/error/empty inks in a live capture.
  - [ ] Cycle 7 (size contract) started; the pinned reference records must be
    captured before the Rust gate can be RED. The private-looking upstream
    repository is public, so the runner materialises the pinned checkout from
    codeload and records the reference side itself.
  - [x] Reference-phase attempt 1 (run 34876400782) failed honestly: the plan step
    short-circuited *before* writing `phase`/`test` to `GITHUB_OUTPUT`, so a
    missing `test` made every `!=` step condition true and the generic Rust unit
    test step ran instead. Fixed by writing the outputs first; 
    `test_ci_workflow.py` now asserts the written outputs.
  - [x] Reference-phase attempt 2 (run 34876621220) failed honestly too: the live
    Rust size gate ran during the reference phase (its guard only checked `test`)
    and failed, stopping the job before the reference steps. The live step and its
    trace step now skip `phase == 'reference'`, and `test_ci_workflow.py` asserts
    that guard. Incidentally this run shows the new live gate is RED-shaped: it
    fails on the assertion with no setup error.
  - [x] Reference-phase attempt 6 (run 34877481167) finally ran the pinned Python
    picker: the environment fix worked. Findings from its per-case lines:
    * 40 columns: the reference draws the picker frame (369 bytes, no error) —
      `Browse sessions — ↑↓ navigate Enter …`, `Title / Preview  Stat Msgs`,
      `→ second topic  intr 1`, footer `1/2 sessions  d delete`;
    * 39 columns: `Terminal too small` is what it draws (99 bytes) — and the
      harness therefore timed out waiting for `Browse sessions`, which is exactly
      the notice case.
    So the contract is per width (40 usable, 39 notice), not a cross product: the
    case/width matrix is now explicit (`PAIRS`), `capture_side` accepts
    `pairs`, and the live test uses the same pairing.
