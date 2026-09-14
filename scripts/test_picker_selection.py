"""Primary selected-row style regression at the agreed actual CLI/PTY seam.

Requires an already built HERMES_PICKER_BINARY. Capture/setup errors are errors,
not style assertion failures. The retained reference pins 100 and 80 columns.
"""
import hashlib
import subprocess
import json
import os
from pathlib import Path
import unittest

from capture_ui import capture_side
from check_picker_selection import case_problems


class PickerSelectionTests(unittest.TestCase):
    def test_selected_row_matches_reference_style(self):
        binary = Path(os.environ['HERMES_PICKER_BINARY']).resolve()
        if not binary.is_file():
            raise RuntimeError('Build the actual CLI first')
        cases = capture_side('rust', binary=binary,
                             names=('picker-normal', 'picker-filter', 'picker-delete'),
                             widths=(100, 80))
        output = Path(os.environ['HERMES_PICKER_RECORDING'])
        if output.exists():
            raise RuntimeError('Never overwrite a recording')
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps({'cases': cases,
                                     'checkout': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                                     'rust_diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', '--binary', '--', '*.rs'])).hexdigest(),
                                     'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}, indent=2)+'\n')
        problems = []
        for case in cases:
            problems.extend(f'{case["id"]}: {problem}'
                            for problem in case_problems(case['rust'], case['scenario']))
        self.assertEqual(
            problems, [],
            'Python reference: the selected row is the whole row in palette2 green + bold with no reverse video')


if __name__ == '__main__':
    unittest.main(verbosity=2)
