# Normal picker help header — actual CLI TDD verified

**NORMAL_HELP_HEADER_FIXED — not whole-picker parity or Spec017 closure.**
All ten paired PNGs were directly opened and inspected. The normal-mode
`Browse sessions` help header now uses palette3 + bold at row1, matching the
retained Python reference at100×30 and80×30. Four header regions are pixel-identical.
See the user-facing [milestones](../../MILESTONES.md).

## Test-first chain

The user explicitly requested `/tdd header bantuan picker mode normal`, continuing
the recommended actual CLI/PTY seam. One primary normal100×30 test,
`PickerNormalHeaderTests.test_normal_help_header_matches_reference_style`, launches
the real CLI through the existing isolated recorder and preserves its trace before
checking visible header foreground/weight. Expected style comes from the independent
Python recording, not private Rust state, mocks, or an ANSI spelling assertion.
Missing/corrupt/unreached header conditions are ERROR, not style RED.

Historical Python normal/delete checks passed4/4; Rust failed4/4. The live primary
then failed before any production fix was proposed. The cause was the normal help
line following the unstyled `Print(line)` branch. A minimal branch now sets
`Color::DarkYellow` and `Attribute::Bold` for the first frame line only when the
filter is empty, prints it, then resets attributes. Header text and coordinates,
column/filter headers, selection, delete prompt and footer implementation remain
unchanged. The normal header also remains on screen during delete confirmation;
that does not extend this change to the prompt's own styling.

| Stage | Source / run | Result |
|---|---|---|
| Live primary RED | 4aa0858 / [34836863589](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34836863589) | fmt/check/build PASS; actual header test FAIL3 at default foreground/non-bold |
| Candidate GREEN | 645f86d / [34837102702](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34837102702) | primary3, fmt/check/clippy/full workspace PASS |
| Exact patch applied | 7fef5140c9242588ddb46749aafcdd7542a559c9 | Officially tested1171-byte patch; local full Rust diff byte-identical |
| Committed-source capture | [34837424495](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34837424495) | Full validation, primary3 and ten captures PASS; header4, footer color6/geometry10/counter6/hint6 PASS |
| Ordinary CI, same source | [34837424464](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34837424464) | SUCCESS; live position, footer color and normal-header tests required |

A successful RED job means expected assertion failures, not a passing product.
`red-result.txt`, `primary-red.json`, `tested.patch`, `green-result.txt`,
`capture-result.txt` and `verification-runs.json` retain the diagnostic chain.
No local cargo was used; official Rust checks preceded source application.
Primary RED trace11973 bytes SHA-256
`56ecf3af0ea8fc93b369d0f23120277926f6ae2d9b6605c957a8b0cb754d5e7e`;
its Rust worktree diff was empty. Tested patch SHA-256
`90fa6de6e8c260cb783260bafb4bc1502fad98ec6a332fc42c7fa5363c4dfd54`.

## Inspected matrix and isolation

| Scenario | 100×30 | 80×30 | Observed result |
|---|---|---|---|
| Normal | [inspected](picker-normal-100x30-bottom.png) | [inspected](picker-normal-80x30-bottom.png) | Normal help header palette3 + bold, row1; footer preserved |
| Filter `topic` | [inspected](picker-filter-100x30-bottom.png) | [inspected](picker-filter-80x30-bottom.png) | Unchanged whole paired PNG; filter-header styling remains pending |
| No match `zzzz` | [inspected](picker-no-match-100x30-bottom.png) | [inspected](picker-no-match-80x30-bottom.png) | Unchanged whole paired PNG; `0/2 sessions` preserved |
| Delete confirmation | [inspected](picker-delete-100x30-bottom.png) | [inspected](picker-delete-80x30-bottom.png) | Normal help header corrected; prompt remains unchanged at row30 |
| Empty store | [inspected](picker-empty-100x30-bottom.png) | [inspected](picker-empty-80x30-bottom.png) | Unchanged whole paired PNG, message row1 |

Against picker-color-ae220ff, **only foreground and bold on row1** change in the
four normal/delete captures. All other cell properties, text, wrapping and rows
are identical. There is no style leakage to the column header, selected/table
rows, delete prompt or footer in the captured states. All six filter/no-match/
empty PNGs are byte-identical to the prior packet. All ten Python records and
cell dumps are unchanged. Rust raw streams are fresh recordings, not claimed
byte-identical when their final screenshots match.

### Indexed modes and pixels

Python uses palette16, Rust palette256; both select **index3 with bold=true and
dim=false**, not index11 or a hard-coded RGB approximation. Xterm foreground-mode
flags16777216/33554432 are retained as observed; they are not rewritten to match.
All other properties of the normal header agree with the Python reference.

`verify-pixels.cjs` decodes the original paired PNGs and compares the complete
first terminal row on both sides. For the unchanged pinned renderer, each of the
30 terminal rows is19px high; row1 is at image y31, below the renderer title.
The full pane width is compared, excluding only the24px pair separator. The
four comparisons are pixel-identical. Exact rectangles and input PNG hashes
are in `header-pixels.json`. This is a row-level proof, not whole-screen parity;
no originals, colors, text or geometry were normalized.

Pyte0.8.2 exposes palette3 as `brown` or `cdcd00`, depending on encoding. The
primary regression accepts both representations with bold=true. Pyte alone
cannot distinguish matching truecolor; the acceptance xterm audit checks actual
indexed mode/index and the remaining attributes. Synthetic decoder checks are
supporting tests only, not claimed as actual application RED.

