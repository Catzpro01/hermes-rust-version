# T12 — Real Python/Rust visual evidence

- Status: IN PROGRESS — four follow-up TDD slices GREEN; review/final recapture pending
- Label: `ready-for-agent`
- Owner: Arena agent (capture/corrections), user (final acceptance)
- Basis: [confirmed Q1-Q8 agreement](../grilling.md), user `setuju lanjutkan`
- Depends on: executable pinned reference, Rust runtime, matched renderer
- No merge; preserve installed Python/data; isolated dummy fixtures only.

## Initial slice

The official reference resolves to `63279301bcbdc185c1b07b98a9312eb0c862f26d`,
Python v0.21.0. Archive/tree verification: 11,315 exact blobs plus 12 PowerShell
files whose CRLF conversion matches upstream `.gitattributes`; every Python
source blob matches exactly. See `docs/hermes-ui-spec/017/evidence/reference.json`.
This verifies upstream, not undocumented changes on the historical VM.

`capture_banner.py` captures the real Rust offline REPL in a PTY at
100x30, 80x30, 94x30 and 95x30. The paired Python side invokes the unmodified
public `build_welcome_banner` display component with equivalent explicit
inputs. This is component-level banner evidence, NOT a full Python CLI run.
The raw byte stream, timed output chunks, screen endpoint, inputs, source/binary
hashes, sanitized environment, and capture boundary are retained.

The minimal Python dependency set is isolated; no user installation or home
is used. Rust is built on GitHub because no local toolchain is available.
Signed artifact download fails in this sandbox; the manual/scoped workflow
also exports a bounded gzip/base64 bundle through checksummed annotations.
A successful capture job does not assert visual equality or close §J.7.

## Coverage / acceptance

- [ ] Banner/title/grid: inspect paired images at all four widths; record deviations.
- [ ] Wizard: each implemented step, normal/cancel/unavailable-feature cases.
- [ ] Picker: normal/empty/filter/delete-confirmation states.
- [ ] Completion dropdown: normal and representative alternatives.
- [ ] Summary: zero/nonzero cases (initial banner includes only four-tools/zero-skills).
- [ ] Complete all pairs + raw recordings + reproduction metadata.
- [ ] Fix in-scope unapproved differences, rerun checks and affected captures.
- [ ] Relevant CI passes on the actual code checkpoint.
- [ ] User explicitly accepts final evidence report; only then close T11/T10.

## Current limitations

First slice has direct image/cell inspection, but remains partial and is not
independent review or user acceptance. Keep component and full-CLI coverage distinct. Do not extrapolate banner evidence
to other areas or label missing cases passed. No new deferred features.

## Observed result and immediate next action

Four real paired banner cases and eight top/bottom screenshots are saved in
`docs/hermes-ui-spec/017/evidence/banner-814c235/`; the report is one directory
above. Python dependency warnings were removed by installing missing packages
in cache and recapturing. Inspected clean pairs: Rust loses unstyled spaces,
so Available Tools starts at column 2 rather than 51/41/48/49 (100/80/94/95).
All four geometry cases FAIL. Raw/cast/screenshots and checksums are retained.

The candidate transport is tracked in `../runner-candidate/`. Authorized
branch-scoped push runs now work with official rustup, without requiring
manual dispatch or additional repository settings changes. The installed
Python is untouched; source/patch digests bind each candidate.

Correct-fixture RED run 34782931817 on 209d04e passed fmt/check and observed
`banner_ansi_preserves_python_reference_layout` fail on the buggy writer.
The earlier test proposal reused the wrong model/context/tool fixtures for
widths 100/80; its baseline was superseded. References and assertions remain
unchanged. Test inputs now match the existing independent Python fixtures.

GREEN run 34783039592 on 3f3d996 passed fmt/check, the named regression,
clippy and full tests. Exact tested delta verified (3469 bytes, SHA-256
5ae5e9b3430ad1a32e44cded4cd25b55cf6d6a81800edc613a6f82df4216a79e) and applied
locally, with byte-for-byte diff equality. Candidate consumed/disabled.
Source a2a3d08 subsequently passed CI and clean-source capture; the new pairs
still reveal layout/style defects, detailed in the latest section below.
Candidate capture alone is not final source evidence. Review and five-area
coverage remain open.

Prior checks: CI `34779396792` on `d7727e5` passed. Capture `34779200470`
and CI `34779200438` on `814c235` passed; these are infrastructure/code checks,
not passing visual comparisons. The user has not accepted closure.

## Skill routing — user invoked `/ask-matt` (2026-09-14)

The next narrowly scoped skill is `/tdd`: run the prepared regression at the
public ANSI serialization boundary, observe RED on the captured geometry
symptom, then validate the smallest correction GREEN. Use `/diagnosing-bugs`
if the reproducible loop contradicts the suspected cause or needs deeper
investigation. Follow with `/code-review` and the remaining evidence cases.
Do not restart the settled interview, broaden into new features, or treat
routing as personal Matt approval. Continue in the same session; there is
no portability need for a handoff or reason to discard the relevant context.
Actions access remains a prerequisite, not resolved by selecting a skill.

## Access recovery request and latest skill refresh (2026-09-14)

User asked to restore Actions, download current official skills, then route
with `/ask-matt`. Permission-settings read and a no-candidate dispatch retry
both returned HTTP 403. No new run was created; T12 remains BLOCKED.
Current official skills were freshly downloaded and verified (same upstream
commit, all 164 files and 37 links). Immediate routing now explicitly includes
the human authorization stage of `/wizard` before the already-planned `/tdd`.
Recovery stages and proof requirements: `docs/agents/github-actions-access.md`.
No credential collection, privilege change, or interactive wizard execution
was performed. Refreshing skills does not resolve the integration permission.

