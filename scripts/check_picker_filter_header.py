"""Observe the filter-mode help header through retained public terminal output.

Palette6 is cyan in pyte0.8.2; it is a base ANSI colour, so both the Python
reference and the Rust port decode to the same name. These decoder
representations are not colour normalization; the xterm acceptance audit
verifies the actual palette index/mode, attributes and rendered pixels
separately.
"""
import argparse
import json
from pathlib import Path
import sys

from check_picker_footer_position import screen_for

FILTER_PREFIX = '  Browse sessions — filter: '
BLOCK = '█'


def filter_header_style(record):
    screen = screen_for(record)
    line = screen.display[0].rstrip()
    if not line.startswith(FILTER_PREFIX) or not line.endswith(BLOCK):
        raise RuntimeError('Capture did not reach the filter help header at row1')
    return {(c.fg, c.bold) for c in screen.buffer[0].values() if c.data.strip()}


def check(bundle, side):
    names = ('picker-filter', 'picker-no-match')
    cases = [c for c in bundle['cases'] if c['scenario'] in names]
    if len(cases) != 4 or {c['id'] for c in cases} != {f'{n}-{w}x30' for n in names for w in (100, 80)}:
        raise RuntimeError('Incomplete or duplicate four filter-header matrix')
    failures = []
    for case in cases:
        record = case[side]
        if case['id'] != f'{case["scenario"]}-{record["width"]}x{record["height"]}':
            raise RuntimeError('Terminal dimensions do not match case id')
        style = filter_header_style(record)
        passed = style in ({('cyan', True)},)
        print(f'{"PASS" if passed else "FAIL"} {side} {case["id"]}: style={sorted(style)}; reference=palette6+bold')
        if not passed:
            failures.append(case['id'])
    return failures


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--side', choices=['python', 'rust'], required=True)
    args = parser.parse_args()
    sys.exit(bool(check(json.loads(args.bundle.read_text()), args.side)))
