# Picker footer color — actual CLI TDD verified

**FOOTER_COLOR_FIXED — not whole-picker parity or Spec017 closure.**
All ten paired PNGs were directly opened and inspected. Normal/filter/no-match
footers use palette index8 at row30 on both widths. Counter/hint and delete/empty
controls remain correct. See the user-facing [milestones](../../MILESTONES.md).

## Test-first chain

The user explicitly requested `/tdd warna footer picker` and milestone visibility,
continuing the recommended actual CLI/PTY seam. One primary normal100×30 test,
`PickerFooterColorTests.test_footer_uses_reference_grey`, captures the real CLI,
preserves its trace, then checks visible footer foreground against the independent
Python reference. It does not inspect private collaborators or assert escape
spelling. Capture/corruption/missing-footer/setup failures are ERROR, not color RED.

Six historical Rust color checks failed while six Python controls passed. Only
then was the live primary RED run; only after that was a production fix proposed.
The source cause was the unstyled `Print(line)` branch used for the footer.
The minimal eight-line addition prints just the footer with `Color::DarkGrey`,
then restores foreground with `Color::Reset`. Text, geometry, selection attributes,
filter/delete eligibility, numbered fallback and domain behavior are untouched.

| Stage | Source / run | Result |
|---|---|---|
| Live primary RED | 5515789 / [34832948842](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34832948842) | fmt/check/build PASS; actual color test FAILED3 times: default foreground, not palette8 |
| Minimal candidate | f633f59 / [34833157938](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34833157938) | exact CLI3, fmt/check/clippy/full workspace PASS |
| Exact patch applied | ae220ff1607f122103af1eb006b6fc4e326531e6 | Officially tested916-byte patch; local full Rust diff byte-identical |
| Committed-source capture | [34833465322](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34833465322) | Full validation, primary3 and ten actual captures PASS; color6/geometry10/counter6/hint6 checks PASS |
| Ordinary CI, same source | [34833465347](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34833465347) | SUCCESS; picker gate requires both live position and color tests |

The successful RED job means expected assertion failures, not a passing product.
`red-result.txt`, `primary-red.json`, `tested.patch`, `green-result.txt`,
`capture-result.txt` and `verification-runs.json` retain the chain. No local cargo
was used; official validation preceded Rust application. Primary RED trace85502
bytes, SHA-256 `f14e3920154956dd0d68cf072166b8044ad621b751c0e8e4c6abb6884d977f97`;
source Rust worktree was unchanged. Tested patch SHA-256
`ea56c272de0cf151168aee03e85803662f9ffe12bc563de97e6e512e73b941a3`.

## Inspected matrix

| Scenario | 100×30 | 80×30 | Observed result |
|---|---|---|---|
| Normal | [inspected](picker-normal-100x30-bottom.png) | [inspected](picker-normal-80x30-bottom.png) | Palette8 footer, `1/2 sessions   d delete`, row30 |
| Filter `topic` | [inspected](picker-filter-100x30-bottom.png) | [inspected](picker-filter-80x30-bottom.png) | Palette8 footer, `1/1 sessions (filtered from 2)`, no delete hint |
| No match `zzzz` | [inspected](picker-no-match-100x30-bottom.png) | [inspected](picker-no-match-80x30-bottom.png) | Palette8 footer, `0/2 sessions`, no delete hint |
| Delete confirmation | [inspected](picker-delete-100x30-bottom.png) | [inspected](picker-delete-80x30-bottom.png) | Existing prompt remains at row30, no gray style leakage |
| Empty store | [inspected](picker-empty-100x30-bottom.png) | [inspected](picker-empty-80x30-bottom.png) | Existing message remains at row1 |

Compared with picker-position-0c0704d, only foreground fields on row30 change in
six footer cases. All other rows and attributes are identical, including the
selected row, header and no-match message. Both delete and both empty paired PNGs
are **byte-identical** to the previous packet. All ten Python records/cell dumps
remain unchanged. This proves style isolation in the captured states, not every
possible interaction sequence.

### Encoding distinction, not a color normalization

Python selects slot8 through palette16; crossterm selects the **same slot8** through
palette256. Xterm reports foreground8 on both, but foreground-mode flags differ:
Python16777216, Rust33554432. Both are indexed modes, not truecolor; both have
`dim=false`. All other footer cell properties and text agree.

The initial audit probe required exact mode-flag equality and failed. That was
too strict for a behavioral color contract: it would enforce an ANSI encoding.
The final audit preserves/reports the distinction, validates index8 in indexed
mode, rejects truecolor, and adds an independent pixel check. **No raw bytes,
cell dumps, colors, geometry or original images were rewritten.**

`verify-pixels.cjs` reads the original PNGs with the pinned browser decoder and
compares the lower halves of the two terminal panes, excluding only the renderer's
24px separator. This region includes the complete footer and blank space above.
All six regions are pixel-identical; coordinates and input PNG hashes are retained
in `footer-pixels.json`. This is not whole-screen pixel parity.

Pyte0.8.2 exposes the two encodings as `brightblack` and `7f7f7f`. The primary
regression accepts both decoder representations; pyte alone cannot prove encoding
mode or distinguish matching truecolor. The xterm audit closes that limitation.
A synthetic decoder test supports the checker only; it is not the live product RED.

