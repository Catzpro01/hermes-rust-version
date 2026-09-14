"""Replay actual PTY snapshots; check delete hints against filter state.

This checks a capture, not current source. Always recapture after a source fix.
"""
import argparse
import base64
import json
import hashlib
from pathlib import Path
import sys

import pyte


def check(bundle, side):
    targets = ('picker-normal', 'picker-filter', 'picker-no-match')
    cases = [c for c in bundle['cases'] if c['scenario'] in targets]
    expected = {f'{name}-{width}x30' for name in targets for width in (100, 80)}
    assert len(cases) == 6 and {c['id'] for c in cases} == expected, 'incomplete or duplicate matrix'
    failures = []
    for case in bundle['cases']:
        if case['scenario'] not in ('picker-normal', 'picker-filter', 'picker-no-match'):
            continue
        record = case[side]
        raw = base64.b64decode(record['raw_base64'], validate=True)
        assert record['error'] is None and 0 < record['snapshot_end_byte'] <= len(raw)
        assert hashlib.sha256(raw).hexdigest() == record['raw_sha256']
        assert b''.join(base64.b64decode(e[1], validate=True) for e in record['events_base64']) == raw
        screen = pyte.Screen(record['width'], record['height'])
        stream = pyte.ByteStream(screen)
        stream.feed(raw[:record['snapshot_end_byte']])
        footer = [line.strip() for line in screen.display if 'sessions' in line and 'Browse' not in line and 'No sessions' not in line]
        assert len(footer) == 1, (case['id'], 'footer not reached', footer)
        expected = case['scenario'] == 'picker-normal'
        actual = 'd delete' in footer[0]
        result = 'PASS' if actual == expected else 'FAIL'
        print(f'{result} {side} {case["id"]}: expected delete_hint={expected}; {footer[0]!r}')
        if actual != expected:
            failures.append(case['id'])
    assert len([c for c in bundle['cases'] if c['scenario'] in ('picker-normal', 'picker-filter', 'picker-no-match')]) == 6
    return failures


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--side', choices=['python', 'rust'], required=True)
    args = parser.parse_args()
    sys.exit(bool(check(json.loads(args.bundle.read_text()), args.side)))
