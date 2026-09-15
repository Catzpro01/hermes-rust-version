"""Primary completion evidence gate at the agreed actual CLI/PTY seam.

Requires an already built HERMES_PICKER_BINARY. Capture/setup errors are
errors, not assertion failures. The W4 contract (Spec 017 Lane 3): the
Rust inline dropdown is proven by pinned PTY evidence — the REPL welcome
frame, Tab-driven unique completion and candidate dropdown, seeded skill
completion, Tab-less ghost text, and an accepted completion opening the
browse picker. Python-side comparison awaits the user's reference
recording (W4-Q1); this gate pins the Rust side only.
"""
import hashlib
import subprocess
import json
import os
from pathlib import Path
import unittest

from capture_ui import capture_side
from check_completion_dropdown import SCENARIOS, case_problems


class CompletionDropdownTests(unittest.TestCase):
    def test_completion_surface_matches_contract(self):
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
            'W4 contract: the REPL welcome frame, Tab completion (unique word, '
            'candidate dropdown, seeded skill), Tab-less ghost text, and the '
            'accepted /sessions completion opening the browse picker must all '
            'appear in captured evidence, in order')


if __name__ == '__main__':
    unittest.main(verbosity=2)
