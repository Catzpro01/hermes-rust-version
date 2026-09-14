"""Primary footer-color regression at the agreed actual CLI/PTY seam.

Requires an already built HERMES_PICKER_BINARY. Capture/setup errors are errors,
not color assertion failures. A normal 100x30 pane is the first TDD slice.
"""
import hashlib
import subprocess
import json
import os
from pathlib import Path
import unittest

from capture_ui import capture_side
from check_picker_footer_color import footer_ink_colors


class PickerFooterColorTests(unittest.TestCase):
    def test_footer_uses_reference_grey(self):
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
        colors = footer_ink_colors(case['rust'])
        # pyte exposes the same palette slot in two forms depending on encoding.
        # The acceptance xterm cell audit separately verifies palette mode/index.
        self.assertIn(colors, ({'brightblack'}, {'7f7f7f'}),
                      'Python reference: footer uses terminal palette index8, not default')


if __name__ == '__main__':
    unittest.main(verbosity=2)
