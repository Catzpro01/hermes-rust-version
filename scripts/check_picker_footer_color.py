"""Observe retained terminal footer foreground; not a whole-screen style waiver.

pyte0.8.2 represents palette slot8 as brightblack for the bright ANSI form,
or 7f7f7f for the indexed form. No raw/image colors are rewritten. This gate
cannot distinguish truecolor with identical RGB; xterm acceptance retains mode.
"""
import argparse
import json
from pathlib import Path
import sys

from check_picker_footer_position import footer_row, screen_for


def footer_ink_colors(record):
    row = footer_row(record)
    screen = screen_for(record)
    if row != record['height'] - 1:
        raise RuntimeError('Color regression requires the already-correct bottom footer')
    colors = {cell.fg for cell in screen.buffer[row].values() if cell.data.strip()}
    if not colors:
        raise RuntimeError('Missing visible footer')
    return colors


def check(bundle, side):
    names = ('picker-normal', 'picker-filter', 'picker-no-match')
    cases = [c for c in bundle['cases'] if c['scenario'] in names]
    expected = {f'{n}-{w}x30' for n in names for w in (100, 80)}
    if len(cases) != 6 or {c['id'] for c in cases} != expected:
        raise RuntimeError('Incomplete or duplicate six-footer matrix')
    failures = []
    for case in cases:
        record = case[side]
        if case['id'] != f'{case["scenario"]}-{record["width"]}x{record["height"]}':
            raise RuntimeError('Terminal dimensions do not match case id')
        colors = footer_ink_colors(record)
        passed = colors in ({'brightblack'}, {'7f7f7f'})
        print(f'{"PASS" if passed else "FAIL"} {side} {case["id"]}: foreground={sorted(colors)}; reference=palette8')
        if not passed:
            failures.append(case['id'])
    return failures


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--side', choices=['python', 'rust'], required=True)
    args = parser.parse_args()
    sys.exit(bool(check(json.loads(args.bundle.read_text()), args.side)))
