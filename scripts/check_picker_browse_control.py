"""Spec 017 W2 browse-control gate: resize, long lists, clear filter.

Reference contract (pinned upstream `_curses_browse`; documented in
`docs/hermes-ui-spec/017/evidence/upstream-browse-control/`):

* resize — the next frame repaints with the new geometry, state survives, the
  scroll window clamps minimally; a terminal that became smaller than 5 rows
  or 40 columns shows `Terminal too small` and exits at the next key;
* long list — the cursor wraps around (modulo) at both ends, the scroll window
  moves only enough to keep the cursor visible, and wrapping down jumps the
  window back to the top item;
* clear filter — Esc while a filter is active, or Backspace until the filter is
  empty, restores the full list with cursor/offset at zero and the normal hint.

Nothing is normalised: the recorded byte stream is replayed through pyte per
input window (a keystroke write or a `@resize` event), and each window's final
screen must satisfy its contract. The footer placement check is byte-level —
the reference repaints the footer with an explicit cursor move to the bottom
row, so a redraw that kept the old geometry positions its footer on the old
bottom row and fails even though the text is identical.
"""
import argparse
import base64
from bisect import bisect_left
import hashlib
import json
from pathlib import Path
import re
import sys

import pyte

SCENARIOS = ('picker-resize-too-small', 'picker-resize-redraw',
             'picker-long-list', 'picker-clear-filter-esc',
             'picker-clear-filter-backspace')
NORMAL_HINT_MARK = '↑↓ navigate'
TOO_SMALL = 'Terminal too small'


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


def scenario_inputs(record):
    """(time, key, kind) of every harness input: scenario keys and resizes."""
    return [(entry[0], base64.b64decode(entry[1]).decode(), str(entry[2]))
            for entry in (record.get('inputs_base64') or [])
            if len(entry) > 2 and str(entry[2]).startswith('scenario')]


def replay(record):
    """Split the stream at every harness input and replay each window.

    Returns (inputs, screens): `screens[i]` is pyte's display after the bytes
    emitted in window i — window 0 precedes the first input, window k follows
    input k-1. The screen is resized at every `@resize` input before its
    window is fed, exactly like the real terminal."""
    _, events = raw_bytes(record)
    ins = scenario_inputs(record)
    times = [time for time, _, _ in ins]
    chunks = [bytearray() for _ in range(len(ins) + 1)]
    for time, data in events:
        # Bytes stamped exactly at an input belong to the window that input
        # opens (they are the response to it).
        chunks[bisect_left(times, time)].extend(data)
    cols, rows = record['width'], record['height']
    screen = pyte.Screen(cols, rows)
    stream = pyte.ByteStream(screen)
    screens = []
    for index, chunk in enumerate(chunks):
        if index > 0:
            _, key, kind = ins[index - 1]
            if kind.startswith('scenario resize'):
                new_cols, new_rows = (int(part) for part
                                      in key[len('@resize '):].split('x'))
                cols, rows = new_cols, new_rows
                screen.resize(rows, cols)
        stream.feed(bytes(chunk))
        screens.append(list(screen.display))
    return ins, screens, chunks


def body_rows(screen):
    """Session rows: below the blank separator (index 2), above the footer."""
    return screen[3:-1]


def observed_names(screen):
    """Session names of the visible window, in display order."""
    names = []
    for line in body_rows(screen):
        text = line.strip().lstrip('→').strip()
        if text:
            names.append(text)
    return names


def footer_of(screen):
    return screen[-1]


