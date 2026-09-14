"""Supporting evidence/decoder checks; not a substitute for actual CLI RED."""
import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import unittest

from check_picker_normal_header import NORMAL_HELP, check, normal_header_style

ROOT = Path(__file__).resolve().parents[1]


class NormalHeaderChecks(unittest.TestCase):
    def setUp(self):
        self.bundle = json.loads((ROOT / 'docs/hermes-ui-spec/017/evidence/picker-color-ae220ff/paired-bundle.json').read_text())

    def test_independent_reference_and_previous_rust(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check(self.bundle, 'python'), [])
            failures = check(self.bundle, 'rust')
        self.assertEqual(set(failures), {f'picker-{n}-{w}x30' for n in ('normal', 'delete') for w in (100, 80)})

    def test_decoder_preserves_palette_and_weight(self):
        for sgr, expected in [('1;33', ('brown', True)), ('1;38;5;3', ('cdcd00', True)),
                              ('33', ('brown', False)), ('1;93', ('brightbrown', True))]:
            raw = f'\x1b[{sgr}m{NORMAL_HELP}'.encode()
            encoded = base64.b64encode(raw).decode()
            record = {'width': 100, 'height': 30, 'error': None,
                      'snapshot_end_byte': len(raw), 'raw_base64': encoded,
                      'raw_sha256': hashlib.sha256(raw).hexdigest(), 'events_base64': [[0, encoded]]}
            with self.subTest(sgr=sgr):
                self.assertEqual(normal_header_style(record), {expected})

    def test_missing_matrix_is_error(self):
        self.bundle['cases'].pop(0)
        with self.assertRaisesRegex(RuntimeError, 'four-header matrix'):
            check(self.bundle, 'python')

    def test_corrupt_trace_is_error(self):
        record = self.bundle['cases'][0]['python']
        record['raw_sha256'] = '0' * 64
        with self.assertRaisesRegex(RuntimeError, 'Corrupt raw'):
            normal_header_style(record)


if __name__ == '__main__':
    unittest.main()
