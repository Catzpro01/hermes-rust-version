"""Primary status-tag CONTENT regression at the agreed actual CLI/PTY seam.

`test_picker_status_ink.py` pins the ink and the column of the status tag; this
gate pins which lifecycle state a row may report at all. The fixture seeds one
session per shape the pinned reference distinguishes, so a port that collapses
every non-empty session onto `done` fails on the two `intr` rows and the `err`
row.

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
from check_picker_status_tags import SCENARIO, WIDTHS, case_problems


class PickerStatusTagsTests(unittest.TestCase):
    def test_every_pinned_lifecycle_shape_is_reachable(self):
        binary = Path(os.environ['HERMES_PICKER_BINARY']).resolve()
        if not binary.is_file():
            raise RuntimeError('Build the actual CLI first')
        cases = capture_side('rust', binary=binary, names=(SCENARIO,),
                             widths=WIDTHS, timeout=45)
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
                            for problem in case_problems(case['rust']))
        self.assertEqual(
            problems, [],
            'Pinned lifecycle content, read from the last message row: '
            'error finish_reason -> err (checked before the role), user/tool last '
            'row or assistant row carrying tool_calls -> intr, any other last row '
            '-> done, no message row at all -> empty')


if __name__ == '__main__':
    unittest.main(verbosity=2)
