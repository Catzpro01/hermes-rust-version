# Picker no-match counter — TDD slice verified

**NO_MATCH_COUNTER_FIXED — not whole-picker parity or Spec017 closure.**
All six paired PNGs below were directly opened and inspected. The two-session
no-match footer is now `0/2 sessions`, matching the pinned Python fixture.
Existing delete-hint behavior is preserved; geometry/palette remain different.

## Test-first correction

The user continued the proposed recommended public `session_picker::frame_lines()`
seam, with actual CLI/PTY acceptance afterwards. Expected output is the literal
Python fixture, not a counter recomputed by the Rust implementation. No internal
collaborators were mocked. The new test calls real filtering and frame construction
with two rows and `zzzz`, then checks the emitted footer.

Before the fix, replay of picker-hint-c07f0c5 failed both no-match counters:
`0/0 sessions (filtered from 2)` instead of `0/2 sessions`. Normal/filtered
counters and all delete hints passed; Python controls passed. Original replay
results are retained in `baseline-rust.txt` and `reference-python.txt`.

The defect was the formatter using the number of matched rows as the denominator
and appending a filtered-from suffix even when no rows matched. The minimal
production change is a three-line empty-result branch returning `0/{total}
sessions`. Nonempty formatting and delete eligibility are unchanged. Two older
no-match assertions were updated to the corrected contract; no assertion was
removed or weakened. Existing empty-frame, normal and filter/hint tests remain.

| Stage | Exact source and run | Result |
|---|---|---|
| New regression only | f232480 / [34826295910](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34826295910) | Exact named frame test FAILED three times after fmt/check; successful job means expected RED, not parity |
| Proposed correction | bdd3cb0 / [34826518574](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34826518574) | Exact test passed three times; fmt/check/clippy/full workspace GREEN |
| Exact patch applied | a8d5e8c28fed8212791f0b97d48526fb150381e8 | 2008 bytes, SHA-256 `40ce7a929dbba8376ec5abf3181d32e2ec18d6ea959f4b6bcf27b6f45c28c66f`; local Rust diff byte-identical |
| Committed-source capture | [34826757031](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34826757031) | Full validation, six real PTY counter checks and six hint checks PASS |
| Ordinary CI, same source | [34826756998](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34826756998) | SUCCESS |

Rust was validated on the official runner before local application; no local
Rust toolchain was used. New proposals are under diagnostics/picker/counter;
prior hint proposals and evidence remain unchanged. The bounded workflow permits
only the two named regressions, rejects compile/zero-test results as RED/GREEN,
and retains read-only permissions, SHA-pinned actions and90-day artifacts.

## Visual verification and remaining differences

| Scenario | 100×30 | 80×30 | Observed result |
|---|---|---|---|
| Normal | [inspected](picker-normal-100x30-bottom.png) | [inspected](picker-normal-80x30-bottom.png) | `1/2 sessions   d delete`; unchanged |
| Filter `topic` | [inspected](picker-filter-100x30-bottom.png) | [inspected](picker-filter-80x30-bottom.png) | `1/1 sessions (filtered from 2)`; no delete hint; unchanged |
| No-match `zzzz` | [inspected](picker-no-match-100x30-bottom.png) | [inspected](picker-no-match-80x30-bottom.png) | `0/2 sessions`; no delete hint; corrected |

Compared with picker-hint-c07f0c5, all four normal/filter paired PNG files are
**byte-identical**. Python cell dumps and original records match in all six cases.
Only Rust terminal cell row4 (1-based) changes in each no-match case. These are
within-side regression comparisons, not claims that the Python/Rust whole images
match: Python's dim bottom-row footer, colored header/selection and layout still
differ from Rust's compact/plain footer and inverse selection. Delete-confirmation
palette and broader wizard/completion differences are not fixed by this slice.

## Provenance / integrity

