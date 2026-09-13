# T12 — Real Python/Rust visual evidence

- Status: BLOCKED — banner geometry FAIL; Actions dispatch permission required
- Label: `ready-for-human`
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

First slice is partial and unreviewed until images are inspected. Keep
component and full-CLI coverage distinct. Do not extrapolate banner evidence
to other areas or label missing cases passed. No new deferred features.

## Observed result and immediate next action

Four real paired banner cases and eight top/bottom screenshots are saved in
`docs/hermes-ui-spec/017/evidence/banner-814c235/`; the report is one directory
above. Python dependency warnings were removed by installing missing packages
in cache and recapturing. Inspected clean pairs: Rust loses unstyled spaces,
so Available Tools starts at column 2 rather than 51/41/48/49 (100/80/94/95).
All four geometry cases FAIL. Raw/cast/screenshots and checksums are retained.

The public ANSI writer regression is prepared at
`/home/user/.cache/hermes-visual-reference/red.patch` (ephemeral candidate,
not applied to source). It compares actual serialized output, with only SGR
removed, against existing independent Python references for four widths and
two color depths. If the cache is unavailable, recreate from this criterion;
never claim RED based on reasoning alone. Minimal correction to validate:
retain unstyled spaces in `write_buffer_ansi` instead of skipping them.

Dispatch for the RED run failed HTTP 403 `Resource not accessible by
integration`. No RED/GREEN job was created and no Rust correction was applied.
Ask the user to reconnect/check the GitHub connection in Arena for Actions
permission; never request tokens. After it is restored, dispatch the bounded
candidate, verify RED, then GREEN with fmt/check/clippy/full tests before
committing the actual Rust change. The workflow exports the exact tested
Rust delta with a digest; verify it before applying locally. Capture again
from the final committed source and record compiler/version provenance.

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
