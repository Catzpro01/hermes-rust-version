# Picker delete-hint diagnosis — fixed slice, not Spec 017 closure

**DELETE_HINT_REGRESSION_FIXED.** Real committed-source PTY replay: 6/6 PASS.
All six paired PNGs below were directly opened and inspected. Other visual
mismatches are explicitly retained, not waived or normalized.

## Symptom, minimal loop and cause

The original Rust picker advertised `d delete` with an active filter, even
when no sessions matched. Python did not. Replaying the retained original
ui-3b39bd7 streams produced four Rust failures and two normal-control passes;
the six Python controls passed. See retained `picker-baseline-red.txt` and
`picker-reference-green.txt`. The initial missing-pyte environment failure was
not counted as bug RED; dependencies were restored in isolated cache.

Ranked hypotheses were shown before the targeted probe: missing eligibility
state, filter inferred from counts, or stale redraw. The minimal regression
uses one row and query `CLI`, which matches that entire row set. A fresh
`frame_lines()` result still showed the incorrect hint. This rules out PTY/
redraw and shows why `shown == total` cannot mean “no active filter.”

Root cause: `footer()` unconditionally appended the hint; `frame_lines()` did
not pass the predicate already used by `browse()`'s d-key handler:

```rust
filter.is_empty() && !shown.is_empty()
```

The fix passes that predicate as `can_delete` into the footer formatter. The
delete-key handler, database operations, palette, positions and counters are
unchanged. Existing wrong hint expectations were updated, not removed. The new
regression covers unfiltered, filter-matches-all, no-match and empty-frame
conditions. It does not claim a separate real-key Backspace/clear-filter test.

## RED → GREEN → committed-source capture

| Stage | Exact source / run | Outcome |
|---|---|---|
| Proposed regression only | 5d3c564 / [34791212317](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34791212317) | Named real-frame test FAILED three times after fmt/check; job success means expected RED, not parity |
| First fix proposal | 96d3eea / [34791352664](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34791352664) | Named test passed; full validation FAILED. Source still had a stale no-match assertion expecting the wrong hint. Complete failure log could not be downloaded; do not claim it was inspected |
| Corrected proposal | 94b0db0 / [34791539009](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34791539009) | fmt/check, named regression three times, clippy and full workspace GREEN |
| Applied exact tested patch | c07f0c593c1842795b6dcb20a3d0911062e193ad | 3397 bytes; SHA-256 c866c9cb5f555527e151ab0e16efffde2a84cfc9af5790bff48e88493bef0e15; local Rust diff byte-identical |
| Committed capture + original-symptom gate | [34791652216](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34791652216) | Full validation and all six real PTY states GREEN |
| Ordinary CI on same source | [34791652229](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34791652229) | SUCCESS |

Proposals remain in `.scratch/hermes-rs-total-parity/diagnostics/picker/`,
including `green-first.patch`; no historical screenshot or source reference
was overwritten. There was no local Rust toolchain: official remote checks
preceded application. Raw transport was retrieved through checksummed action
annotations because signed log/artifact URLs failed EOF.

## Evidence / provenance

- `rust-bundle.json`: 66532 original bytes; SHA-256
  `12f618b6c65a08af10a0da660c18dcd7362566b468d968257cf7d3e0795f7c13`.
  Contains committed source/binary/driver hashes, commands, sanitized environment,
  input/output events and snapshot endpoints. Capture refused a dirty Rust tree.
- `paired-bundle.json` reuses **unchanged Python streams** from
  `../ui-3b39bd7/python-bundle.json`, originally captured at1c3c9dd using upstream
  `63279301bcbdc185c1b07b98a9312eb0c862f26d`. Not a new Python run. Parent digest
  is included; capture-driver hashes match. No live reference/user home accessed.
- Same isolated two-session fixture and inputs as the original bug: `topic`,
  `zzzz`, and normal control; UUIDs and timestamps retained. No deletion confirmed,
  credentials supplied, provider calls or service setup performed.
