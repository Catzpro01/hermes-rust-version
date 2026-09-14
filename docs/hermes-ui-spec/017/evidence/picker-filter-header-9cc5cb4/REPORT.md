# Spec 017 — session picker filter help header (palette slot 6 + bold)

**Result: REVIEWED — filter help header corrected and verified. NOT whole-screen
parity, NOT Spec017 closure, no acceptance claimed.**

This slice fixes one behaviour: the `Browse sessions` help line while a filter is
typed (`picker-filter` and `picker-no-match`). The pinned Python v0.21.0 reference
draws that row bold with palette slot 6 (`SGR 0;1` then `36`); the Rust port drew
it with default attributes. No text, geometry, footer, counter, deletion flow or
empty-store behaviour is changed here.

## What was done

| Stage | Evidence |
|---|---|
| Contract | Pinned reference `63279301…` captured Python records (retained `ui-3b39bd7` bundle) decode to `{('cyan', True)}` for all four filter/no-match cases |
| RED (real CLI/PTY) | [34858664487](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34858664487) — live regression FAILs three times on the committed binary; earlier RED [34857289160](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34857289160) recorded the first (default-attribute) failure |
| GREEN (official) | [34858865863](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34858865863) — fmt/check/clippy, full workspace suite and the strict live regression three times |
| Exact patch | 598 bytes, SHA-256 `21fcdead245179fb4eda6032fd39f5f5593b3ae8c4c5efe01b78bb676ef2a6d1`, fetched from the run's annotations, byte-identical to the bound proposal and to the applied diff |
| Committed-source capture | [34859140850](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34859140850) — clean tree asserted, ten cases recaptured, hint/counter/position/colour/normal-header/filter-header checks and the live regression all pass |
| Ordinary CI | [34859140646](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34859140646) on the same commit |

Rust bundle: 98,231 bytes, SHA-256
`a9dd557366f2f391df1daaff4ab75fb88b26c0f034ab72c30ddf646c339dd85c`, captured at
`9cc5cb4c416df66f43cd30c33adf8055b82269ab`, whose only difference from the fix
commit `48cf58715f2164c53cbab115df34e7cbe3c47fbc` is the diagnostic request file
(asserted in `verify.py`).

## The near-miss this slice caught

The first accepted proposal used crossterm's `Color::Cyan`, which this capture
environment encodes as `38;5;14` — **bright** cyan, a different palette entry
from the reference's slot 6. The pinned renderer happened to draw both panes
identically, so only the cell-level slot comparison exposed it. The gate was
tightened to accept slot 6 only (`cyan` or `00cdcd`), the bright variant became
the recorded RED of run `34858664487`, and the corrected one-token proposal
(`Color::DarkCyan`, `38;5;6`) is what this packet evidences. The rejected
attempt is retained in `first-attempt-bright-variant.txt`; nothing was
normalized away.

## Results

- Filter/no-match header row, all four cases (100×30 and 80×30):
  Python `fg=6, mode=palette16, bold`; Rust `fg=6, mode=palette256, bold`. Same
  palette slot, same text, same geometry, bold on both sides; the encoder-mode
  difference is retained and recorded, not normalized.
- Rendered row-1 pixels are **identical** between the Python and Rust panes in
  all four cases (`filter-header-pixels.json`, pinned renderer, zero differing
  pixels).
- Only row 1 of those four cases changed; the other six PNGs
  (normal/delete/empty at both widths) are byte-identical to the previous packet
  and the Python records/cells are unchanged.
- 20 raw/`.cast` round trips, 10 PNG hash checks, 4 slot checks and 4 pixel
  comparisons pass; `verify.py` reproduces all of it.

## Limitations

- Scope is the picker header subset plus the retained regression checks, not
  whole-screen picker parity. Column-header colour, selected-row styling,
  delete-prompt colour, status/`Active`/`ID` column content and remaining body
  geometry are still open, as are wizard/completion evidence and T12 coverage.
- Python streams are reused unchanged from the retained `ui-3b39bd7` capture
  (not a new Python run, not a new source audit).
- Resize, long lists and clear-filter behaviour remain unproven.
- No acceptance, no Spec017 closure, and no merge is claimed or performed.

## Reproduction

```sh
PYTHONPATH=/tmp/picker-qa python3 docs/hermes-ui-spec/017/evidence/picker-filter-header-9cc5cb4/verify.py
python3 scripts/check_picker_filter_header.py \
  docs/hermes-ui-spec/017/evidence/picker-filter-header-9cc5cb4/paired-bundle.json --side rust
python3 scripts/check_picker_filter_header.py \
  docs/hermes-ui-spec/017/evidence/picker-filter-header-9cc5cb4/paired-bundle.json --side python
# Live regression, after building the CLI:
HERMES_PICKER_BINARY=target/debug/hermes-rs HERMES_PICKER_RECORDING=/NEW/filter.json \
  python3 scripts/test_picker_filter_header.py
```

Pixel replay (pinned renderer; needs `npm ci --prefix scripts/visual-renderer`
and the packaged Chromium libraries, as in `scripts/visual-renderer/README.md`):

```sh
NODE_PATH=scripts/visual-renderer/node_modules node \
  docs/hermes-ui-spec/017/evidence/picker-filter-header-9cc5cb4/verify-pixels.cjs \
  docs/hermes-ui-spec/017/evidence/picker-filter-header-9cc5cb4 \
  /NEW/filter-header-pixels.json \
  picker-filter-100x30 picker-filter-80x30 picker-no-match-100x30 picker-no-match-80x30
```

## Files

`rust-bundle.json` / `paired-bundle.json` (Python side from `python_origin`),
per-case `.ansi`, `.cast`, `-cells.json.gz`, `-bottom.png`, `renderer.json`,
`verify-pixels.cjs` + `filter-header-pixels.json`, `tested.patch`,
`red-result.txt` / `green-result.txt` / `capture-result.txt`,
`reference-python.txt`, `baseline-rust.txt`, `corrected-filter-header.txt`,
`first-attempt-bright-variant.txt`, `verification-runs.json`, `pair-bundle.py`,
`verify.py`, `audit.json`, `SHA256SUMS`.
