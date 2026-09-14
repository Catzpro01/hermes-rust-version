"""Observe the delete-confirm prompt and the no-match message rows.

The retained Python reference colours the delete prompt red plus bold and writes
`  No sessions match the filter.` with the dim attribute (`ESC[0;2m` opened on
row 4 and only reset at the footer redraw). pyte 0.8.2 does not represent dim at
all, so that single attribute is read from the retained byte stream while every
other attribute is decoded by pyte; `verify-regions.cjs` proves the rendered
effect pixel by pixel on the paired packet. Nothing is normalized: palette slot 1
(`red`, SGR 31) and indexed `38;5;1` (`cd0000`) are the accepted pair, exactly
like the palette3/palette6/palette8 pairs already shipped.
"""
import argparse
import base64
import json
from pathlib import Path
import re
import sys

import pyte

from check_picker_footer_position import screen_for

NO_MATCH = '  No sessions match the filter.'
DELETE_PROMPT = "  Delete session 'second topic'? [y/N]"
PROMPT_INKS = {'red', 'cd0000'}
SGR = re.compile(rb'\x1b\[([0-9;]*)m')
ESCAPES = re.compile(rb'\x1b\[[0-9;?]*[A-Za-z]|\x1b[()][A-Z0-9]|\x1b[=>]')
CONTROLS = re.compile(rb'[\x00-\x1f\x7f]')


def printable(segment):
    return CONTROLS.sub(b'', ESCAPES.sub(b'', segment)).decode('utf-8', 'replace')


def dim_events(record):
    """Dim on/off transitions with the cursor position pyte reports there."""
    screen_for(record)
    raw = base64.b64decode(record['raw_base64'], validate=True)
    screen = pyte.Screen(record['width'], record['height'])
    stream = pyte.ByteStream(screen)
    position, dim = 0, False
    events = []
    for match in SGR.finditer(raw):
        stream.feed(raw[position:match.start()])
        state = dim
        for code in (int(code) for code in match.group(1).split(b';') if code):
            if code == 0:
                state = False
            elif code == 2:
                state = True
            elif code == 22:
                state = False
        if state != dim:
            events.append({'state': 'on' if state else 'off', 'row': screen.cursor.y + 1,
                           'col': screen.cursor.x + 1, 'start': match.start(),
                           'end': match.end(), 'sgr': match.group(0).decode('ascii')})
        dim = state
        position = match.end()
    stream.feed(raw[position:])
    return raw, events


def dim_windows(record):
    raw, events = dim_events(record)
    windows = []
    for first, second in zip(events, events[1:]):
        if first['state'] == 'on' and second['state'] == 'off':
            windows.append({'on': first, 'off': second,
                            'text': printable(raw[first['end']:second['start']])})
    return windows


def case_problems(record, scenario):
    screen = screen_for(record)
    width = record['width']
    windows = dim_windows(record)
    problems = []
    if scenario == 'picker-no-match':
        text = screen.display[3].rstrip()
        if text != NO_MATCH:
            problems.append(f'no-match message row={text!r}')
        if not windows:
            problems.append('no-match message is not written with the dim attribute')
        else:
            first = windows[0]['text'].rstrip()
            if first != NO_MATCH:
                problems.append(f'first dim window covers {first!r}, expected the message')
            for window in windows:
                if (window['on']['row'], window['on']['col']) != (4, 1):
                    problems.append('dim turns on at row {row} col {col}, expected 4/1'.format(**window['on']))
                if not window['text'].startswith(NO_MATCH):
                    problems.append('a dim window does not start with the message')
        if any(screen.buffer[3][x].bold for x in range(width)):
            problems.append('no-match message must not be bold')
        if any(screen.buffer[3][x].reverse for x in range(width)):
            problems.append('no-match message must not use reverse video')
        if any('Delete session' in line for line in screen.display):
            problems.append('delete prompt visible on a no-match screen')
    elif scenario == 'picker-delete':
        if windows:
            problems.append(f'delete prompt screen emitted {len(windows)} dim window(s)')
        row = record['height'] - 1
        text = screen.display[row].rstrip()
        if text != DELETE_PROMPT:
            problems.append(f'delete prompt row={text!r}')
        else:
            styles = {(screen.buffer[row][x].fg, screen.buffer[row][x].bold,
                       screen.buffer[row][x].reverse) for x in range(len(text))}
            wrong = {style for style in styles
                     if style[0] not in PROMPT_INKS or not style[1] or style[2]}
            if wrong:
                problems.append(f'delete prompt style {sorted(wrong)}, '
                                'expected palette1 red + bold without reverse')
    else:
        if windows:
            problems.append(f'control screen emitted {len(windows)} dim window(s)')
        row = record['height'] - 1
        text = screen.display[row].rstrip()
        if {screen.buffer[row][x].fg for x in range(len(text))} & PROMPT_INKS:
            problems.append('control footer carries the delete-prompt ink')
    return problems


def check(bundle, side):
    names = ('picker-normal', 'picker-delete', 'picker-no-match')
    cases = [c for c in bundle['cases'] if c['scenario'] in names]
    expected = {f'{n}-{w}x30' for n in names for w in (100, 80)}
    if len(cases) != 6 or {c['id'] for c in cases} != expected:
        raise RuntimeError('Incomplete or duplicate six-message matrix')
    failures = []
    for case in cases:
        record = case[side]
        if case['id'] != f'{case["scenario"]}-{record["width"]}x{record["height"]}':
            raise RuntimeError('Terminal dimensions do not match case id')
        problems = case_problems(record, case['scenario'])
        passed = not problems
        detail = 'reference styling' if passed else '; '.join(problems)
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
