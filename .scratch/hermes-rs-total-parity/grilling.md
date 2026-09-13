# Spec 017: visual-evidence decision interview

Status: awaiting round 1 answer (`needs-info`)
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

1. **Evidence policy** (current frontier, Q1)
   - A: retain §J.7, complete real Python/Rust side-by-side captures and keep
     automated tests as complementary regression evidence. Recommended.
   - B: explicitly amend §J.7 to an alternative evidence policy; its exact
     coverage and limitations must be decided in subsequent rounds.
2. **Evidence matrix**, blocked by Q1
   - Applicable runtime reference/provenance and scenarios for all five areas.
   - What must be captured versus asserted automatically.
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

**User answer:** pending. No policy, glossary change, ADR, or acceptance
waiver has been approved.
