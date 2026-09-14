"""Observe the selected-row styling of the session picker.

The retained Python reference draws the whole cursor row — marker included — in
palette2 green plus bold with no reverse video, at both pinned widths. pyte
represents the bright ANSI form as `green` and the indexed form as `00cd00`,
the same accepted pair the palette3/palette6/palette8 slices already shipped.

Only the styling and the name-field span are asserted here: the status,
`Active` and `ID` column *content* is a documented adaptation, so the pixel
regions this gate pins stop at the status column.
"""
import argparse
import json
from pathlib import Path
import sys

from check_picker_footer_position import screen_for

SELECTED_STYLES = {('green', True), ('00cd00', True)}
MARKER = ' → '
SCENARIOS = ('picker-normal', 'picker-filter', 'picker-delete')


def cursor_rows(record):
    screen = screen_for(record)
    return [row for row, line in enumerate(screen.display) if line.startswith(MARKER)]


def row_styles(screen, row):
    line = screen.display[row].rstrip()
    if not line:
        raise RuntimeError(f'Row {row + 1} is empty')
    return line, [(screen.buffer[row][x].fg, screen.buffer[row][x].bold,
                   screen.buffer[row][x].reverse) for x in range(len(line))]


def case_problems(record, scenario):
    screen = screen_for(record)
    problems = []
    rows = cursor_rows(record)
    if rows != [3]:
        problems.append(f'cursor marker rows={[row + 1 for row in rows]}, expected exactly row 4')
        return problems
    line, styles = row_styles(screen, 3)
    if any(reverse for _, _, reverse in styles):
        problems.append('reverse video still set on the cursor row')
    inks = {(fg, bold) for fg, bold, _ in styles}
    if len(inks) != 1 or inks - SELECTED_STYLES:
        problems.append(f'cursor row style {sorted(inks)}, expected palette2 green + bold only')
    # The rows below must not inherit the cursor styling. Their own ink is not
    # asserted here: the status column has no pinned evidence for the Rust value.
    below = screen.display[4].rstrip()
    if below:
        _, styles = row_styles(screen, 4)
        if any(reverse for _, _, reverse in styles):
            problems.append('reverse video leaked to unselected row 5')
        if any(bold for _, bold, _ in styles):
            problems.append('bold leaked to unselected row 5')
        if any((fg, bold) in SELECTED_STYLES for fg, bold, _ in styles):
            problems.append('cursor-row green leaked to unselected row 5')
    return problems


def check(bundle, side):
    cases = [c for c in bundle['cases'] if c['scenario'] in SCENARIOS]
    expected = {f'{name}-{width}x30' for name in SCENARIOS for width in (100, 80)}
    if len(cases) != len(expected) or {c['id'] for c in cases} != expected:
        raise RuntimeError('Incomplete or duplicate selected-row matrix')
    failures = []
    for case in cases:
        record = case[side]
        if case['id'] != f'{case["scenario"]}-{record["width"]}x{record["height"]}':
            raise RuntimeError('Terminal dimensions do not match case id')
        problems = case_problems(record, case['scenario'])
        passed = not problems
        detail = 'palette2 green + bold, no reverse video' if passed else '; '.join(problems)
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
