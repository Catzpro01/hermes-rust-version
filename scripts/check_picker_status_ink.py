"""Observe the status-tag ink of the session picker.

The retained spec §F names Python's `_status_attr` without its mapping, so the
mapping was read from the pinned upstream source
(`docs/hermes-ui-spec/017/evidence/upstream-status-attr/`, blob verified against
commit `63279301`): pair1 green for `complete`, pair2 yellow for `interrupted`,
pair5 red for `error`, pair4 palette8 for `empty`; `_session_status_tag` spells
those `done` / `intr` / `err` / `empty`, and the tag is recoloured in place, 5
cells wide, at `3 + name_width + 2`, only on rows that are not the cursor.

Corroboration from the retained capture: `intr` renders with palette index 3 and
the delete prompt with index 1, which matches pair2 and pair5.

pyte spells the slots `green`/`00cd00`, `brown`/`cdcd00`, `red`/`cd0000` and
`brightblack`/`7f7f7f`; both spellings are accepted, exactly like the palette3 /
palette6 / palette8 slices already shipped. Nothing is normalized: the tag text
is whatever the side under test draws (the Rust fixture's words are the
documented adaptation), and only the ink and column are asserted here.
"""
import argparse
import json
from pathlib import Path
import sys

from check_picker_footer_position import screen_for

TAG_WIDTH = 5
# Python's tag -> colour pair -> palette slot, from the pinned upstream source.
TAG_SLOT = {'done': 2, 'intr': 3, 'err': 1, 'empty': 8}
SLOT_SPELLINGS = {
    1: {'red', 'cd0000'},
    2: {'green', '00cd00'},
    3: {'brown', 'cdcd00'},
    8: {'brightblack', '7f7f7f'},
}
# `picker-filter` legitimately shows a single filtered row, which is the cursor
# row; there is then no unselected row to check, and that is not a failure.
MIN_ROWS = {'picker-normal': 1, 'picker-delete': 1, 'picker-filter': 0}
DEFAULT = {'default'}
SCENARIOS = ('picker-normal', 'picker-filter', 'picker-delete')


def tag_column(record):
    """0-based column of the 5-cell status tag (spec §F: 3 + name_width + 2)."""
    width = record['width']
    return max(20, width - 62) + TAG_WIDTH


def body_rows(record):
    screen = screen_for(record)
    start = tag_column(record)
    rows = []
    for row in range(3, record['height'] - 1):
        line = screen.display[row]
        if not line.strip() or line.startswith(' → '):
            continue
        if len(line) < start + TAG_WIDTH:
            continue
        rows.append((row, line))
    return rows


def case_problems(record, scenario):
    screen = screen_for(record)
    start = tag_column(record)
    problems = []
    rows = body_rows(record)
    if len(rows) < MIN_ROWS[scenario]:
        problems.append(f'expected at least {MIN_ROWS[scenario]} unselected body row(s), found {len(rows)}')
        return problems
    for row, line in rows:
        tag = ''.join(screen.buffer[row][x].data for x in range(start, start + TAG_WIDTH))
        word = tag.strip()
        if word not in TAG_SLOT:
            problems.append(f'row {row + 1}: status tag {tag!r} is not one of the four pinned tags')
            continue
        inks = {screen.buffer[row][x].fg for x in range(start, start + TAG_WIDTH)}
        bold = {screen.buffer[row][x].bold for x in range(start, start + TAG_WIDTH)}
        if not inks <= SLOT_SPELLINGS[TAG_SLOT[word]]:
            problems.append(f'row {row + 1}: status tag {word!r} ink {sorted(inks)}, '
                            f'expected palette slot {TAG_SLOT[word]}')
        if bold != {False}:
            problems.append(f'row {row + 1}: status tag {word!r} must not be bold')
        outside = {screen.buffer[row][x].fg for x in range(len(line.rstrip()))
                   if not start <= x < start + TAG_WIDTH}
        stray = outside - DEFAULT
        if stray:
            problems.append(f'row {row + 1}: ink {sorted(stray)} outside the status tag')
    return problems


def check(bundle, side):
    cases = [c for c in bundle['cases'] if c['scenario'] in SCENARIOS]
    expected = {f'{name}-{width}x30' for name in SCENARIOS for width in (100, 80)}
    if len(cases) != len(expected) or {c['id'] for c in cases} != expected:
        raise RuntimeError('Incomplete or duplicate status-tag matrix')
    failures = []
    for case in cases:
        record = case[side]
        if case['id'] != f'{case["scenario"]}-{record["width"]}x{record["height"]}':
            raise RuntimeError('Terminal dimensions do not match case id')
        problems = case_problems(record, case['scenario'])
        passed = not problems
        detail = 'status tag ink matches the pinned mapping' if passed else '; '.join(problems)
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
