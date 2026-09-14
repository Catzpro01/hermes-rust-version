"""Primary filter help-header style regression at the agreed actual CLI/PTY seam.

Requires an already built HERMES_PICKER_BINARY. Capture/setup errors are errors,
not style assertion failures. A filtered 100x30 pane is the first TDD slice.
"""
import hashlib
import subprocess
import json
import os
from pathlib import Path
import unittest

from capture_ui import capture_side
from check_picker_filter_header import filter_header_style


class PickerFilterHeaderTests(unittest.TestCase):
    def test_filter_help_header_matches_reference_style(self):
        binary = Path(os.environ['HERMES_PICKER_BINARY']).resolve()
        if not binary.is_file():
            raise RuntimeError('Build the actual CLI first')
        case = capture_side('rust', binary=binary, names=('picker-filter',), widths=(100,))[0]
        output = Path(os.environ['HERMES_PICKER_RECORDING'])
        if output.exists():
            raise RuntimeError('Never overwrite a recording')
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps({'cases': [case],
                                     'checkout': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                                     'rust_diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', '--binary', '--', '*.rs'])).hexdigest(),
                                     'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}, indent=2)+'\n')
        style = filter_header_style(case['rust'])
        self.assertIn(style, ({('cyan', True)}, {('00cdcd', True)}),
                      'Python reference: filter help header uses palette6 (not the bright variant) and bold')


if __name__ == '__main__':
    unittest.main(verbosity=2)
