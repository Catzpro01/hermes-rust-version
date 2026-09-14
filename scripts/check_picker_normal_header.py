"""Observe the normal-mode help header through retained public terminal output.

Palette3 is brown (ANSI16) or cdcd00 (indexed) in pyte0.8.2. These decoder
representations are not color normalization; xterm acceptance verifies the
actual palette index/mode, other attributes and rendered pixels separately.
"""
import argparse
import json
from pathlib import Path
import sys

from check_picker_footer_position import screen_for

NORMAL_HELP = '  Browse sessions — ↑↓ navigate  Enter select  Type to filter  Esc quit'


def normal_header_style(record):
    screen = screen_for(record)
    if screen.display[0].rstrip() != NORMAL_HELP:
        raise RuntimeError('Capture did not reach the unchanged normal help header at row1')
    return {(c.fg, c.bold) for c in screen.buffer[0].values() if c.data.strip()}


def check(bundle, side):
    names = ('picker-normal', 'picker-delete')
    cases = [c for c in bundle['cases'] if c['scenario'] in names]
    if len(cases) != 4 or {c['id'] for c in cases} != {f'{n}-{w}x30' for n in names for w in (100, 80)}:
        raise RuntimeError('Incomplete or duplicate four-header matrix')
    failures = []
    for case in cases:
        record = case[side]
        if case['id'] != f'{case["scenario"]}-{record["width"]}x{record["height"]}':
            raise RuntimeError('Terminal dimensions do not match case id')
        style = normal_header_style(record)
        passed = style in ({('brown', True)}, {('cdcd00', True)})
        print(f'{"PASS" if passed else "FAIL"} {side} {case["id"]}: style={sorted(style)}; reference=palette3+bold')
        if not passed:
            failures.append(case['id'])
    return failures


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--side', choices=['python', 'rust'], required=True)
    args = parser.parse_args()
    sys.exit(bool(check(json.loads(args.bundle.read_text()), args.side)))