def check_resize_too_small(record, ins, screens, chunks):
    problems = []
    if 'Browse sessions' not in screens[0][0]:
        problems.append('the initial frame is not the browse picker')
        return problems
    resize_at = next((i for i, (_, _, kind) in enumerate(ins)
                      if kind.startswith('scenario resize')), None)
    if resize_at is None:
        return problems + ['the scenario never resized the terminal']
    _, key, _ = ins[resize_at]
    new_cols, new_rows = (int(part) for part in key[len('@resize '):].split('x'))
    if new_rows >= 5 and new_cols >= 40:
        return problems + [f'the scenario resized to {new_cols}x{new_rows}, which '
                           'is not below the reference minimum of 5 rows / 40 columns']
    after = screens[resize_at + 1]
    if not any(TOO_SMALL in line for line in after):
        problems.append(f'after the resize to {new_cols}x{new_rows} the picker never '
                        f'showed {TOO_SMALL!r} — the reference refuses terminals below '
                        '5 rows / 40 columns and exits at the next key')
    return problems


def check_resize_redraw(record, ins, screens, chunks):
    problems = []
    if 'Browse sessions' not in screens[0][0]:
        problems.append('the initial frame is not the browse picker')
        return problems
    resize_at = next((i for i, (_, _, kind) in enumerate(ins)
                      if kind.startswith('scenario resize')), None)
    if resize_at is None:
        return problems + ['the scenario never resized the terminal']
    _, key, _ = ins[resize_at]
    new_cols, new_rows = (int(part) for part in key[len('@resize '):].split('x'))
    chunk = bytes(chunks[resize_at + 1])
    # The footer is repainted with an explicit move to the bottom row
    # (crossterm MoveTo is 1-based). A redraw still using the pre-resize
    # geometry positions it on the old bottom row instead.
    footer_move = f'\x1b[{new_rows};1H'.encode()
    if footer_move not in chunk:
        problems.append(f'after the resize to {new_cols}x{new_rows} no frame '
                        f'repositioned the footer on row {new_rows} — the redraw '
                        'kept the old geometry')
    after = screens[resize_at + 1]
    if 'Browse sessions' not in after[0]:
        problems.append('after the resize the hint header is missing from the top row')
    if not re.search(r'\d+/\d+ sessions', footer_of(after)):
        problems.append(f'after the resize the bottom row {footer_of(after)!r} '
                        'is not the sessions footer')
    return problems


def check_long_list(record, ins, screens, chunks):
    problems = []
    first = screens[0]
    if 'Browse sessions' not in first[0]:
        problems.append('the initial frame is not the browse picker')
        return problems
    names = observed_names(first)
    if len(names) < 4:
        return problems + [f'only {len(names)} rows visible in the initial frame; '
                           'the scenario needs a multi-window list']

    def window_with(footer_mark, after_key=None):
        for i, screen in enumerate(screens):
            if footer_mark in footer_of(screen):
                if after_key is None or (i > 0 and ins[i - 1][1] == after_key):
                    return i, screen
        return None, None

    # Window A: scrolled down so the window moved exactly one row (minimal
    # clamp — the reference only shifts the window to keep the cursor visible).
    idx, screen = window_with('27/30 sessions')
    if idx is None or idx == 0:
        problems.append('the footer never reached 27/30 sessions while moving down')
    else:
        top = body_rows(screen)[0]
        if names[1] not in top:
            problems.append(f'at 27/30 the top row should be the second session '
                            f'({names[1]!r}, minimal scroll), got {top.strip()!r}')
    # Window B: the single Down sent from the last item — the reference cursor
    # is modulo, so the window jumps back to the top item.
    wrap_down = [i for i, (_, key, kind) in enumerate(ins)
                 if kind == 'scenario key' and key == '\x1b[B']
    if not wrap_down:
        problems.append('the scenario never moved down from the last item')
    else:
        screen = screens[wrap_down[0] + 1]
        if '1/30 sessions' not in footer_of(screen):
            problems.append(f'wrapping down from the last item should land on '
                            f'1/30 sessions, footer is {footer_of(screen).strip()!r} '
                            '— the reference cursor is modulo and the window jumps '
                            'to the top')
        else:
            top = body_rows(screen)[0]
            if names[0] not in top:
                problems.append(f'after the wrap to 1/30 the top row should be the '
                                f'first session ({names[0]!r}), got {top.strip()!r}')
    # Window C: wrapped up from the first item -> last item, window at bottom.
    # The window offset is then len-rows, so the top visible row is the one at
    # that index in the initial frame (names has the first full window).
    wrap_up = [i for i, (_, key, kind) in enumerate(ins)
               if kind == 'scenario key' and key == '\x1b[A']
    if not wrap_up:
        problems.append('the scenario never moved up from the first item')
    else:
        screen = screens[wrap_up[0] + 1]
        if '30/30 sessions' not in footer_of(screen):
            problems.append(f'wrapping up from the first item should land on '
                            f'30/30 sessions, footer is {footer_of(screen).strip()!r}')
        else:
            # The window clamps to the bottom: offset = total - visible, so the
            # top visible row is the session at that index (names holds the
            # first full window of the initial frame in display order).
            offset = 30 - len(names)
            expected_top = names[offset] if 0 <= offset < len(names) else None
            top = body_rows(screen)[0]
            if expected_top and expected_top not in top:
                problems.append(f'after the wrap to 30/30 the top row should be '
                                f'{expected_top!r} (window bottom-clamped), got {top.strip()!r}')
    return problems


