"""Supporting evidence/decoder checks; not a substitute for actual CLI RED."""
import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import unittest

from check_picker_column_layout import (HEADER_INK, body_columns, body_field, case_problems,
                                        check, header_columns, header_field, header_style)
from check_picker_footer_position import screen_for

ROOT = Path(__file__).resolve().parents[1]


def record(line, sgr, width=100, height=30):
    raw = f'\x1b[2;1H\x1b[{sgr}m{line}'.encode()
    encoded = base64.b64encode(raw).decode()
    return {'width': width, 'height': height, 'error': None,
            'snapshot_end_byte': len(raw), 'raw_base64': encoded,
            'raw_sha256': hashlib.sha256(raw).hexdigest(), 'events_base64': [[0, encoded]]}


def header_line(indent=3, field=41, bold=False):
    star = '\x1b[1m' if bold else ''
    return star + ' ' * indent + 'Title / Preview'.ljust(field) + '  Stat    Msgs  Active      Src   ID'


class ColumnLayoutChecks(unittest.TestCase):
    def setUp(self):
        self.bundle = json.loads((ROOT / 'docs/hermes-ui-spec/017/evidence/ui-3b39bd7/python-bundle.json').read_text())

    def test_reference_records_pass(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check(self.bundle, 'python'), [])

    def test_header_ink_accepts_both_palette8_spellings(self):
        for sgr, expected in [('90', ('brightblack', False)), ('38;5;8', ('7f7f7f', False))]:
            with self.subTest(sgr=sgr):
                record_ = record(header_line(), sgr)
                line = screen_for(record_).display[1].rstrip()
                self.assertEqual(header_style(screen_for(record_), line), {expected})
                self.assertIn(expected, HEADER_INK)

    def test_bold_header_is_rejected(self):
        record_ = record(header_line(bold=True), '38;5;8')
        style = header_style(screen_for(record_), header_line(bold=True).replace('\x1b[1m', ''))
        self.assertEqual(style, {('7f7f7f', True)})

    def test_two_space_indent_is_rejected(self):
        problems = case_problems(record(header_line(indent=2, field=48), '38;5;8'), 'picker-normal')
        self.assertTrue(any('three-cell indent' in p for p in problems), problems)

    def test_field_derivation_matches_the_two_pinned_widths(self):
        self.assertEqual((header_field(100), body_field(100)), (41, 38))
        self.assertEqual((header_field(80), body_field(80)), (21, 20))
        self.assertEqual(header_columns(100)['Stat'], 46)
        self.assertEqual(header_columns(80)['Stat'], 26)
        self.assertEqual(body_columns(100)['status'], 43)
        self.assertEqual(body_columns(80)['status'], 25)

    def test_missing_matrix_is_error(self):
        self.bundle['cases'] = [c for c in self.bundle['cases'] if c['scenario'] != 'picker-no-match']
        with self.assertRaisesRegex(RuntimeError, 'four column-layout matrix'):
            check(self.bundle, 'python')

    def test_corrupt_trace_is_error(self):
        record_ = [c for c in self.bundle['cases'] if c['scenario'] == 'picker-normal'][0]['python']
        record_ = {**record_, 'raw_sha256': '0' * 64}
        with self.assertRaisesRegex(RuntimeError, 'Corrupt raw'):
            case_problems(record_, 'picker-normal')


if __name__ == '__main__':
    unittest.main()
