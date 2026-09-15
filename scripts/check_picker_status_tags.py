"""Observe the status-column CONTENT of the session picker (Spec 017 W5).

`check_picker_status_ink.py` pins the ink and the column of the status tag.
This checker pins the half that was missing: which lifecycle state a row may
report at all. The pinned reference derives the state from the session's LAST
message row — never from "does the session have turns" — and it distinguishes
four shapes (see `docs/hermes-ui-spec/017/evidence/upstream-lifecycle-status/`):

  finish_reason in {error, agent_error, content_filter}   -> error       -> `err`
  last row is an assistant row carrying tool_calls       -> interrupted -> `intr`
  last row is a user or tool row                         -> interrupted -> `intr`
  any other last row                                     -> complete    -> `done`
  no message row at all                                  -> empty       -> `empty`

The fixture seeds exactly one session per shape, in that listing order, so the
expected sequence is the table in `capture_ui.LIFECYCLE_TAGS`. A port that
collapses every non-empty session onto `done` therefore fails on the three
`intr` rows and the `err` row, and each message names the row it is judging.
The middle `intr` row is the tool-result row of ADR 0007: the reference reads
its role (`shell`, the tool's own name) as `complete` and renders `done`, while
Hermes-RS renders `intr`. The gate is run per side, so the Rust expectation is
the one that carries the adaptation.

Like the ink checker, nothing is normalized: the tag is read off the rendered
screen at the pinned column, and the cursor row's ink is deliberately not
asserted here — the reference recolours the tag only `if i != cursor`, and the
selection styling cycle owns that row.
"""
import argparse
import json
from pathlib import Path
import sys

from capture_ui import LIFECYCLE_TAGS
from check_picker_footer_position import screen_for
from check_picker_status_ink import SLOT_SPELLINGS, TAG_SLOT, TAG_WIDTH, tag_column

SCENARIO = 'picker-status-tags'
WIDTHS = (100, 80)
CURSOR_PREFIX = ' \u2192 '


def _rows(screen, record):
    """(row index, tag text, is_cursor, inks, bold) per rendered body row."""
    start = tag_column(record)
    rows = []
    for row in range(3, record['height'] - 1):
        line = screen.display[row]
        if not line.strip() or len(line) < start + TAG_WIDTH:
            continue
        cells = [screen.buffer[row][x] for x in range(start, start + TAG_WIDTH)]
        tag = ''.join(cell.data for cell in cells).strip()
        rows.append((row, tag, line.startswith(CURSOR_PREFIX),
                     {cell.fg for cell in cells}, {cell.bold for cell in cells}))
    return rows


def row_tags(record):
    """(row index, tag text, is_cursor) — the shape this gate is judged on."""
    return [(row, tag, is_cursor) for row, tag, is_cursor, _, _ in
            _rows(screen_for(record), record)]


def case_problems(record):
    screen = screen_for(record)
    rows = _rows(screen, record)
    if len(rows) != len(LIFECYCLE_TAGS):
        return [f'expected {len(LIFECYCLE_TAGS)} lifecycle rows, found {len(rows)}: '
                f'{[tag for _, tag, _, _, _ in rows]}']
    problems = []
    for (row, tag, is_cursor, inks, bold), expected in zip(rows, LIFECYCLE_TAGS):
        if tag != expected:
            problems.append(f'row {row + 1}: status tag {tag!r}, expected {expected!r} '
                            f'for the seeded lifecycle shape')
            continue
        if is_cursor:
            continue
        if not inks <= SLOT_SPELLINGS[TAG_SLOT[expected]]:
            problems.append(f'row {row + 1}: status tag {expected!r} ink {sorted(inks)}, '
                            f'expected palette slot {TAG_SLOT[expected]}')
        if bold != {False}:
            problems.append(f'row {row + 1}: status tag {expected!r} must not be bold '
                            f'(`_status_attr` carries only the colour pair)')
    return problems


def check(bundle, side):
    cases = [c for c in bundle['cases'] if c['scenario'] == SCENARIO]
    expected = {f'{SCENARIO}-{width}x30' for width in WIDTHS}
    if {c['id'] for c in cases} != expected:
        raise RuntimeError(f'Expected exactly the {sorted(expected)} lifecycle cases, '
                           f'found {sorted(c["id"] for c in cases)}')
    failures = []
    for case in cases:
        record = case[side]
        if case['id'] != f'{case["scenario"]}-{record["width"]}x{record["height"]}':
            raise RuntimeError('Terminal dimensions do not match case id')
        problems = case_problems(record)
        passed = not problems
        detail = ('one row per pinned lifecycle shape, tags in listing order'
                  if passed else '; '.join(problems))
        print(f'{"PASS" if passed else "FAIL"} {side} {case["id"]}: {detail}')
        if not passed:
            failures.append(case['id'])
    return failures


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--side', choices=['python', 'rust'], required=True)
    args = parser.parse_args()
    sys.exit(bool(check(json.loads(args.bundle.read_text()), args.side)))
