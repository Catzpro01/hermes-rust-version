"""Supporting evidence/decoder checks; not a substitute for actual CLI RED."""
import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import unittest

from check_picker_footer_position import screen_for
from check_picker_selection import (MARKER, SCENARIOS, SELECTED_STYLES, case_problems,
                                    check, cursor_rows)

ROOT = Path(__file__).resolve().parents[1]
BODY = ' → second topic'


def record(body, width=100, height=30):
    raw = body.encode()
    encoded = base64.b64encode(raw).decode()
    return {'width': width, 'height': height, 'error': None,
            'snapshot_end_byte': len(raw), 'raw_base64': encoded,
            'raw_sha256': hashlib.sha256(raw).hexdigest(), 'events_base64': [[0, encoded]]}


def row4(sgr, text=BODY):
    return f'\x1b[4;1H\x1b[{sgr}m{text}'


class SelectionChecks(unittest.TestCase):
    def setUp(self):
        self.bundle = json.loads((ROOT / 'docs/hermes-ui-spec/017/evidence/ui-3b39bd7/python-bundle.json').read_text())

    def test_reference_records_pass(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check(self.bundle, 'python'), [])

    def test_cursor_row_is_found_by_the_marker(self):
        self.assertEqual(cursor_rows(record(row4('1;32'))), [3])
        self.assertEqual(cursor_rows(record('\x1b[5;1H\x1b[1;32m → other')), [4])

    def test_palette_pair_matches_the_shipped_precedent(self):
        self.assertEqual(SELECTED_STYLES, {('green', True), ('00cd00', True)})
        for sgr, expected in [('1;32', ('green', True)), ('1;38;5;2', ('00cd00', True))]:
            with self.subTest(sgr=sgr):
                screen = screen_for(record(row4(sgr)))
                cell = screen.buffer[3][2]
                self.assertEqual((cell.fg, cell.bold, cell.reverse), expected + (False,))

    def test_bright_green_or_unbold_selection_is_reported(self):
        for sgr, label in [('1;38;5;10', 'bright green'), ('38;5;2', 'not bold')]:
            with self.subTest(label=label):
                problems = case_problems(record(row4(sgr)), 'picker-normal')
                self.assertTrue(any('cursor row style' in p for p in problems), problems)

    def test_reverse_video_is_reported(self):
        problems = case_problems(record(row4('7')), 'picker-normal')
        self.assertTrue(any('reverse video still set' in p for p in problems), problems)

    def test_missing_marker_is_reported(self):
        problems = case_problems(record(f'\x1b[4;1H\x1b[1;32m{":"} second topic'), 'picker-normal')
        self.assertTrue(any('cursor marker rows' in p for p in problems), problems)

    def test_leaked_styling_on_the_row_below_is_reported(self):
        for leak, message in [('\x1b[1;32m', 'cursor-row green leaked'),
                              ('\x1b[7m', 'reverse video leaked'),
                              ('\x1b[1m', 'bold leaked')]:
            with self.subTest(message=message):
                problems = case_problems(
                    record(row4('1;32') + f'\x1b[5;1H{leak}   intr'), 'picker-normal')
                self.assertIn(message + ' to unselected row 5', problems)

    def test_unselected_row_own_ink_is_not_asserted(self):
        # The reference colours the status token brown; that is not this gate.
        problems = case_problems(record(row4('1;38;5;2') + '\x1b[0m\x1b[5;1H\x1b[33m   intr'),
                                 'picker-normal')
        self.assertEqual(problems, [])

    def test_missing_matrix_is_error(self):
        cases = [c for c in self.bundle['cases'] if c['scenario'] in SCENARIOS]
        with self.assertRaisesRegex(RuntimeError, 'selected-row matrix'):
            check({'cases': cases[:4]}, 'python')

    def test_corrupt_trace_is_error(self):
        case = next(c for c in self.bundle['cases'] if c['scenario'] == 'picker-normal')
        broken = {**case['python'], 'raw_sha256': '0' * 64}
        with self.assertRaisesRegex(RuntimeError, 'Corrupt raw capture'):
            case_problems(broken, 'picker-normal')

    def test_marker_constant_matches_the_reference_frame(self):
        self.assertEqual(MARKER, ' → ')


if __name__ == '__main__':
    unittest.main()