## Provenance / integrity

- Fresh Rust bundle99678 bytes, SHA-256
  `b82134515d7b59fe40346a38ef136fb6c69c63861c9fbab3aad261f999022db1`.
  Source and annotation digest verified before use; binary/driver/helper hashes,
  Rustc/Cargo1.98.1, commands, input/output events, endpoints and fixture metadata
  retained. Capture refuses a dirty Rust source tree.
- Python streams are **unchanged reused originals**, not a new Python run or a
  new5128-file source audit. Parent `ui-3b39bd7/python-bundle.json` SHA-256
  `3cb5a47246d5b0ccb47c20cff20b48b3b589b5e7be5529666455fd9c27cfd48a`, original
  checkout1c3c9dd, upstream63279301bcbdc185c1b07b98a9312eb0c862f26d. No real
  Python installation or home was modified.
- Recorder unchanged SHA-256
  `96695c3f647a59d2a7e41a43f80d98257273873feaefb837a854e45196e5598e`. Existing
  isolated two-session fixture, fixed UUIDs/timestamps1700000000/1. No credentials
  supplied or external model/service calls. Delete is displayed, not accepted;
  empty-store case is independently isolated.
- Matched100×30/80×30, xterm-256color/truecolor, C.UTF-8. Same renderer, xterm5.5.0,
  Chromium138.0.7204.0, DejaVu Mono5.2.5,16px, scale1; font/browser/settings checked.
- `verify.py`/`audit.json`:20 raw/cast roundtrips,10 PNG hashes,6 indexed-gray
  footers, only footer foreground changes, unchanged Python/source provenance and
  renderer settings. `SHA256SUMS` covers packet files except itself. Direct image
  inspection is reported separately, not claimed by an automated script.

## Reproduce

Use pyte0.8.2/wcwidth0.8.3 on PYTHONPATH. From the repository root:

```sh
# Historical color RED: expected exit1 with six failures.
python3 scripts/check_picker_footer_color.py docs/hermes-ui-spec/017/evidence/picker-position-0c0704d/paired-bundle.json --side rust
# Retained corrected capture and prior behavior: expected exit0.
python3 scripts/check_picker_footer_color.py docs/hermes-ui-spec/017/evidence/picker-color-ae220ff/paired-bundle.json --side rust
python3 scripts/check_picker_footer_position.py docs/hermes-ui-spec/017/evidence/picker-color-ae220ff/paired-bundle.json --side rust
python3 scripts/check_picker_counter.py docs/hermes-ui-spec/017/evidence/picker-color-ae220ff/paired-bundle.json --side rust
python3 docs/hermes-ui-spec/017/evidence/picker-color-ae220ff/verify.py
# Official Rust toolchain / developer machine:
cargo build --locked -p hermes-rs --bin hermes-rs
HERMES_PICKER_BINARY=target/debug/hermes-rs HERMES_PICKER_RECORDING=/NEW/color.json python3 scripts/test_picker_footer_color.py
python3 scripts/capture_picker_diagnostic.py target/debug/hermes-rs /NEW/picker.json
```

For images, use the pinned scripts/visual-renderer/package-lock.json dependencies:
`node scripts/visual-renderer/render.cjs PAIRED_JSON NEW_DIRECTORY ui`.
For the retained pixel proof:
`node docs/hermes-ui-spec/017/evidence/picker-color-ae220ff/verify-pixels.cjs`.
The sandbox sets NODE_PATH to the isolated pinned install and LD_LIBRARY_PATH to
its Sparticuz al2023.tar.br NSS/NSPR libraries. Never overwrite existing recordings
or render directories. Run `sha256sum -c SHA256SUMS` inside this packet directory.

## Standards — limited direct review

Scope7a97580…ae220ff. No independent reviewer subagents or personal Matt approval;
this uses the previously disclosed limited direct review alternative.

**No new hard violation found.** Real CLI seam, literal independent color target,
error propagation and small scoped runtime change conform to the project rules.
Workflow policy rejects missing tests and setup ERROR as RED; ordinary CI fails
if either real picker test fails. Read-only permissions, SHA pins and90-day artifact
retention remain. One non-blocking maintainability observation: color/position
runner and capture-test setup are similar; consolidation can be considered at a
separate review, not by expanding this already verified production slice.

## Spec — limited direct review

The requested footer color matches the captured reference in both widths, without
regressing footer geometry/text/hints or changing other captured rows. No new
visual adaptation was introduced. Milestone page records the requested progress.

Remaining: header and selected/delete colors, no-match message styling, other
body geometry, wizard/completion differences, T12 nested wizard/full-Python-CLI
coverage. Resize, long-list and clear-filter behavior are not established by this
fixed-size fixture. These remain open; no whole-picker/Spec017 PASS is asserted.

Supporting checks:13 workflow-policy,4 color-checker,4 geometry-checker,5 recorder
and3 retained-audit tests PASS (29 total), plus the official Rust/CLI runs above.
**No merge, user closure acceptance or final Spec017 closure.**
