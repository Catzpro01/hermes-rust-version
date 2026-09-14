# Picker footer position — actual CLI TDD slice verified

**FOOTER_POSITION_FIXED — not whole-picker parity or Spec017 closure.**
All ten paired PNGs were directly opened and inspected. Normal, filtered and
no-match footers now occupy terminal row30 at both100×30 and80×30, matching the
retained Python reference. Counter and delete-hint fixes remain intact.

## Test-first correction

The continued `/tdd` uses the previously agreed actual CLI/PTY seam: public
`frame_lines()` text cannot establish screen coordinates. One primary test,
`PickerFooterPositionTests.test_footer_is_on_terminal_last_row`, launches the
real CLI through the existing isolated recorder, preserves its raw recording,
then asserts the independent Python literal row30 via terminal screen state.
It does not assert ANSI escape spelling or mock private rendering internals.
Setup, missing-footer or corrupt-recording failures are ERROR, not geometry RED.

Historical replay first showed Python10/10 geometry PASS and six Rust failures
(normal row5, filter/no-match row4); delete row30 and empty row1 already passed.
The live primary RED then proved the defect in the current source, not merely
an old screenshot. Only after that run was the Rust correction proposed.

The cause was sequentially printing every frame element immediately after the
table. The minimal fix explicitly positions the final element at the last row
and omits its next-line move. Body rendering, selection attributes, footer text,
filter/delete eligibility and numbered fallback are unchanged. No speculative
refactoring, API redesign or palette change was included.

| Stage | Source / Actions run | Result |
|---|---|---|
| Primary CLI RED | eedd9a0 / [34829070839](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34829070839) | fmt/check/build PASS; actual named test FAILED three times, row5 !=30; successful job means expected RED |
| Minimal candidate | 984b92f / [34829279773](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34829279773) | fmt/check/build, exact CLI test3, clippy/full workspace PASS |
| Applied source | 0c0704d08d367444e780a0951506ba28ce125b2d | Officially tested1199-byte patch applied; local full Rust diff byte-identical |
| Committed-source capture | [34829549105](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34829549105) | Full validation and primary3 PASS; ten actual PTY captures; geometry10, counters6, hints6 PASS |
| Ordinary CI, same source | [34829549118](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34829549118) | SUCCESS, including required primary actual CLI coordinate regression |

No local cargo was used. `verification-runs.json` retains source/step receipts;
`primary-red-result.txt`, `primary-red.json`, `tested.patch`, `green-result.txt`
and `capture-result.txt` retain the diagnostic chain. Primary RED recording is
24779 bytes, SHA-256 `fc71b1f359b1ea7d40ee06e359b26d8a3308340db9298dfd1163b8aa2c6c43c7`;
its Rust worktree diff is empty. Tested patch SHA-256 is
`e7ad21d2a7baab771e3b0a5742d5d741720302c1ed082e70448911406208fede`.

A separate negative workflow-policy test exposed Bash `! grep` bypassing
errexit with a synthetic mixed FAIL+ERROR log. Explicit exit1 fixed this before
GREEN, and the gate now requires one executed test. The real RED log contained
no setup ERROR. Ordinary CI requires four successful outcomes (fmt, clippy,
workspace tests, live picker test); all625 gate combinations are checked.

## Inspected visual matrix

| Scenario | 100×30 | 80×30 | Result |
|---|---|---|---|
| Normal | [inspected](picker-normal-100x30-bottom.png) | [inspected](picker-normal-80x30-bottom.png) | `1/2 sessions   d delete` at row30 |
| Filter `topic` | [inspected](picker-filter-100x30-bottom.png) | [inspected](picker-filter-80x30-bottom.png) | `1/1 sessions (filtered from 2)` at row30, no delete hint |
| No match `zzzz` | [inspected](picker-no-match-100x30-bottom.png) | [inspected](picker-no-match-80x30-bottom.png) | `0/2 sessions` at row30, no delete hint |
| Delete confirmation | [inspected](picker-delete-100x30-bottom.png) | [inspected](picker-delete-80x30-bottom.png) | Prompt still at row30; no stale footer below table |
| Empty store | [inspected](picker-empty-100x30-bottom.png) | [inspected](picker-empty-80x30-bottom.png) | `No sessions found.` at row1; unchanged |

Cell-level regression comparisons use picker-counter-a8d5e8c for normal/filter/
no-match and ui-3b39bd7 for delete/empty (absent from the counter packet). At both
widths, normal changes only rows5/30; filter/no-match only rows4/30. Footer cells
including styles move intact. Delete changes only row5, removing the old footer;
the bottom-row prompt and selected/table cells remain unchanged. Both empty-store
paired PNGs are byte-identical to the original packet. Python records and cell
dumps are unchanged in all ten cases.

