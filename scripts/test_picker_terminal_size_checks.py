"""Supporting evidence/decoder checks; not a substitute for actual CLI RED."""
import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import unittest

from check_picker_footer_position import screen_for
from check_picker_terminal_size import NARROW, TOO_SMALL, check, size_problems


def record(body, width, exited=False):
    raw = body.encode()
    encoded = base64.b64encode(raw).decode()
    return {'width': width, 'height': 30, 'error': None, 'snapshot_end_byte': len(raw),
            'raw_base64': encoded, 'raw_sha256': hashlib.sha256(raw).hexdigest(),
            'events_base64': [[0, encoded]], 'exit_code_at_snapshot': 0 if exited else None}


FRAME = ('\x1b[1;1H  Browse sessions — ↑↓ navigate  Enter select  Type to filter  Esc quit'
         '\x1b[2;1H   Title / Preview                            Stat    Msgs  Active')
NOTICE = '\x1b[1;1HTerminal too small'


class TerminalSizeChecks(unittest.TestCase):
    def test_notice_screen_passes(self):
        self.assertEqual(size_problems(record(NOTICE, TOO_SMALL), TOO_SMALL), [])

    def test_notice_with_other_output_is_reported(self):
        problems = size_problems(record(NOTICE + '\x1b[2;1H   deploy  done', TOO_SMALL), TOO_SMALL)
        self.assertTrue(any('only thing drawn' in p for p in problems), problems)

    def test_notice_off_column_one_is_reported(self):
        problems = size_problems(record('\x1b[1;5HTerminal too small', TOO_SMALL), TOO_SMALL)
        self.assertTrue(any('column 1' in p for p in problems), problems)

    def test_returning_before_a_key_is_reported(self):
        problems = size_problems(record(NOTICE, TOO_SMALL, exited=True), TOO_SMALL)
        self.assertTrue(any('returned before any key' in p for p in problems), problems)

    def test_narrow_screen_draws_the_frame(self):
        self.assertEqual(size_problems(record(FRAME, NARROW), NARROW), [])

    def test_refusing_a_usable_terminal_is_reported(self):
        problems = size_problems(record(NOTICE, NARROW), NARROW)
        self.assertTrue(any('must draw the picker frame' in p for p in problems), problems)
        self.assertTrue(any('must not show' in p for p in problems), problems)

    def test_capture_error_is_reported(self):
        broken = record(NOTICE, TOO_SMALL)
        broken.update({'error': 'process exit 1 at stage 0'})
        problems = size_problems(broken, TOO_SMALL)
        self.assertTrue(any('capture error' in p for p in problems), problems)

    def test_width_mismatch_is_error(self):
        with self.assertRaisesRegex(RuntimeError, 'width'):
            size_problems(record(NOTICE, TOO_SMALL), NARROW)

    def test_missing_matrix_is_error(self):
        with self.assertRaisesRegex(RuntimeError, 'size-contract matrix'):
            check({'cases': []}, 'rust')

    def test_full_matrix_fails_a_repainting_capture(self):
        bundle = {'cases': [
            {'id': f'picker-narrow-{NARROW}x30', 'scenario': 'picker-narrow', 'fixture': {},
             'rust': record(NOTICE, NARROW)},
            {'id': f'picker-too-small-{TOO_SMALL}x30', 'scenario': 'picker-too-small', 'fixture': {},
             'rust': record(NOTICE, TOO_SMALL, exited=True)}]}
        with contextlib.redirect_stdout(io.StringIO()):
            failures = check(bundle, 'rust')
        self.assertEqual(failures, [f'picker-narrow-{NARROW}x30', f'picker-too-small-{TOO_SMALL}x30'])

    def test_screen_lookup_reads_row_one(self):
        self.assertIn('Terminal too small', screen_for(record(NOTICE, TOO_SMALL)).display[0])


if __name__ == '__main__':
    unittest.main()
