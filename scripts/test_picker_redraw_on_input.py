"""Primary redraw-on-input regression at the agreed actual CLI/PTY seam.

Requires an already built HERMES_PICKER_BINARY. Capture/setup errors are errors,
not style assertion failures. The retained reference draws one frame per key
(`stdscr.getch()` blocks) and never repaints while it waits.
"""
import hashlib
import subprocess
import json
import os
from pathlib import Path
import unittest

from capture_ui import capture_side
from check_picker_redraw_on_input import SCENARIOS, case_problems


class PickerRedrawOnInputTests(unittest.TestCase):
    def test_picker_draws_on_input_not_on_its_poll_timeout(self):
        binary = Path(os.environ['HERMES_PICKER_BINARY']).resolve()
        if not binary.is_file():
            raise RuntimeError('Build the actual CLI first')
        cases = capture_side('rust', binary=binary, names=SCENARIOS,
                             widths=(100, 80), timeout=45)
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
            'Reference behaviour (`_curses_browse`): draw the frame, then block in getch() — '
            'one frame per typed key and nothing at all while waiting for input')


if __name__ == '__main__':
    unittest.main(verbosity=2)