- Matched100×30/80×30, TERM xterm-256color, COLORTERM truecolor, C.UTF-8 locale.
  Pinned xterm5.5.0/Chromium138.0.7204.0/DejaVu Mono5.2.5,16px,scale1; exact fonts
  and PNG hashes in `renderer.json`. Same renderer bytes as the prior packet.
- `audit.json`: 12 raw/cast roundtrips, six PNG hashes, byte-identical reused
  Python records/cell dumps and matched renderer font/browser settings verified.
  No normalization. Original session/status/ID/color/geometry differences remain.
- Local supporting checks: 5 recorder, 3 retained-packet audit and 9 workflow
  policy tests PASS. New gate test rejects compile errors/zero selected tests as
  substitutes for the exact named regression. Replay checks reject incomplete
  matrices or mismatched stream digests.

## Direct inspection ledger

| Scenario | 100×30 | 80×30 | Delete-hint result |
|---|---|---|---|
| Normal | [inspected](picker-normal-100x30-bottom.png) | [inspected](picker-normal-80x30-bottom.png) | Present on both |
| Filter `topic` | [inspected](picker-filter-100x30-bottom.png) | [inspected](picker-filter-80x30-bottom.png) | Absent on both |
| No-match `zzzz` | [inspected](picker-no-match-100x30-bottom.png) | [inspected](picker-no-match-80x30-bottom.png) | Absent on both |

## Reproduce

Install pyte0.8.2/wcwidth0.8.3 in an isolated Python path. From the repo root:

```sh
# Historical RED: expected exit1, four failures; normal controls pass.
python3 scripts/check_picker_filter_hint.py docs/hermes-ui-spec/017/evidence/ui-3b39bd7/paired-bundle.json --side rust
# Corrected captured source: expected exit0, six passes.
python3 scripts/check_picker_filter_hint.py docs/hermes-ui-spec/017/evidence/picker-hint-c07f0c5/paired-bundle.json --side rust
# On an authorized Rust runner, the fast compiled regression:
cargo test --locked -p hermes-rs session_picker::tests::picker_footer_tracks_delete_availability -- --exact
# Fresh original-scenario capture, always to a NEW path:
cargo build --locked -p hermes-rs --bin hermes-rs
python3 scripts/capture_picker_diagnostic.py target/debug/hermes-rs /NEW/picker.json
python3 scripts/check_picker_filter_hint.py /NEW/picker.json --side rust
```

For image replay, install the pinned `scripts/visual-renderer/package-lock.json`
dependencies and run `render.cjs` in `ui` mode on the paired bundle, to a NEW
directory. Minimal sandbox rendering used the NSS/NSPR libraries from the pinned
Sparticuz `al2023.tar.br` in isolated LD_LIBRARY_PATH (no system installation).
The attempted Debian package-index/download route failed and was not used.
Verify this packet with `sha256sum -c SHA256SUMS` inside its directory.

## Limited direct Standards / Spec review and cleanup

Scope: 2c7397b…c07f0c5 diagnosis delta; original059cd65 review remains historical.
This is direct agent review, not independent subagents or personal Matt approval.

**Standards:** no new hard violation found in the scoped change. One formatter
receives a named eligibility boolean from the actual frame caller; the existing
renderer seam supports a real regression, so no architecture rewrite is needed.
No temporary runtime debug logging was introduced. Proposal files are clearly
marked diagnostics; scratch tooling/dependency downloads stay outside Git.

**Spec:** this hint bug is fixed and visually rechecked. This is not whole-picker
parity: Rust's compact footer, inverse selection/palette and no-match `0/0 sessions
(filtered from 2)` still differ from Python's bottom-row footer and `0/2 sessions`.
Wizard/complete-Python-CLI coverage and other T12/T13 discrepancies remain open.
No new adaptation, Spec017 closure, or merge is implied. User acceptance is still
required after the remaining agreed work is actually complete.
