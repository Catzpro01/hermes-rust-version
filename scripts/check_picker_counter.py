"""Check retained two-session PTY footer counts and the fixed delete hints.

Expected strings come from the pinned Python fixture, not Rust formatting.
This checks the recorded source, not whatever source is currently on disk.
"""
import argparse
import base64
import json
from pathlib import Path
import sys

import pyte
from check_picker_filter_hint import check as check_hints

EXPECTED = {
    'picker-normal': '1/2 sessions   d delete',
    'picker-filter': '1/1 sessions (filtered from 2)',
    'picker-no-match': '0/2 sessions',
}


def check(bundle, side):
    failures = check_hints(bundle, side)  # Also validates the exact matrix/streams.
    for case in bundle['cases']:
        if case['scenario'] not in EXPECTED:
            continue
        record = case[side]
        screen = pyte.Screen(record['width'], record['height'])
        pyte.ByteStream(screen).feed(base64.b64decode(record['raw_base64'])[:record['snapshot_end_byte']])
        footers = [line.strip() for line in screen.display if 'sessions' in line
                   and 'Browse' not in line and 'No sessions' not in line]
        assert len(footers) == 1, (case['id'], 'footer not reached')
        expected = EXPECTED[case['scenario']]
        verdict = 'PASS' if footers[0] == expected else 'FAIL'
        print(f'{verdict} counter {side} {case["id"]}: actual={footers[0]!r}; expected={expected!r}')
        if footers[0] != expected:
            failures.append(case['id'])
    return failures


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--side', choices=['python', 'rust'], required=True)
    args = parser.parse_args()
    sys.exit(bool(check(json.loads(args.bundle.read_text()), args.side)))