## Explicit wizard/TDD/review/recapture request

Retried RED dispatch on the assigned branch with the existing test candidate:
HTTP 403, so no regression execution occurred. Prepared the three-stage
one-shot wizard at `/home/user/actions-access-wizard.sh`; scope and checksum
are in `docs/agents/github-actions-access.md`. It requires user browser
reconnection, collects no credentials, and does not execute GitHub writes.
Syntax/static checks passed; interactive completion and actual permission
recovery remain unverified. TDD, correction review, and new captures remain
downstream; no source fix or independent review is claimed.

## Approved workflow policy application

User approved the settings recommendations. SHA pins are already in place;
now both workflows explicitly use contents: read and 90-day artifact uploads.
Six local QA tests pass (the new configuration-policy test was observed RED
then GREEN). This is not the pending ANSI regression. Repository settings
reads still return 403; no server-side allowlist/approval/log-retention change
or dispatch recovery is claimed. CI 34781191605 on 9349006 passed before this
policy change; evaluate the new commit separately.

## Push validation is available, but the repository allowlist is malformed

Run [34782293467](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34782293467)
on `596fad0` was created via push but concluded startup_failure before jobs.
The ordinary CI run 34782293878 also failed startup; the earlier 1fd48e6 CI
34782172539 did too, before the candidate workflow change.

The run page gives the exact reason: dtolnay/rust-toolchain and swatinem/rust-cache
are not allowed because the selected-actions pattern is literally:

```text
permissions:
contents: write
pull-requests: write
```

This belongs neither in the action allowlist nor in the current read-only
capture workflow. The user must replace the **allow/block actions textbox**
at repository Settings → Actions → General with the five action patterns
in `docs/agents/github-actions-access.md`, then save. Keep full-SHA enforcement.
No API dispatch restoration or permission escalation is needed for the push
validation path. No RED/formatter/check run occurred yet; actual Rust unchanged.

The persisted candidate patch is a transparent, unapplied test proposal, not
production Rust. `request.json` selects RED and binds source and patch hashes.
Once settings are corrected, update a retry marker there and push; require
actual named RED evidence, then propose/test GREEN before applying Rust.

## Verified source and new real capture — 2026-09-14

- Actual Rust source `a2a3d083bf7e79c91aee31fa7e7b1cec7be0ef96`: CI
  34783196808 and clean-source capture 34783196812 both SUCCESS.
- New immutable originals + paired PNGs in
  `docs/hermes-ui-spec/017/evidence/banner-a2a3d08/`; REPORT.md and
  comparison.json record direct inspection, not independent review/acceptance.
- rustc 1.98.1 (48a229cea), Cargo 1.98.1, empty Rust worktree diff checked.
  All eight raw/event/cast round trips and original evidence hashes verified.
- Space-collapse fixed; Tools columns at 100/80 now 51/41 (match Python).
  At 94/95 Rust remains 42 vs Python 48/49 with long session UUID + four tools.
- Open style defects: bold leaks into border after title; missing dim on
  labels/summary/cwd/session; comma/ellipsis foreground differs. No blanket
  adaptation or normalization permitted. All cases remain visually unclosed.
- Next small slices at agreed public `write_banner` seam: long-session
  allocation-after-wrap regression; then SGR modifier-reset and dim/separator
  regressions. Run actual RED/GREEN before source commits and retain recaptures.
  Investigate `layout_banner` post-wrap `left_w` vs allocated width; the simple
  reference fixtures do not cover this case. No extra feature or merge.

## `/ask-matt` routing — 2026-09-14, after post-fix captures

Read the installed upstream router and PHASE-BOUNDARIES.md. Recommendation:
continue in this session with `/tdd`, not another interview, prototype, or
Actions wizard. The next behavior is concrete and the public `write_banner`
seam is already agreed; do not assert a layout root cause before testing it.

1. One slice: replay the exact long-session/four-tool fixture from
   banner-a2a3d08/paired-bundle.json through the real public writer. Require
   a new RED for the 94/95-column symptom (Rust 42 vs Python 48/49), then the
   smallest GREEN correction. Keep 100/80 and existing references passing.
2. Separate slices: title-to-border bold reset, then dim/separator styling.
   Use independent Python attributes; do not reuse the geometry test as
   evidence that colors match or replace expected values with Rust output.
3. If a reliable reproducer cannot be made, or the minimal correction does
   not resolve it, use `/diagnosing-bugs`: establish its executed feedback
   loop before hypotheses; do not call reading old captures a live RED loop.
4. `/code-review` on Standards + Spec, then new committed-source paired
   captures and remaining T12 coverage. Review still needs the user's fixed
   point and unavailable independent subagents; no substitute verdict claimed.

Routing only this turn; no new RED/GREEN or runtime changes. This is not a
personal Matt Pocock review, acceptance of evidence, or permission to merge.

## Four follow-up TDD slices verified — 2026-09-14

User confirmed exact sequence. Layout, bold transition, secondary dim/cropped
session ellipsis, and tool punctuation each completed independent RED/GREEN
cycles at public write_banner. All GREEN runs passed fmt/check/clippy/full
workspace tests. Exact exported deltas were hash-verified and applied before
source commits; see `docs/hermes-ui-spec/017/evidence/tdd-followup.md`.

Candidates consumed, `phase: none, capture: false`. Actual source 3e7c895
passed CI 34784967731. Run 34784967746 confirms final capture/export skipped,
not a new capture result. Review fixed point and handling of missing independent
subagents must be clarified with user; no fabricated review. Then enable
capture and obtain fresh committed-source paired evidence. Existing FAIL
images remain unchanged; no overall visual PASS or closure/merge permission.

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
