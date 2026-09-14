"""Supporting evidence/decoder checks; not a substitute for actual CLI RED."""
import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import unittest

from check_picker_message_style import (DELETE_PROMPT, NO_MATCH, PROMPT_INKS, case_problems,
                                        check, dim_windows, printable)
from check_picker_footer_position import screen_for

ROOT = Path(__file__).resolve().parents[1]
MESSAGE_RECORD = f'\x1b[4;1H\x1b[0;2m{NO_MATCH}'


def record(body, width=100, height=30):
    raw = body.encode()
    encoded = base64.b64encode(raw).decode()
    return {'width': width, 'height': height, 'error': None,
            'snapshot_end_byte': len(raw), 'raw_base64': encoded,
            'raw_sha256': hashlib.sha256(raw).hexdigest(), 'events_base64': [[0, encoded]]}


def bundle_cases(bundle, names):
    return [dict(case) for case in bundle['cases'] if case['scenario'] in names]


class MessageStyleChecks(unittest.TestCase):
    def setUp(self):
        self.bundle = json.loads((ROOT / 'docs/hermes-ui-spec/017/evidence/ui-3b39bd7/python-bundle.json').read_text())

    def test_reference_records_pass(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check(self.bundle, 'python'), [])

    def test_dim_windows_read_the_retained_stream(self):
        windows = dim_windows(record(MESSAGE_RECORD + '\x1b[0m'))
        self.assertEqual(len(windows), 1)
        self.assertEqual(windows[0]['text'], NO_MATCH)
        self.assertEqual((windows[0]['on']['row'], windows[0]['on']['col']), (4, 1))
        self.assertEqual(windows[0]['on']['sgr'], '\x1b[0;2m')
        self.assertEqual(dim_windows(record(f'\x1b[4;1H\x1b[0m{NO_MATCH}')), [])

    def test_python_carries_dim_to_the_footer_reset(self):
        # The reference does not reset dim before the frame ends; the rendered
        # message row is what the packet compares pixel by pixel.
        windows = dim_windows(record(MESSAGE_RECORD + '\x1b[30;1H\x1b[0m'))
        self.assertEqual([w['text'] for w in windows], [NO_MATCH])

    def test_normal_intensity_code_clears_dim(self):
        self.assertEqual(dim_windows(record(MESSAGE_RECORD.replace('0;2', '0;2;22'))), [])

    def test_message_without_dim_is_reported(self):
        problems = case_problems(record(f'\x1b[4;1H{NO_MATCH}'), 'picker-no-match')
        self.assertTrue(any('dim attribute' in p for p in problems), problems)

    def test_dim_window_that_does_not_cover_the_message_is_reported(self):
        problems = case_problems(record(f'\x1b[4;1H{NO_MATCH}\x1b[4;1H\x1b[2m  other text\x1b[0m'),
                                 'picker-no-match')
        self.assertTrue(any('first dim window covers' in p for p in problems), problems)

    def test_dim_outside_row_four_is_reported(self):
        problems = case_problems(record(f'\x1b[4;1H{NO_MATCH}\x1b[10;1H\x1b[2m{NO_MATCH}\x1b[0m'),
                                 'picker-no-match')
        self.assertTrue(any(p.startswith('dim turns on at row 10') for p in problems), problems)

    def test_bold_message_is_reported(self):
        problems = case_problems(record(f'\x1b[4;1H\x1b[1m\x1b[2m{NO_MATCH}\x1b[0m'), 'picker-no-match')
        self.assertTrue(any('must not be bold' in p for p in problems), problems)

    def test_palette_pair_matches_the_shipped_precedent(self):
        self.assertEqual(PROMPT_INKS, {'red', 'cd0000'})
        for sgr, expected in [('1;31', ('red', True)), ('1;38;5;1', ('cd0000', True))]:
            with self.subTest(sgr=sgr):
                screen = screen_for(record(f'\x1b[30;1H\x1b[{sgr}m{DELETE_PROMPT}'))
                cell = screen.buffer[29][0]
                self.assertEqual((cell.fg, cell.bold), expected)

    def test_bright_red_or_unbold_prompt_is_reported(self):
        for sgr, label in [('1;38;5;9', 'bright red'), ('38;5;1', 'not bold')]:
            with self.subTest(label=label):
                case = dict(next(c for c in self.bundle['cases'] if c['id'] == 'picker-delete-100x30'))
                case['python'] = {**case['python'],
                                  **record(f'\x1b[30;1H\x1b[{sgr}m{DELETE_PROMPT}')}
                problems = case_problems(case['python'], 'picker-delete')
                self.assertTrue(any('delete prompt style' in p for p in problems), problems)

    def test_dim_on_a_control_screen_is_reported(self):
        cases = bundle_cases(self.bundle, ('picker-normal', 'picker-delete', 'picker-no-match'))
        normal = next(c for c in cases if c['id'] == 'picker-normal-100x30')
        normal['python'] = {**normal['python'], **record(MESSAGE_RECORD + '\x1b[0m')}
        with contextlib.redirect_stdout(io.StringIO()):
            failures = check({'cases': cases}, 'python')
        self.assertEqual(failures, ['picker-normal-100x30'])

    def test_missing_matrix_is_error(self):
        cases = bundle_cases(self.bundle, ('picker-delete', 'picker-no-match'))
        with self.assertRaisesRegex(RuntimeError, 'six-message matrix'):
            check({'cases': cases}, 'python')

    def test_corrupt_trace_is_error(self):
        case = next(c for c in self.bundle['cases'] if c['scenario'] == 'picker-delete')
        broken = {**case['python'], 'raw_sha256': '0' * 64}
        with self.assertRaisesRegex(RuntimeError, 'Corrupt raw capture'):
            case_problems(broken, 'picker-delete')

    def test_printable_drops_escapes_and_controls(self):
        self.assertEqual(printable(b'\x1b[2m  abc\x1b[0m\r\n'), '  abc')


if __name__ == '__main__':
    unittest.main()
