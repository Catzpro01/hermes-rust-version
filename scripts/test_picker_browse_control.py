"""Primary browse-control regression at the agreed actual CLI/PTY seam.

Requires an already built HERMES_PICKER_BINARY. Capture/setup errors are
errors, not style assertion failures. The pinned reference (`_curses_browse`)
repaints at the new geometry after a resize (exiting with `Terminal too small`
below 5 rows / 40 columns), wraps the cursor modulo on long lists with a
minimally clamped scroll window, and restores the full list with cursor/offset
at zero when the filter is cleared via Esc or Backspace. See
`docs/hermes-ui-spec/017/evidence/upstream-browse-control/` (Spec 017 W2).
"""
import hashlib
import subprocess
import json
import os
from pathlib import Path
import unittest

from capture_ui import capture_side
from check_picker_browse_control import SCENARIOS, case_problems


class PickerBrowseControlTests(unittest.TestCase):
    def test_picker_browse_control_matches_the_pinned_reference(self):
        binary = Path(os.environ['HERMES_PICKER_BINARY']).resolve()
        if not binary.is_file():
            raise RuntimeError('Build the actual CLI first')
        cases = capture_side('rust', binary=binary, names=SCENARIOS,
                             widths=(100, 80), timeout=60)
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
            'Reference behaviour (`_curses_browse`): resize repaints at the new '
            'geometry (below 5x40 the picker shows `Terminal too small` and exits '
            'at the next key); long lists wrap the cursor modulo with a minimally '
            'clamped window; clearing the filter resets cursor/offset to zero')


if __name__ == '__main__':
    unittest.main(verbosity=2)
