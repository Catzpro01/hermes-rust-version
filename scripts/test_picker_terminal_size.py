"""Primary terminal-size regression at the agreed actual CLI/PTY seam.

Requires an already built HERMES_PICKER_BINARY. Capture/setup errors are errors,
not contract failures. The pinned threshold is `max_y < 5 or max_x < 40`, the
notice is drawn inside the screen and the picker waits for one key before
returning.
"""
import hashlib
import subprocess
import json
import os
from pathlib import Path
import unittest

from capture_ui import PICKER_SIZE_CASES, capture_side
from check_picker_terminal_size import NARROW, TOO_SMALL, size_problems

# Each scenario belongs to one width: see capture_picker_sizes.PAIRS.
PAIRS = (('picker-narrow', NARROW), ('picker-too-small', TOO_SMALL))


class PickerTerminalSizeTests(unittest.TestCase):
    def test_size_contract_follows_the_pinned_threshold(self):
        binary = Path(os.environ['HERMES_PICKER_BINARY']).resolve()
        if not binary.is_file():
            raise RuntimeError('Build the actual CLI first')
        cases = capture_side('rust', binary=binary, names=PICKER_SIZE_CASES,
                             widths=(NARROW, TOO_SMALL), timeout=45, pairs=PAIRS)
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
                            for problem in size_problems(case['rust'], case['rust']['width']))
        self.assertEqual(
            problems, [],
            'Pinned size contract: 40 columns draw the picker, 39 columns draw only '
            '"Terminal too small" at row 1 inside the screen and wait for one key')


if __name__ == '__main__':
    unittest.main(verbosity=2)