## Provenance / integrity

- Fresh Rust bundle102020 bytes, SHA-256
  `f41d50ce422ee795a1fc7b1c5981b739a808389f97c4309e36df05970842b58f`.
  Source/annotation digest verified; binary, driver/helper hashes, compiler/Cargo
  versions, commands, input/output events, endpoints and fixture metadata retained.
  Capture refuses a dirty Rust tree. `verify.py` verifies recorder/helper hashes
  against their committed source blobs at7fef514.
- Python streams are **reused unchanged**, not a new Python run or source audit.
  Parent `ui-3b39bd7/python-bundle.json` SHA-256
  `3cb5a47246d5b0ccb47c20cff20b48b3b589b5e7be5529666455fd9c27cfd48a`;
  original checkout1c3c9dd, upstream63279301bcbdc185c1b07b98a9312eb0c862f26d.
  The5128-file source audit remains historical. No real Python installation/home
  or user session data was modified.
- Recorder unchanged SHA-256
  `96695c3f647a59d2a7e41a43f80d98257273873feaefb837a854e45196e5598e`. Existing
  isolated two-session fixture, fixed UUIDs/timestamps1700000000/1. No credentials
  supplied or external model/service calls. Delete is displayed, not accepted;
  empty store is separately isolated.
- Matched100×30/80×30, xterm-256color/truecolor, C.UTF-8. Renderer unchanged:
  xterm5.5.0, Chromium138.0.7204.0, DejaVu Mono5.2.5,16px, scale1. Browser,
  font hashes and settings agree with the previous packet.
- `verify.py`/`audit.json`:20 raw/cast roundtrips,10 PNG hashes,4 normal headers,
 4 header pixel comparisons, unchanged Python provenance and scoped cell changes.
  `SHA256SUMS` covers packet files except itself. Direct image inspection is
  documented separately; the scripts do not claim agent/human review.

## Reproduce

Use pyte0.8.2/wcwidth0.8.3 on PYTHONPATH. From the repository root:

```sh
# Historical header RED: expected exit1 with four failures.
python3 scripts/check_picker_normal_header.py docs/hermes-ui-spec/017/evidence/picker-color-ae220ff/paired-bundle.json --side rust
# Corrected capture and prior behavior: expected exit0.
python3 scripts/check_picker_normal_header.py docs/hermes-ui-spec/017/evidence/picker-header-7fef514/paired-bundle.json --side rust
python3 scripts/check_picker_footer_color.py docs/hermes-ui-spec/017/evidence/picker-header-7fef514/paired-bundle.json --side rust
python3 scripts/check_picker_footer_position.py docs/hermes-ui-spec/017/evidence/picker-header-7fef514/paired-bundle.json --side rust
python3 scripts/check_picker_counter.py docs/hermes-ui-spec/017/evidence/picker-header-7fef514/paired-bundle.json --side rust
python3 docs/hermes-ui-spec/017/evidence/picker-header-7fef514/verify.py
# Official Rust toolchain / developer machine:
cargo build --locked -p hermes-rs --bin hermes-rs
HERMES_PICKER_BINARY=target/debug/hermes-rs HERMES_PICKER_RECORDING=/NEW/header.json python3 scripts/test_picker_normal_header.py
python3 scripts/capture_picker_diagnostic.py target/debug/hermes-rs /NEW/picker.json
```

With the pinned scripts/visual-renderer/package-lock.json dependency installation:
`node scripts/visual-renderer/render.cjs PAIRED_JSON NEW_DIRECTORY ui`.
Recheck pixels with
`node docs/hermes-ui-spec/017/evidence/picker-header-7fef514/verify-pixels.cjs`.
The sandbox uses NODE_PATH for the isolated pinned install and LD_LIBRARY_PATH
for its Sparticuz al2023.tar.br NSS/NSPR libraries. Existing recordings/render
folders must not be overwritten. Run `sha256sum -c SHA256SUMS` in this packet.

## Standards — limited direct review

Scopeb193250…7fef514; direct review under the previously disclosed alternative,
not independent subagents or personal Matt Pocock approval.

No new hard violation found. The primary test observes the agreed real CLI seam;
expected style is independent and encoding-neutral. One scoped renderer branch
uses existing crossterm primitives and resets attributes before subsequent rows.
No domain/API change, new feature or speculative production refactor. Workflow
policy rejects missing tests/setup errors as RED and requires all three live
picker tests in ordinary CI; read-only/SHA-pinned/90-day policies remain.
One nonblocking maintainability observation persists: duplicated CLI diagnostic
runner/test setup can be consolidated at a separate review, not disguised as a
normal-header behavior change.

## Spec — limited direct review

The requested normal help-header slice passes in both widths, including while
its delete confirmation is displayed. Prior footer fixes are preserved. No new
visual adaptation is introduced. Milestone H1–H5 is complete for this slice only.

Filter/column headers, selected-row styling, delete/no-match colors, other body
geometry, wizard/completion differences and T12 nested-field/full-Python-CLI
coverage remain open. Resize, long-list and clear-filter behavior are not proved
by these fixed-size fixtures. None is silently marked complete or waived.

Supporting checks:14 workflow-policy,4 header-checker,4 color-checker,4 geometry-
checker,5 recorder and3 retained-audit tests PASS (34 total), plus official
Rust/CLI results above. **No merge, final user acceptance or Spec017 closure.**
