# Spec 017: visual-evidence decision interview

Status: Q1 settled; awaiting round 2 answers (`needs-info`)
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
2. **Evidence matrix** (current frontier, Q2 and Q3)
   - Applicable runtime reference/provenance and scenarios for all five areas.
   - Q2: review artifact format and raw capture provenance.
   - Q3: scenario coverage across all five required areas.
3. **Comparison rules**, blocked by the evidence matrix
   - Terminal conditions, dynamic-value normalization, intentional adaptations,
     and concrete examples of acceptable versus unacceptable differences.
4. **Failure handling and acceptance**, blocked by comparison rules
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

**User answers:** Q2 pending; Q3 pending. The detailed capture matrix,
normalization rules, permitted corrective scope, and acceptance/sign-off
remain unresolved. Do not produce artifacts or change code before the final
shared-understanding confirmation.
