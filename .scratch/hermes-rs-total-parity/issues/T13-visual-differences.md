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
  - [ ] Cycle 2 (column layout) in progress: the new live gate
    `picker_column_layout` pins the three-cell cursor column (` → `/`   `), the
    blank row 3, the header at width-54 with palette8 ink without bold and the
    body columns at width-57 as the retained 100/80 reference shows. RED
    requested against source `8f05d71f…`; fix not applied yet.
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
