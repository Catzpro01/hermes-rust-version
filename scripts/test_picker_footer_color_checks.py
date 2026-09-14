"""Supporting decoder/evidence checks, not a replacement for live CLI RED."""
import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import unittest

from check_picker_footer_color import check, footer_ink_colors

ROOT = Path(__file__).resolve().parents[1]


class FooterColorChecks(unittest.TestCase):
    def setUp(self):
        self.bundle = json.loads((ROOT / 'docs/hermes-ui-spec/017/evidence/picker-position-0c0704d/paired-bundle.json').read_text())

    def test_reference_passes_and_previous_rust_fails(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check(self.bundle, 'python'), [])
            failures = check(self.bundle, 'rust')
        self.assertEqual(set(failures), {f'picker-{n}-{w}x30' for n in ('normal', 'filter', 'no-match') for w in (100, 80)})

    def test_decoder_aliases_for_palette_eight(self):
        # Synthetic emulator inputs only: never counted as actual application RED.
        for sgr, expected in [('90', 'brightblack'), ('38;5;8', '7f7f7f'), ('2', 'default')]:
            raw = f'\x1b[30;1H\x1b[{sgr}m1/2 sessions'.encode()
            encoded = base64.b64encode(raw).decode()
            record = {'width': 100, 'height': 30, 'error': None,
                      'snapshot_end_byte': len(raw), 'raw_base64': encoded,
                      'raw_sha256': hashlib.sha256(raw).hexdigest(), 'events_base64': [[0, encoded]]}
            with self.subTest(sgr=sgr):
                self.assertEqual(footer_ink_colors(record), {expected})

    def test_missing_matrix_is_error_not_color_failure(self):
        self.bundle['cases'].pop(0)
        with self.assertRaisesRegex(RuntimeError, 'six-footer matrix'):
            check(self.bundle, 'python')

    def test_corrupt_record_is_error(self):
        record = self.bundle['cases'][0]['python']
        record['raw_sha256'] = '0' * 64
        with self.assertRaisesRegex(RuntimeError, 'Corrupt raw'):
            footer_ink_colors(record)


if __name__ == '__main__':
    unittest.main()
