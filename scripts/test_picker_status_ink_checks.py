"""Supporting evidence/decoder checks; not a substitute for actual CLI RED."""
import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import unittest

from check_picker_footer_position import screen_for
from check_picker_status_ink import (SCENARIOS, SLOT_SPELLINGS, TAG_SLOT, TAG_WIDTH,
                                     body_rows, case_problems, check, tag_column)

ROOT = Path(__file__).resolve().parents[1]


def record(body, width=100, height=30):
    raw = body.encode()
    encoded = base64.b64encode(raw).decode()
    return {'width': width, 'height': height, 'error': None,
            'snapshot_end_byte': len(raw), 'raw_base64': encoded,
            'raw_sha256': hashlib.sha256(raw).hexdigest(), 'events_base64': [[0, encoded]]}


def body_with_tag(sgr, tag='done', width=100):
    """A minimal screen whose unselected row carries a tag with the given ink."""
    name_width = max(20, width - 62)
    row = f'\x1b[5;1H   deploy{" " * (name_width - 6)}  \x1b[{sgr}m{tag:<5}\x1b[0m  1'
    return record(row, width=width)


class StatusInkChecks(unittest.TestCase):
    def setUp(self):
        self.bundle = json.loads((ROOT / 'docs/hermes-ui-spec/017/evidence/ui-3b39bd7/python-bundle.json').read_text())

    def test_reference_records_pass(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check(self.bundle, 'python'), [])

    def test_tag_column_follows_the_pinned_formula(self):
        self.assertEqual(tag_column({'width': 100}), 43)
        self.assertEqual(tag_column({'width': 80}), 25)
        self.assertEqual(TAG_WIDTH, 5)

    def test_tag_slot_mapping_matches_the_pinned_source(self):
        self.assertEqual(TAG_SLOT, {'done': 2, 'intr': 3, 'err': 1, 'empty': 8})
        self.assertEqual(SLOT_SPELLINGS[2], {'green', '00cd00'})
        self.assertEqual(SLOT_SPELLINGS[3], {'brown', 'cdcd00'})
        self.assertEqual(SLOT_SPELLINGS[1], {'red', 'cd0000'})
        self.assertEqual(SLOT_SPELLINGS[8], {'brightblack', '7f7f7f'})

    def test_each_pinned_slot_is_accepted_in_both_spellings(self):
        # The status tag is never bold in the reference: `_status_attr` returns
        # only the colour pair, and the bold attribute belongs to the cursor row.
        for word, sgr in (('done', '32'), ('done', '38;5;2'), ('intr', '33'),
                          ('intr', '38;5;3'), ('err', '31'), ('empty', '38;5;8')):
            with self.subTest(word=word, sgr=sgr):
                problems = case_problems(body_with_tag(sgr, word), 'picker-normal')
                self.assertEqual(problems, [])

    def test_wrong_slot_or_default_ink_is_reported(self):
        # `intr` must not be green, and no tag may stay uncoloured.
        problems = case_problems(body_with_tag('1;32', 'intr'), 'picker-normal')
        self.assertTrue(any('expected palette slot 3' in p for p in problems), problems)
        problems = case_problems(body_with_tag('0', 'done'), 'picker-normal')
        self.assertTrue(any("ink ['default']" in p for p in problems), problems)

    def test_bold_or_stray_ink_is_reported(self):
        problems = case_problems(body_with_tag('1;32', 'done'), 'picker-normal')
        self.assertTrue(any('must not be bold' in p for p in problems), problems)
        stray = body_with_tag('1;32', 'done').copy()
        raw = base64.b64decode(stray['raw_base64']).replace(
            b'\x1b[0m  1', b'\x1b[0m\x1b[35m  1')
        encoded = base64.b64encode(raw).decode()
        stray.update({'raw_base64': encoded, 'raw_sha256': hashlib.sha256(raw).hexdigest(),
                      'snapshot_end_byte': len(raw), 'events_base64': [[0, encoded]]})
        problems = case_problems(stray, 'picker-normal')
        self.assertTrue(any('outside the status tag' in p for p in problems), problems)

    def test_unknown_tag_is_reported(self):
        problems = case_problems(body_with_tag('1;32', 'weird'), 'picker-normal')
        self.assertTrue(any('not one of the four pinned tags' in p for p in problems), problems)

    def test_filtered_single_row_screen_is_not_a_failure(self):
        screen = record('\x1b[4;1H\x1b[1;32m → second topic')
        self.assertEqual(body_rows(screen), [])
        self.assertEqual(case_problems(screen, 'picker-filter'), [])

    def test_normal_screen_without_body_row_is_reported(self):
        empty = record('\x1b[1;1H  Browse sessions')
        problems = case_problems(empty, 'picker-normal')
        self.assertTrue(any('expected at least 1 unselected body row' in p for p in problems), problems)

    def test_missing_matrix_is_error(self):
        cases = [c for c in self.bundle['cases'] if c['scenario'] in SCENARIOS]
        with self.assertRaisesRegex(RuntimeError, 'status-tag matrix'):
            check({'cases': cases[:4]}, 'python')

    def test_corrupt_trace_is_error(self):
        case = next(c for c in self.bundle['cases'] if c['scenario'] == 'picker-normal')
        broken = {**case['python'], 'raw_sha256': '0' * 64}
        with self.assertRaisesRegex(RuntimeError, 'Corrupt raw capture'):
            case_problems(broken, 'picker-normal')

    def test_screen_lookup_is_row_five_for_normal_widths(self):
        screen = screen_for(body_with_tag('38;5;2', 'done'))
        self.assertIn('done', screen.display[4])


if __name__ == '__main__':
    unittest.main()
