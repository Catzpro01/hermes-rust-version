"""Observe the pinned picker column layout through retained terminal output.

The retained Python reference pins two widths (100 and 80 columns). It shows a
three-cell cursor column as part of every body row (` → ` on the cursor row,
three spaces otherwise), one blank row between the column header and the body,
and a column header that carries its own name field: `Stat` lands at width-54
while the body status column sits at width-57, exactly as the reference rows do.

Field widths are *inferred from the two pinned widths only* — the body name
field is width-62 and the header field width-59, both floored at the 80-column
value of 20. Terminals narrower than 80 columns are not evidenced by the
reference and are not claimed here. Colours are matched by palette slot
(`brightblack` for the ANSI16 spelling, `7f7f7f` for the indexed 256 form, the
already-accepted palette8 equivalence); the header must not be bold.
"""
import argparse
import json
from pathlib import Path
import sys

from check_picker_footer_position import screen_for

NO_MATCH = '  No sessions match the filter.'
HEADER_INK = {('brightblack', False), ('7f7f7f', False)}
MARKER = ' → '


def header_field(width):
    """Header name field: the reference shows width-59, floored at 20."""
    return max(20, width - 59)


def body_field(width):
    """Body name field: the reference shows width-62, floored at 20."""
    return max(20, width - 62)


def header_columns(width):
    stat = 3 + header_field(width) + 2
    return {'Stat': stat, 'Msgs': stat + 8, 'Active': stat + 14, 'Src': stat + 26}


def body_columns(width):
    status = 3 + body_field(width) + 2
    return {'status': status, 'active': status + 14, 'src': status + 26,
            'sid': status + 32}


def header_style(screen, line):
    cells = [screen.buffer[1][x] for x in range(len(line))]
    return {(c.fg, c.bold) for c in cells}


def row_prefix(text, selected):
    if selected:
        return text.startswith(MARKER)
    return text.startswith('   ') and text[:3] == '   '


def case_problems(record, scenario):
    screen = screen_for(record)
    width = record['width']
    problems = []

    header = screen.display[1].rstrip()
    if not header.startswith('   Title / Preview'):
        problems.append(f'header does not start with the three-cell indent: {header!r}')
    else:
        for label, x in header_columns(width).items():
            if header.index(label) != x:
                problems.append(f'header {label} at x={header.index(label)}, expected {x}')
        style = header_style(screen, header)
        if style not in ({ink} for ink in HEADER_INK):
            problems.append(f'header style {sorted(style)}, expected palette8 brightblack without bold')

    if screen.display[2].strip():
        problems.append(f'row 3 must stay blank, got {screen.display[2].rstrip()!r}')

    if scenario == 'picker-no-match':
        if screen.display[3].rstrip() != NO_MATCH:
            problems.append(f'no-match message row={screen.display[3].rstrip()!r}')
        return problems

    for index, selected in ((3, True), (4, False)):
        text = screen.display[index].rstrip()
        if not row_prefix(text, selected):
            problems.append(f'row {index + 1} prefix {text[:3]!r}, expected '
                            f'{"cursor arrow" if selected else "three spaces"}')
            continue
        columns = body_columns(width)
        status = text[columns['status']:columns['status'] + 5]
        if not status.strip() or text[columns['status'] - 1] != ' ':
            problems.append(f'row {index + 1} status column at x={columns["status"]} '
                            f'is not a padded field: {text!r}')
        if text[columns['active'] - 1] != ' ':
            problems.append(f'row {index + 1} active column at x={columns["active"]} '
                            f'is not padded: {text!r}')
        if text[columns['src']:columns['src'] + 3] != 'cli':
            problems.append(f'row {index + 1} source column at x={columns["src"]}: {text!r}')
        sid = text[columns['sid']:columns['sid'] + 8]
        if len(sid) != 8 or not sid.isalnum():
            problems.append(f'row {index + 1} id column at x={columns["sid"]}: {text!r}')
    return problems


def check(bundle, side):
    names = ('picker-normal', 'picker-no-match')
    cases = [c for c in bundle['cases'] if c['scenario'] in names]
    if len(cases) != 4 or {c['id'] for c in cases} != {f'{n}-{w}x30' for n in names for w in (100, 80)}:
        raise RuntimeError('Incomplete or duplicate four column-layout matrix')
    failures = []
    for case in cases:
        record = case[side]
        if case['id'] != f'{case["scenario"]}-{record["width"]}x{record["height"]}':
            raise RuntimeError('Terminal dimensions do not match case id')
        problems = case_problems(record, case['scenario'])
        passed = not problems
        detail = 'reference geometry' if passed else '; '.join(problems)
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
