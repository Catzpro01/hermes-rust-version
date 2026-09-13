# Spec 017: visual-evidence decision interview

Status: Q1-Q6 settled; awaiting round 4 answers (`needs-info`)
Owner: user (decisions), Arena agent (fact-finding and documentation)
Started: 2026-09-14
Origin: explicit user invocation of `/grill-with-docs`
Related: `issues/T11-closure-review.md`

## Method and constraints

Read the original `grill-with-docs`, `grilling`, and `domain-modeling` skills.
The session has no native Skill tool or independent subagent facility;
facts are inspected directly, not presented as independently delegated work.
Ask the current decision frontier, recommend an answer, and wait. Record
resolved terms/decisions as they arise; leave proposals labelled pending.
Implementation and closure require the user's final confirmation of shared
understanding. Commit/push notes; merge remains unauthorized.

## Facts already inspected

- `docs/HERMES_UI_SPEC.md` §J.7 requires side-by-side captures of banner
  title/grid, each wizard step, session picker, completion candidates, and
  summary line.
- T11 records existing normalized banner references and selected PTY/unit
  assertions, not a complete comparison set for those five areas.
- `docs/PARITY.md` documents intentional branding and picker adaptations;
  this interview does not silently revoke or expand them.
- CI `34778089793` passed both jobs on exact checkpoint `37d10f1`.
  This is observed remote verification, not a new local Rust run.
- The previously referenced Python installation and Rust toolchain are
  absent from the inspected sandbox paths. Python 3 is available. Choosing
  an evidence policy does not imply capture execution is already available.

## Design tree

1. **Evidence policy** (settled, Q1 = A)
   - A: retain §J.7, complete real Python/Rust side-by-side captures and keep
     automated tests as complementary regression evidence. Selected by user.
   - B: explicitly amend §J.7 to an alternative evidence policy; its exact
     coverage and limitations must be decided in subsequent rounds.
2. **Evidence format and coverage** (settled, Q2 = A and Q3 = A)
   - Applicable runtime reference/provenance and scenarios for all five areas.
   - Q2: review artifact format and raw capture provenance.
   - Q3: scenario coverage across all five required areas.
3. **Comparison rules** (settled, Q4 = A, Q5 = A, Q6 = A)
   - Terminal conditions, dynamic-value normalization, intentional adaptations,
     and concrete examples of acceptable versus unacceptable differences.
4. **Failure handling and acceptance** (current frontier, Q7-Q8)
   - Which mismatches block closure; permitted corrective-work scope.
   - Review/sign-off requirements and handling unavailable evidence.
5. **Shared-understanding confirmation**, blocked by all remaining decisions
   - Summarize the agreed policy; obtain explicit confirmation before action.

## Round 1

**Q1:** Keep the existing real side-by-side capture requirement plus tests,
or explicitly change the requirement to an alternative evidence policy?

**Recommendation:** A. It closes the known evidence gap without weakening
the specification merely because CI is already green.

**User answer:** `a` (2026-09-14). Decision A is settled: keep the real
side-by-side captures for all five §J.7 areas and supporting automated tests.
Updated the shared term "Hermes visual parity evidence" in `CONTEXT.md`.
No ADR is needed merely to retain an existing requirement. This answer is
not approval of the remaining matrix, implementation, closure, or a merge.

## Round 2: two independent decisions unlocked by Q1

Fact-finding: `wizard/setup.rs` implements Model, Terminal, Gateway, and Tools
sections and Quick/Full/Blank modes. Existing PTY suites already exercise
wide/narrow banners, cancellation, picker filtering/deletion/empty state,
and explicit unavailable-feature notices. Their test assertions are not
already a complete paired visual evidence set.

**Q2: Evidence format.**
- A (recommended): paired Python/Rust images of actual captured screens,
  retained raw terminal recordings, and provenance (source commits, terminal
  conditions, fixture identity, and reproduction steps).
- B: paired images of actual screens plus provenance/reproduction metadata,
  without requiring the underlying terminal recording to be retained.

Both preserve Q1's real-capture requirement; neither permits AI-generated
mockups or a raw unit-test result to masquerade as captured UI evidence.

**Q3: Scenario coverage.**
- A (recommended): each implemented wizard step and the other four §J.7
  areas, covering normal paths plus representative risky states: wide/narrow
  banner; wizard cancellation and an unavailable-feature notice; picker
  empty/filter/delete-confirmation states; completion alternatives and summary
  zero/nonzero counts. Use isolated dummy fixtures, not real account data.
  This is not the Cartesian product of all providers/platforms or new features.
- B: real captures of the normal paths for all five areas, including each
  implemented wizard step; edge-state evidence stays in automated tests only.

