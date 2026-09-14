"""Observe actual terminal coordinates; never infer them from ANSI spellings."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import sys

import pyte


def screen_for(record):
    raw = base64.b64decode(record['raw_base64'], validate=True)
    if record['error'] or not 0 < record['snapshot_end_byte'] <= len(raw):
        raise RuntimeError('UI capture did not reach its snapshot')
    if hashlib.sha256(raw).hexdigest() != record['raw_sha256']:
        raise RuntimeError('Corrupt raw capture')
    if b''.join(base64.b64decode(e[1], validate=True) for e in record['events_base64']) != raw:
        raise RuntimeError('Corrupt event recording')
    screen = pyte.Screen(record['width'], record['height'])
    pyte.ByteStream(screen).feed(raw[:record['snapshot_end_byte']])
    return screen


def footer_row(record):
    screen = screen_for(record)
    positions = [y for y, line in enumerate(screen.display) if 'sessions' in line
                 and 'Browse' not in line and 'No sessions' not in line]
    if len(positions) != 1:
        raise RuntimeError(f'Expected one reached footer, got {positions}')
    return positions[0]


def check(bundle, side):
    names = ('picker-normal', 'picker-filter', 'picker-no-match', 'picker-delete', 'picker-empty')
    cases = [c for c in bundle['cases'] if c['scenario'] in names]
    if len(cases) != 10 or {c['id'] for c in cases} != {f'{n}-{w}x30' for n in names for w in (100, 80)}:
        raise RuntimeError('Incomplete or duplicate ten-case matrix')
    failures = []
    for case in cases:
        record = case[side]
        if case['id'] != f'{case["scenario"]}-{record["width"]}x{record["height"]}':
            raise RuntimeError('Terminal dimensions do not match case id')
        screen = screen_for(record)
        if case['scenario'] == 'picker-empty':
            passed = screen.display[0].strip() == 'No sessions found.'
            detail = 'empty-store message at row1'
        elif case['scenario'] == 'picker-delete':
            positions = [y for y, line in enumerate(screen.display) if "Delete session 'second topic'? [y/N]" in line]
            passed = positions == [record['height']-1]
            detail = f'delete prompt rows={[y+1 for y in positions]}'
        else:
            actual = footer_row(record)
            passed = actual == record['height']-1
            detail = f'footer row={actual+1}; expected={record["height"]}'
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
