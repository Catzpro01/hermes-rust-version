# T12 — Real Python/Rust visual evidence

- Status: IN PROGRESS — banner tracer slice; other areas not captured
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

First slice is partial and unreviewed until images are inspected. Keep
component and full-CLI coverage distinct. Do not extrapolate banner evidence
to other areas or label missing cases passed. No new deferred features.
