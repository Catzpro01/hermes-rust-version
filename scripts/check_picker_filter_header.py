"""Observe the filter-mode help header through retained public terminal output.

pyte0.8.2 represents palette slot6 as `cyan` for the bright ANSI form (the
Python reference emits SGR 36) or `00ffff` for the indexed 256-colour form
(the Rust port emits 38;5;6 in this capture environment), exactly like the
already-accepted palette3 (`brown`/`cdcd00`) and palette8
(`brightblack`/`7f7f7f`) equivalences. These decoder representations are not
colour normalization; the xterm acceptance audit verifies the actual palette
index/mode, attributes and rendered pixels separately.
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
        passed = style in ({('cyan', True)}, {('00ffff', True)})
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