- Rust source a8d5e8c; original `rust-bundle.json` is61347 bytes, SHA-256
  `7e4d1c32191cb1fc42d812dba0469a04decde1dc814c82ea24c1f38ba69002f8`.
  Digest and source were verified against action annotations before use.
  Bundle contains binary/driver/capture-script hashes, commands, inputs/output
  events, snapshot boundaries and Rust/Cargo1.98.1 versions. Dirty Rust source
  is refused by the capture path.
- Python streams are unchanged originals from ui-3b39bd7/python-bundle.json,
  captured at1c3c9dd against upstream63279301bcbdc185c1b07b98a9312eb0c862f26d.
  Parent bundle digest is in paired-bundle.json. **Not a new Python run**, nor
  a claim that its5128 source files were re-read in this turn. Recorder hash
  matches both sides. No real Python home/reference installation was modified.
- Existing isolated two-session fixture, timestamps1700000000/1 and fixed UUIDs;
  no credentials, external services, model/provider requests or deletion confirmed.
- Matched100×30/80×30, TERM=xterm-256color, COLORTERM=truecolor, C.UTF-8 locale.
  Shared unchanged renderer: xterm5.5.0, Chromium138.0.7204.0, DejaVu Mono5.2.5,
  16px, scale1. Font/browser/settings and renderer script agree with prior packet.
- `audit.json` records12 raw/cast roundtrips, six PNG hashes, reused Python
  records/cells, four unchanged PNGs and the two changed footer rows.
  Originals are not normalized or overwritten. Replay assertions trim exterior
  whitespace only to inspect footer text, and do not certify geometry/colors.
- `SHA256SUMS` covers packet files except itself. `corrected-rust.txt` records
  the six current counter and six preserved hint checks.

## Reproduce

Use isolated pyte0.8.2/wcwidth0.8.3 on PYTHONPATH. From the repo root:

```sh
# Historical counter RED: expected exit1 at both no-match widths.
python3 scripts/check_picker_counter.py docs/hermes-ui-spec/017/evidence/picker-hint-c07f0c5/paired-bundle.json --side rust
# Corrected source capture: expected exit0, counters and hints pass.
python3 scripts/check_picker_counter.py docs/hermes-ui-spec/017/evidence/picker-counter-a8d5e8c/paired-bundle.json --side rust
# Rust runner / developer machine:
cargo test --locked -p hermes-rs session_picker::tests::picker_no_match_counter_preserves_total -- --exact
cargo build --locked -p hermes-rs --bin hermes-rs
python3 scripts/capture_picker_diagnostic.py target/debug/hermes-rs /NEW/picker.json
python3 scripts/check_picker_counter.py /NEW/picker.json --side rust
```

Replay images with the pinned scripts/visual-renderer/package-lock.json install,
`node scripts/visual-renderer/render.cjs PAIRED_JSON NEW_DIRECTORY ui`; never use
an existing output directory. Minimal sandbox rendering uses pinned Sparticuz
al2023.tar.br NSS/NSPR in isolated LD_LIBRARY_PATH. Verify this packet by running
`sha256sum -c SHA256SUMS` from this directory.

## Limited direct review

Scope:0329c7d…a8d5e8c counter delta; original059cd65 review remains historical.
Direct review only, not independent subagents or personal Matt Pocock approval.

**Standards:** no new hard violation found. A small formatter branch satisfies
the observed public behavior; tests use real interfaces and independent literal
expectations. No speculative refactor or domain/API redesign was needed. Selected
workflow regressions are explicitly allowlisted; originals are preserved.

**Spec:** this counter slice and preserved hint controls pass actual capture.
No new adaptation is introduced. T12 field/full-Python-CLI coverage and remaining
T13 layout/palette/wizard/completion discrepancies still block overall closure.
Local supporting checks:10 workflow policy,5 recorder,3 retained-packet audit tests
PASS. Full Rust validation is the remote result above. No merge or user closure
acceptance is implied.
