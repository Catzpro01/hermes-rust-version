"""Supporting evidence/decoder checks; not a substitute for actual CLI RED."""
import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import unittest

from check_picker_filter_header import FILTER_PREFIX, check, filter_header_style

ROOT = Path(__file__).resolve().parents[1]


class FilterHeaderChecks(unittest.TestCase):
    def setUp(self):
        self.bundle = json.loads((ROOT / 'docs/hermes-ui-spec/017/evidence/ui-3b39bd7/python-bundle.json').read_text())

    def test_reference_records_pass(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check(self.bundle, 'python'), [])

    def test_decoder_accepts_only_palette_slot_six(self):
        for sgr, expected in [('1;36', ('cyan', True)), ('1;38;5;6', ('00cdcd', True)),
                              ('1;38;5;14', ('00ffff', True)), ('36', ('cyan', False)),
                              ('1;96', ('brightcyan', True))]:
            raw = f'\x1b[{sgr}m{FILTER_PREFIX}topic█'.encode()
            encoded = base64.b64encode(raw).decode()
            record = {'width': 100, 'height': 30, 'error': None,
                      'snapshot_end_byte': len(raw), 'raw_base64': encoded,
                      'raw_sha256': hashlib.sha256(raw).hexdigest(), 'events_base64': [[0, encoded]]}
            with self.subTest(sgr=sgr):
                self.assertEqual(filter_header_style(record), {expected})

    def test_bright_variant_is_rejected_by_the_checker(self):
        raw = f'\x1b[1;38;5;14m{FILTER_PREFIX}topic█'.encode()
        encoded = base64.b64encode(raw).decode()
        record = {'width': 100, 'height': 30, 'error': None,
                  'snapshot_end_byte': len(raw), 'raw_base64': encoded,
                  'raw_sha256': hashlib.sha256(raw).hexdigest(), 'events_base64': [[0, encoded]]}
        self.assertEqual(filter_header_style(record), {('00ffff', True)})
        case = {**self.bundle['cases'][0], 'python': {**self.bundle['cases'][0]['python'], **record}}
        bundle = {'cases': [case]}
        with self.assertRaises(RuntimeError):
            check(bundle, 'python')

    def test_unfiltered_header_is_not_accepted_here(self):
        raw = b'\x1b[1;38;5;3m  Browse sessions \xe2\x80\x94 \xe2\x86\x91\xe2\x86\x93 navigate  Enter select  Type to filter  Esc quit'
        encoded = base64.b64encode(raw).decode()
        record = {'width': 100, 'height': 30, 'error': None,
                  'snapshot_end_byte': len(raw), 'raw_base64': encoded,
                  'raw_sha256': hashlib.sha256(raw).hexdigest(), 'events_base64': [[0, encoded]]}
        with self.assertRaisesRegex(RuntimeError, 'filter help header'):
            filter_header_style(record)

    def test_missing_matrix_is_error(self):
        self.bundle['cases'] = [c for c in self.bundle['cases'] if c['scenario'] != 'picker-no-match']
        with self.assertRaisesRegex(RuntimeError, 'four filter-header matrix'):
            check(self.bundle, 'python')

    def test_corrupt_trace_is_error(self):
        record = [c for c in self.bundle['cases'] if c['scenario'] == 'picker-filter'][0]['python']
        record = {**record, 'raw_sha256': '0' * 64}
        with self.assertRaisesRegex(RuntimeError, 'Corrupt raw'):
            filter_header_style(record)


if __name__ == '__main__':
    unittest.main()
