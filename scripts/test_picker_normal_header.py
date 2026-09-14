"""Primary normal help-header style regression at the agreed actual CLI/PTY seam.

Requires an already built HERMES_PICKER_BINARY. Capture/setup errors are errors,
not style assertion failures. A normal 100x30 pane is the first TDD slice.
"""
import hashlib
import subprocess
import json
import os
from pathlib import Path
import unittest

from capture_ui import capture_side
from check_picker_normal_header import normal_header_style


class PickerNormalHeaderTests(unittest.TestCase):
    def test_normal_help_header_matches_reference_style(self):
        binary = Path(os.environ['HERMES_PICKER_BINARY']).resolve()
        if not binary.is_file():
            raise RuntimeError('Build the actual CLI first')
        case = capture_side('rust', binary=binary, names=('picker-normal',), widths=(100,), timeout=45)[0]
        output = Path(os.environ['HERMES_PICKER_RECORDING'])
        if output.exists():
            raise RuntimeError('Never overwrite a recording')
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps({'cases': [case],
                                     'checkout': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                                     'rust_diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', '--binary', '--', '*.rs'])).hexdigest(),
                                     'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}, indent=2)+'\n')
        style = normal_header_style(case['rust'])
        self.assertIn(style, ({('brown', True)}, {('cdcd00', True)}),
                      'Python reference: normal help header uses palette3 and bold')


if __name__ == '__main__':
    unittest.main(verbosity=2)
