"""Supporting geometry/evidence checker tests, not the primary live CLI test."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest

from check_picker_footer_position import check, footer_row

ROOT = Path(__file__).resolve().parents[1]


class FooterEvidenceChecks(unittest.TestCase):
    def setUp(self):
        original = json.loads((ROOT / 'docs/hermes-ui-spec/017/evidence/ui-3b39bd7/paired-bundle.json').read_text())
        self.bundle = {'cases': [c for c in original['cases'] if c['scenario'].startswith('picker-')]}

    def test_reference_and_historical_geometry(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check(self.bundle, 'python'), [])
            failures = check(self.bundle, 'rust')
        self.assertEqual(set(failures), {f'picker-{n}-{w}x30' for n in ('normal', 'filter', 'no-match') for w in (100, 80)})

    def test_missing_case_is_error_not_geometry_failure(self):
        self.bundle['cases'].pop()
        with self.assertRaisesRegex(RuntimeError, 'ten-case matrix'):
            check(self.bundle, 'python')

    def test_mismatched_dimensions_are_error(self):
        self.bundle['cases'][0]['python']['width'] = 79
        with self.assertRaisesRegex(RuntimeError, 'dimensions'):
            check(self.bundle, 'python')

    def test_corrupt_capture_is_error_not_geometry_failure(self):
        record = copy.deepcopy(self.bundle['cases'][0]['python'])
        record['raw_sha256'] = '0' * 64
        with self.assertRaisesRegex(RuntimeError, 'Corrupt raw'):
            footer_row(record)


if __name__ == '__main__':
    unittest.main()
