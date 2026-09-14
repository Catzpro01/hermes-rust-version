"""Unit tests for the Spec 017 W2 browse-control checker.

The checker judges recorded PTY streams against the pinned `_curses_browse`
contracts (resize / long list / clear filter). These tests feed it synthetic
streams built from the same escape sequences the picker emits — no binary, no
PTY — so the checker's own logic is verified before the live gate runs in CI.
"""
import base64
import hashlib
import unittest

from check_picker_browse_control import case_problems


def paint(rows_map, cols=100, rows=30):
    parts = ['\x1b[2J\x1b[1;1H']
    for row in sorted(rows_map):
        parts.append(f'\x1b[{row + 1};1H{rows_map[row][:cols - 1]}')
    return ''.join(parts).encode()


HINT = '  Browse sessions — ↑↓ navigate  Enter select  Type to filter  Esc quit'
FILTER_HINT = '  Browse sessions — filter: sec█'
COL = '   Title / Preview  Stat   Msgs  Active      Src   ID'


def session_row(name, selected=False):
    arrow = ' → ' if selected else '   '
    return f'{arrow}{name:<38}  done      1  just now   cli   deadbeef'


def long_frame(cursor, total=30, start=0, visible=26):
    rows = {0: HINT, 1: COL, 2: ''}
    for i in range(visible):
        index = start + i
        rows[3 + i] = session_row(f'session-{index:02d}', selected=index == cursor)
    rows[29] = f'  {cursor + 1}/{total} sessions   d delete'
    return paint(rows)


def two_session_frame(cursor, filtered=False, hint=HINT):
    names = ['deploy the thing', 'second topic']
    rows = {0: hint, 1: COL, 2: ''}
    for i, name in enumerate(names):
        rows[3 + i] = session_row(name, selected=i == cursor)
    if filtered:
        footer = '  1/2 sessions (filtered from 2)'
    else:
        # The reference footer counter is cursor+1/total, not the list size.
        footer = f'  {cursor + 1}/2 sessions   d delete'
    rows[29] = footer
    return paint(rows)


def make_record(width, height, windows, inputs):
    """windows: byte chunk per input window (len = len(inputs) + 1); inputs:
    (key, kind) pairs interleaved before window k+1."""
    raw = b''.join(windows)
    events, input_entries = [], []
    events.append([0.1, base64.b64encode(windows[0]).decode()])
    for k, (key, kind) in enumerate(inputs):
        input_entries.append([1.0 + 2 * k, base64.b64encode(key.encode()).decode(), kind])
        events.append([1.5 + 2 * k, base64.b64encode(windows[k + 1]).decode()])
    return {'width': width, 'height': height,
            'raw_base64': base64.b64encode(raw).decode(),
            'raw_sha256': hashlib.sha256(raw).hexdigest(),
            'events_base64': events, 'inputs_base64': input_entries,
            'error': None}


LONG_INPUTS = [('\x1b[B' * 26, 'scenario key'), ('\x1b[B' * 3, 'scenario key'),
               ('\x1b[B', 'scenario key'), ('\x1b[A', 'scenario key'),
               ('\x1b', 'scenario key')]


def long_record_pass():
    windows = [long_frame(0), long_frame(26, start=1), long_frame(29, start=4),
               long_frame(0, start=0), long_frame(29, start=4), b'']
    return make_record(100, 30, windows, LONG_INPUTS)


