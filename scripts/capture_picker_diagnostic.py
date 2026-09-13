"""Capture only the six picker states in the diagnosis, using the real recorder."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from capture_ui import capture_side

binary, output = map(Path, sys.argv[1:])
assert not output.exists(), 'never overwrite evidence'
assert not subprocess.check_output(['git', 'diff', '--binary', '--', '*.rs'])
bundle = {'rust_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
          'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
          'capture_driver_sha256': hashlib.sha256(Path('scripts/capture_ui.py').read_bytes()).hexdigest(),
          'scope': 'picker delete hint only, not whole-screen parity',
          'cases': capture_side('rust', binary=binary.resolve(), names=('picker-normal', 'picker-filter', 'picker-no-match'))}
assert len(bundle['cases']) == 6 and all(c['rust']['error'] is None for c in bundle['cases'])
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(bundle, indent=2)+'\n')