**User answers:** explicit `2A, 3A` (2026-09-14), after clarification.
Both decisions are settled: paired real-screen images, retained terminal
recordings and metadata; normal paths plus representative risky states in
all five §J.7 areas, using isolated dummy fixtures. This confirms artifact
format and scenario breadth, not comparison rules or implementation approval.

## Round 3: comparison rules

Facts checked: `docs/PARITY.md` records deliberate Rust branding and picker
adaptations (including eight-character IDs versus six in the Python UI).
`banner_e2e.rs` uses 80/100 columns and 30 rows; the banner's logo threshold
is 95 columns. `session_picker_e2e.rs` uses 100 columns by 30 rows.
These facts ground the proposals below; they are not user approvals.

**Q4: Intentional adaptations.**
- A (recommended): accept only the specifically listed existing adaptations
  in `docs/PARITY.md` for this comparison, with a per-case exception ledger.
  Example: Hermes-RS branding is allowed; a missing button/hint, wrong static
  label, or undocumented interaction difference is not silently accepted.
- B: require Python-identical presentation/behavior including the currently
  adapted branding and picker details; revisit their scope before any build.

**Q5: Dynamic values.**
- A (recommended): use deterministic dummy fixtures, retain original screen
  images/terminal recordings unchanged, and permit an explicitly logged
  normalization of unavoidable timestamp/session-ID/path differences in a
  separate comparison view only. Preserve positions/widths, colors, static
  labels, and meaningful counters; normalization cannot hide a UI defect.
- B: permit no normalization; arrange fixed inputs/environment to compare
  raw rendered outputs directly. Report unavoidable differences as unresolved.

**Q6: Terminal conditions.**
- A (recommended): baseline 100x30, narrow-layout cases at 80x30, and 94x30 /
  95x30 specifically around the banner logo threshold. Match TERM, color
  capability, locale, font, and renderer for each Python/Rust pair; record
  exact tool versions and dimensions. This is not all resolutions on all OSes.
- B: a user-specified terminal matrix instead; state desired dimensions and
  target conditions before fixing the capture matrix.

**User answers:** explicit `4A, 5A, 6A` (2026-09-14). All three are settled.

- Q4's existing-adaptation reference is the Spec 017 table and picker
  adaptations in `docs/PARITY.md` at checkpoint `b38d09e`, the material used
  in the question. New documentation cannot silently add accepted exceptions.
  A different rendering crate is not blanket permission for arbitrary visual
  differences. Each applicable exception must be identified in the report.
- Q5 preserves original paired captures and recordings unchanged; dynamic
  normalization is confined to an explicitly logged derived comparison view.
  Wrong geometry, colors, static labels, or meaningful counts cannot be hidden.
- Q6 fixes 100x30 baseline, 80x30 narrow layouts, and 94x30/95x30 banner
  threshold cases with matched rendering conditions and recorded versions.

Added the resolved distinction "Hermes comparison view" to the glossary.
These are evidence-policy decisions, not capture execution or closure approval.
No new architecture ADR is required for the reversible comparison setup.

## Round 4: corrective scope and final sign-off

Facts checked: T10 explicitly keeps new features such as Nous Portal OAuth,
live model catalogs, Docker egress firewall, and toolset-to-registry wiring
outside this closure. The user's standing instruction is to fix errors;
the unresolved scope question is whether that instruction should also expand
this task into those deliberately deferred features.

The already-agreed real-evidence requirement means a required case that
cannot be captured is BLOCKED, not passed or replaced by a mockup/test result.
Undocumented mismatches stay visible until resolved or explicitly reconsidered
by the user. This does not create a new automatic waiver category.

**Q7: Corrective-work scope (after final plan confirmation).**
- A (recommended): fix deviations in the five agreed UI areas and directly
  related regressions; regenerate affected evidence. Keep deliberately
  deferred features as separately tracked work, not implicit additions.
- B: also bring deferred features into this effort; choose the exact additions
  in a subsequent round before broadening the plan.

**Q8: Who gives final closure sign-off?**

Before presenting closure for approval, every required case must have its
agreed evidence, comparison outcomes must respect the agreed exception list,
and relevant CI must pass. Green CI alone is not sign-off.
- A (recommended): the user explicitly reviews/accepts the final evidence
  report before Spec 017 is marked closed.
- B: a reviewer explicitly designated by the user gives sign-off instead;
  establish their identity and review channel before relying on that approval.

**User answers:** Q7 pending; Q8 pending. Once these and any branches they
open are settled, present the complete agreement for confirmation. No
implementation begins and no closure is declared merely from a round answer.
Merge remains a separate explicit user instruction in either option.