class BrowseControlCheckerTests(unittest.TestCase):
    def test_long_list_reference_behaviour_passes(self):
        self.assertEqual(case_problems(long_record_pass(), 'picker-long-list'), [])

    def test_long_list_without_wrap_down_fails(self):
        record = long_record_pass()
        # The wrap-down window keeps the footer at 30/30: no modulo cursor.
        windows = [long_frame(0), long_frame(26, start=1), long_frame(29, start=4),
                   long_frame(29, start=4), long_frame(29, start=4), b'']
        record = make_record(100, 30, windows, LONG_INPUTS)
        problems = case_problems(record, 'picker-long-list')
        self.assertTrue(any('1/30 sessions' in p for p in problems), problems)

    def test_long_list_wrap_without_window_jump_fails(self):
        windows = [long_frame(0), long_frame(26, start=1), long_frame(29, start=4),
                   long_frame(0, start=1), long_frame(29, start=4), b'']
        record = make_record(100, 30, windows, LONG_INPUTS)
        problems = case_problems(record, 'picker-long-list')
        self.assertTrue(any('first' in p and 'top row' in p for p in problems), problems)

    def test_long_list_wrap_up_keeps_wrong_window_fails(self):
        windows = [long_frame(0), long_frame(26, start=1), long_frame(29, start=4),
                   long_frame(0, start=0), long_frame(29, start=1), b'']
        record = make_record(100, 30, windows, LONG_INPUTS)
        problems = case_problems(record, 'picker-long-list')
        self.assertTrue(any('bottom-clamped' in p for p in problems), problems)

    def test_resize_too_small_reference_behaviour_passes(self):
        windows = [two_session_frame(0),
                   paint({0: 'Terminal too small'}, cols=60, rows=4), b'']
        inputs = [('@resize 60x4', 'scenario resize 60x4'), ('\r', 'scenario key')]
        record = make_record(100, 30, windows, inputs)
        self.assertEqual(case_problems(record, 'picker-resize-too-small'), [])

    def test_resize_too_small_without_notice_fails(self):
        windows = [two_session_frame(0), two_session_frame(0), b'']
        inputs = [('@resize 60x4', 'scenario resize 60x4'), ('\r', 'scenario key')]
        record = make_record(100, 30, windows, inputs)
        problems = case_problems(record, 'picker-resize-too-small')
        self.assertTrue(any('Terminal too small' in p for p in problems), problems)

    def test_resize_too_small_requires_sub_minimum_target(self):
        windows = [two_session_frame(0),
                   paint({0: 'Terminal too small'}, cols=80, rows=20), b'']
        inputs = [('@resize 80x20', 'scenario resize 80x20'), ('\r', 'scenario key')]
        record = make_record(100, 30, windows, inputs)
        problems = case_problems(record, 'picker-resize-too-small')
        self.assertTrue(any('not below the reference minimum' in p for p in problems),
                        problems)

    def test_resize_redraw_reference_behaviour_passes(self):
        redraw = paint({0: HINT, 1: COL, 2: '', 3: session_row('deploy the thing', True),
                        4: session_row('second topic'), 19: '  2/2 sessions   d delete'},
                       cols=80, rows=20)
        windows = [two_session_frame(0), redraw, b'']
        inputs = [('@resize 80x20', 'scenario resize 80x20'), ('\x1b', 'scenario key')]
        record = make_record(100, 30, windows, inputs)
        self.assertEqual(case_problems(record, 'picker-resize-redraw'), [])

    def test_resize_redraw_with_old_geometry_footer_fails(self):
        # Same frame, but the footer move still targets the pre-resize row 30.
        redraw = paint({0: HINT, 1: COL, 2: '', 3: session_row('deploy the thing', True),
                        4: session_row('second topic'), 29: '  2/2 sessions   d delete'},
                       cols=80, rows=20)
        windows = [two_session_frame(0), redraw, b'']
        inputs = [('@resize 80x20', 'scenario resize 80x20'), ('\x1b', 'scenario key')]
        record = make_record(100, 30, windows, inputs)
        problems = case_problems(record, 'picker-resize-redraw')
        self.assertTrue(any('old geometry' in p for p in problems), problems)

    CLEAR_INPUTS = [('sec', 'scenario key'), ('\x1b', 'scenario key'),
                    ('\x1b', 'scenario key')]

    def test_clear_filter_esc_reference_behaviour_passes(self):
        windows = [two_session_frame(0), two_session_frame(1, filtered=True,
                                                           hint=FILTER_HINT),
                   two_session_frame(0), b'']
        record = make_record(100, 30, windows, self.CLEAR_INPUTS)
        self.assertEqual(case_problems(record, 'picker-clear-filter-esc'), [])

    def test_clear_filter_esc_that_keeps_filter_fails(self):
        windows = [two_session_frame(0), two_session_frame(1, filtered=True,
                                                           hint=FILTER_HINT),
                   two_session_frame(1, filtered=True, hint=FILTER_HINT), b'']
        record = make_record(100, 30, windows, self.CLEAR_INPUTS)
        problems = case_problems(record, 'picker-clear-filter-esc')
        self.assertTrue(any('normal' in p and 'hint' in p for p in problems), problems)

    def test_clear_filter_backspace_reference_behaviour_passes(self):
        inputs = [('sec', 'scenario key'), ('\x7f\x7f\x7f', 'scenario key'),
                  ('\x1b', 'scenario key')]
        windows = [two_session_frame(0), two_session_frame(1, filtered=True,
                                                           hint=FILTER_HINT),
                   two_session_frame(0), b'']
        record = make_record(100, 30, windows, inputs)
        self.assertEqual(case_problems(record, 'picker-clear-filter-backspace'), [])

    def test_clear_filter_backspace_keeping_filtered_footer_fails(self):
        inputs = [('sec', 'scenario key'), ('\x7f\x7f\x7f', 'scenario key'),
                  ('\x1b', 'scenario key')]
        cleared = two_session_frame(0)
        # Footer still claims a filter although the filter text is gone.
        windows = [two_session_frame(0), two_session_frame(1, filtered=True,
                                                           hint=FILTER_HINT),
                   cleared.replace(b'1/2 sessions   d delete',
                                   b'1/2 sessions (filtered from 2)'), b'']
        record = make_record(100, 30, windows, inputs)
        problems = case_problems(record, 'picker-clear-filter-backspace')
        self.assertTrue(any('still reports a filter' in p for p in problems), problems)

    def test_capture_error_is_surfaced_not_masked(self):
        record = make_record(100, 30, [b''], [])
        record['error'] = 'missing readiness at stage 2: 2/2 sessions'
        problems = case_problems(record, 'picker-clear-filter-esc')
        self.assertEqual(problems, ['capture error: missing readiness at stage 2: 2/2 sessions'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
