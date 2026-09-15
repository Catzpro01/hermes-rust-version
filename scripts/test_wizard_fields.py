"""Primary wizard-field evidence gate at the agreed actual CLI/PTY seam.

Requires an already built HERMES_PICKER_BINARY. Capture/setup errors are
errors, not assertion failures. The W3 contract (Spec 017 Lane 2): every
prompt/label/value the wizard renders per section frame must appear in the
captured evidence — the full common-model field sequence, the docker-image
field after the unavailable-docker notice, and the cancel paths. Secrets are
never part of these scenarios (gateway token entry stays out of scope here).
"""
import hashlib
import subprocess
import json
import os
from pathlib import Path
import unittest

from capture_ui import capture_side
from check_wizard_fields import SCENARIOS, case_problems


class WizardFieldsTests(unittest.TestCase):
    def test_wizard_renders_every_contracted_field(self):
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
            'W3 contract: every rendered wizard field (provider, API base URL, '
            'key environment variable, model name, docker image) must appear in '
            'captured evidence, in order; cancel paths end with `Setup cancelled.`')


if __name__ == '__main__':
    unittest.main(verbosity=2)
