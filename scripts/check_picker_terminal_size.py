"""Observe the picker's terminal-size contract at the threshold.

The pinned upstream source (`_session_browse_picker`, commit `63279301`, file
kept at `docs/hermes-ui-spec/017/evidence/upstream-status-attr/`) fixes it:

    max_y, max_x = stdscr.getmaxyx()
    if max_y < 5 or max_x < 40:
        stdscr.addstr(0, 0, "Terminal too small")
        stdscr.refresh()
        stdscr.getch()      # waits for one key
        return

so 40 columns is the narrowest terminal that draws the picker, 39 columns shows
the five-word notice at row 1 *inside* the alternate screen, and the picker has
not returned before that key arrives.

Both properties are observable in the retained records: the screen text, and
`exit_code_at_snapshot` (which stays `None` while the child waits for the key).
Nothing is normalized.
"""
import argparse
import json
from pathlib import Path
import sys

from check_picker_footer_position import screen_for

NARROW = 40
TOO_SMALL = 39
FRAME_MARKERS = ('Browse sessions', 'Title / Preview')
NOTICE = 'Terminal too small'


def size_problems(record, width):
    problems = []
    if record['width'] != width:
        raise RuntimeError('Recorded width does not match the case id')
    if record['error']:
        problems.append(f'capture error: {record["error"]}')
        return problems
    screen = screen_for(record)
    display = [line.rstrip() for line in screen.display]
    if width < NARROW:
        first = display[0].strip()
        if first != NOTICE:
            problems.append(f'row 1 shows {display[0]!r}, expected the {NOTICE!r} notice')
        if any(line.strip() for line in display[1:]):
            drawn = [f'row {i + 1}: {line!r}' for i, line in enumerate(display[1:], start=1)
                     if line.strip()]
            problems.append(f'the notice must be the only thing drawn, found {drawn}')
        if display[0] != NOTICE:
            problems.append(f'the notice must start at column 1, found {display[0]!r}')
        if record['exit_code_at_snapshot'] is not None:
            problems.append('the picker returned before any key was pressed; the reference '
                            'waits in getch()')
    else:
        text = '\n'.join(display)
        for marker in FRAME_MARKERS:
            if marker not in text:
                problems.append(f'{width} columns must draw the picker frame; {marker!r} is missing')
        if NOTICE in text:
            problems.append(f'{width} columns is usable and must not show the {NOTICE!r} notice')
        # The reference draws with `addnstr(..., max_x - 1, ...)`, so a line is
        # clipped, never wrapped: the frame keeps its rows even when the fixed
        # columns are wider than the terminal.
        for row, expected in ((1, 'Browse sessions'), (2, 'Title / Preview'),
                              (record['height'], 'sessions')):
            line = display[row - 1]
            if expected not in line:
                problems.append(f'row {row} must hold the {expected!r} part of the frame, '
                                f'found {line!r} (lines are clipped, never wrapped)')
    return problems


def check(bundle, side):
    expected = {f'picker-narrow-{NARROW}x30', f'picker-too-small-{TOO_SMALL}x30'}
    cases = [c for c in bundle['cases'] if c['id'] in expected]
    if {c['id'] for c in cases} != expected:
        raise RuntimeError('Incomplete or duplicate size-contract matrix')
    failures = []
    for case in cases:
        record = case[side]
        problems = size_problems(record, record['width'])
        passed = not problems
        detail = 'size contract matches the pinned threshold' if passed else '; '.join(problems)
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
