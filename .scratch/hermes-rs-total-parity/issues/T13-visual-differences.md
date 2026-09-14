# T13 — Resolve retained UI discrepancies and coverage boundaries

- Status: IN PROGRESS (delete-hint slice fixed and recaptured; other gaps remain)
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
