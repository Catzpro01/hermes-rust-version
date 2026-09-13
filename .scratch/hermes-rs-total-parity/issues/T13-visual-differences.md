# T13 — Resolve retained UI discrepancies and coverage boundaries

- Status: IN PROGRESS (evidence recorded; runtime corrections not applied)
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
