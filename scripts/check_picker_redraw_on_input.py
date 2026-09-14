"""Observe that the picker draws because of input, not because of its poll timeout.

The retained reference (`_curses_browse` in the pinned upstream source) draws the
frame at the top of its loop and then blocks in `stdscr.getch()`: one frame per
key, and nothing at all while it waits. The Rust port polls with a 100 ms timeout
so it can stay responsive to signals, which is fine as long as it only redraws
when something changed.

This gate splits each recorded session at the scenario keystrokes and counts the
frames the child actually drew in every input window:

* window 0 is everything before the first keystroke — the reference draws exactly
  one frame there and then stops;
* window k starts at keystroke write k, so at most one frame per typed key is
  allowed (`topic` is five keys, so five frames are the reference behaviour).

Nothing is normalized: the frames are counted in the recorded byte stream, at the
byte level, exactly as the child emitted them.
"""
import argparse
from bisect import bisect_left
import base64
import hashlib
import json
from pathlib import Path
import sys

SCENARIOS = ('picker-normal', 'picker-filter', 'picker-no-match', 'picker-delete', 'picker-empty')
DEFAULT_MARKER = b'Browse sessions'
FRAME_MARKERS = {'picker-empty': b'No sessions found.'}
CSI_FINALS = range(0x40, 0x7F)


def key_events(text):
    """Number of key presses a single scenario write delivers.

    Plain characters are one key each; a CSI/SS3 escape sequence (`\\x1b[B`,
    `\\x1bOB`, `\\x1b[1;1R`-style replies are filtered out by the caller) is one
    key. Never zero: a write that carries only an escape still delivers a key.
    """
    data = text.encode() if isinstance(text, str) else text
    count, i = 0, 0
    while i < len(data):
        if data[i] == 0x1B:
            count += 1
            if i + 1 < len(data) and data[i + 1] == ord('['):
                i += 2
                while i < len(data) and data[i] not in CSI_FINALS:
                    i += 1
                i += 1
            elif i + 1 < len(data) and data[i + 1] == ord('O'):
                i += 3
            else:
                i += 1
        else:
            count += 1
            i += 1
    return max(count, 1)


def raw_bytes(record):
    raw = base64.b64decode(record['raw_base64'], validate=True)
    if hashlib.sha256(raw).hexdigest() != record['raw_sha256']:
        raise RuntimeError('Corrupt raw capture: the recorded hash does not match')
    events = b''.join(base64.b64decode(entry[1], validate=True)
                      for entry in record['events_base64'])
    if events != raw:
        raise RuntimeError('Corrupt raw capture: events do not rebuild the byte stream')
    return raw, [(entry[0], base64.b64decode(entry[1], validate=True))
                 for entry in record['events_base64']]


def scenario_keys(record):
    """(time, text) of the harness keystrokes; terminal replies are not input."""
    return [(entry[0], base64.b64decode(entry[1]).decode())
            for entry in (record.get('inputs_base64') or [])
            if len(entry) > 2 and str(entry[2]).startswith('scenario')]


def windows(record):
    """Frame allowance and emitted bytes for every input window."""
    _, events = raw_bytes(record)
    keys = scenario_keys(record)
    boundaries = [time for time, _ in keys]
    allowances = [1] + [key_events(text) for _, text in keys]
    chunks = [bytearray() for _ in allowances]
    for time, data in events:
        # An event stamped exactly at a keystroke is the frame that keystroke
        # produced, so it belongs to that window.
        chunks[bisect_left(boundaries, time)].extend(data)
    return list(zip(allowances, [bytes(chunk) for chunk in chunks])), keys


def case_problems(record, scenario):
    marker = FRAME_MARKERS.get(scenario, DEFAULT_MARKER)
    problems = []
    spans, keys = windows(record)
    total = sum(chunk.count(marker) for _, chunk in spans)
    if total == 0:
        problems.append(f'the picker never drew {marker.decode()!r}')
        return problems
    for index, (allowance, chunk) in enumerate(spans):
        drawn = chunk.count(marker)
        if drawn > allowance:
            where = ('while waiting for input, before any keystroke' if index == 0
                     else f'after the keystroke write {index} ({keys[index - 1][1]!r})')
            problems.append(f'{where}: drew {drawn} frame(s) but at most {allowance} '
                            f'is the reference behaviour')
    return problems


def check(bundle, side):
    cases = [c for c in bundle['cases'] if c['scenario'] in SCENARIOS]
    expected = {f'{name}-{width}x30' for name in SCENARIOS for width in (100, 80)}
    if len(cases) != len(expected) or {c['id'] for c in cases} != expected:
        raise RuntimeError('Incomplete or duplicate redraw-on-input matrix')
    failures = []
    for case in cases:
        record = case[side]
        if case['id'] != f'{case["scenario"]}-{record["width"]}x{record["height"]}':
            raise RuntimeError('Terminal dimensions do not match case id')
        problems = case_problems(record, case['scenario'])
        passed = not problems
        detail = 'the picker drew once per input window' if passed else '; '.join(problems)
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