def check_clear_filter(record, ins, screens, chunks, cleared_key):
    problems = []
    first = screens[0]
    if 'Browse sessions' not in first[0]:
        problems.append('the initial frame is not the browse picker')
        return problems
    names = observed_names(first)
    filter_at = next((i for i, screen in enumerate(screens)
                      if 'filter: sec' in screen[0]), None)
    if filter_at is None:
        return problems + ['the filter header (filter: sec) never appeared']
    if '(filtered from' not in footer_of(screens[filter_at]):
        problems.append('while filtering, the footer must keep the unfiltered total '
                        '(... filtered from N)')
    clear_at = next((i for i, (_, key, kind) in enumerate(ins)
                     if kind == 'scenario key' and key == cleared_key), None)
    if clear_at is None:
        return problems + [f'the scenario never sent the clearing key {cleared_key!r}']
    after = screens[clear_at + 1]
    if NORMAL_HINT_MARK not in after[0]:
        problems.append(f'after clearing the filter the top row must be the normal '
                        f'hint, got {after[0].strip()!r}')
    footer = footer_of(after)
    if '1/2 sessions' not in footer:
        problems.append(f'after clearing the filter the footer must show the full '
                        f'list with the cursor back on item 1 (1/2 sessions), '
                        f'got {footer.strip()!r}')
    if '(filtered from' in footer:
        problems.append('after clearing the filter the footer still reports a filter')
    top = body_rows(after)[0]
    if names and (names[0] not in top or '→' not in top):
        problems.append(f'after clearing the filter the cursor must be back on the '
                        f'first session ({names[0]!r}), got {top.strip()!r}')
    return problems


CHECKS = {
    'picker-resize-too-small': lambda r, i, s, c: check_resize_too_small(r, i, s, c),
    'picker-resize-redraw': lambda r, i, s, c: check_resize_redraw(r, i, s, c),
    'picker-long-list': lambda r, i, s, c: check_long_list(r, i, s, c),
    'picker-clear-filter-esc':
        lambda r, i, s, c: check_clear_filter(r, i, s, c, '\x1b'),
    'picker-clear-filter-backspace':
        lambda r, i, s, c: check_clear_filter(r, i, s, c, '\x7f\x7f\x7f'),
}


def case_problems(record, scenario):
    if record.get('error'):
        return [f'capture error: {record["error"]}']
    ins, screens, chunks = replay(record)
    return CHECKS[scenario](record, ins, screens, chunks)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('recording', type=Path)
    args = parser.parse_args()
    bundle = json.loads(args.recording.read_text())
    failures = []
    for case in bundle['cases']:
        problems = case_problems(case['rust'], case['scenario'])
        status = 'PASS' if not problems else 'FAIL'
        print(f'{status} {case["id"]}: '
              + ('contract holds' if not problems else '; '.join(problems)))
        failures.extend(problems)
    sys.exit(bool(failures))