**Remaining differences:** Python's dim footer, yellow/cyan header, green arrow
selection and red delete prompt still differ from Rust's plain/inverse styling.
The body/no-match rows and columns are not fully aligned. Existing ID/Active/status
adaptations are preserved, not broadened. This fixed-size slice does not test or
fix resize handling, scrolling beyond the two-row fixture or clearing the filter
with Backspace. Wizard/completion differences and T12 nested-field/full-Python-CLI
coverage remain open; this evidence is not a waiver of those requirements.

## Provenance and integrity

- Fresh Rust bundle108695 bytes; SHA-256
  `5ab4a98ea7d1cc38b8977e76994b7a885290d04d5bf95407d0ba267fba9d900f`.
  Source, annotation digest, binary/driver/helper hashes and clean Rust worktree
  capture gate verified. Rustc/Cargo1.98.1, commands, inputs, raw/events and
  snapshot endpoints are retained in the bundles.
- Python streams are **reused unchanged**, not a new Python run/source audit.
  Parent `ui-3b39bd7/python-bundle.json` SHA-256
  `3cb5a47246d5b0ccb47c20cff20b48b3b589b5e7be5529666455fd9c27cfd48a`;
  original checkout1c3c9dd, upstream63279301bcbdc185c1b07b98a9312eb0c862f26d.
  Original5128-file source audit remains historical. No real Python home changed.
- Recorder unchanged SHA-256
  `96695c3f647a59d2a7e41a43f80d98257273873feaefb837a854e45196e5598e`.
  Existing isolated dummy two-session fixture, fixed UUIDs/timestamps1700000000/1;
  no credentials supplied or external model/service requests. Delete confirmation
  is displayed, not accepted; empty store is separately isolated.
- Matched100×30/80×30, xterm-256color/truecolor, C.UTF-8. Renderer unchanged:
  xterm5.5.0, Chromium138.0.7204.0, DejaVu Mono5.2.5,16px, scale1. Font hashes,
  browser and renderer settings match prior evidence. No normalization performed.
- `verify.py`/`audit.json` validate20 raw/cast roundtrips,10 PNG hashes, unchanged
  Python provenance/cells, renderer settings and bounded within-Rust cell changes.
  Direct inspection above is human-readable agent review, not claimed by the script.
  `SHA256SUMS` covers all packet files except itself; originals are not overwritten.

## Reproduce

Use pyte0.8.2/wcwidth0.8.3 on PYTHONPATH. From the repository root:

```sh
# Historical geometry RED: expected exit1, six failures.
python3 scripts/check_picker_footer_position.py docs/hermes-ui-spec/017/evidence/ui-3b39bd7/paired-bundle.json --side rust
# Corrected retained source: both commands exit0.
python3 scripts/check_picker_footer_position.py docs/hermes-ui-spec/017/evidence/picker-position-0c0704d/paired-bundle.json --side rust
python3 scripts/check_picker_counter.py docs/hermes-ui-spec/017/evidence/picker-position-0c0704d/paired-bundle.json --side rust
python3 docs/hermes-ui-spec/017/evidence/picker-position-0c0704d/verify.py
# Official toolchain / developer machine: real primary CLI regression.
cargo build --locked -p hermes-rs --bin hermes-rs
HERMES_PICKER_BINARY=target/debug/hermes-rs HERMES_PICKER_RECORDING=/NEW/primary.json python3 scripts/test_picker_footer_position.py
# Ten real Rust captures, from a clean committed Rust tree.
python3 scripts/capture_picker_diagnostic.py target/debug/hermes-rs /NEW/picker.json
```

For images use the pinned scripts/visual-renderer/package-lock.json install:
`node scripts/visual-renderer/render.cjs PAIRED_JSON NEW_DIRECTORY ui`.
The sandbox uses isolated pinned Sparticuz al2023.tar.br NSS/NSPR libraries.
Never reuse an existing recording/render directory. Run `sha256sum -c SHA256SUMS`
inside this packet directory. Geometry inspection uses terminal cells, not ANSI
spelling; counter checks' whitespace trimming is not geometry normalization.

## Limited direct Standards / Spec review

Scope:16ae36b…0c0704d, with Spec017/T13 V2 as the originating requirement.
Direct review only, not independent subagents or personal Matt Pocock approval.

**Standards:** no new hard violation found. The primary regression observes the
real CLI at the agreed seam with an independent expected row. Minimal production
change preserves existing terminal ownership/error propagation and avoids newline
movement after the footer. No broader refactor was justified. Bounded runner,
read-only permissions, SHA-pinned actions and90-day artifact policy remain.

**Spec:** the footer-position slice satisfies the demonstrated fixed-size defect,
with counter/hint and delete/empty controls retained. No new adaptation. Overall
visual parity remains incomplete for the differences/coverage listed above.
Supporting tests:11 workflow-policy,4 geometry-evidence,5 recorder,3 retained-audit
checks PASS (23 total), plus the official Rust/CLI results above. No merge, final
user acceptance or Spec017 closure is implied.
