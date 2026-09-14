"""Capture the picker size contract on both sides, then pair the records.

The pinned upstream source (`_session_browse_picker`) fixes the size contract:
`max_y < 5 or max_x < 40` draws `Terminal too small` at row 1 *inside* the curses
screen, waits for one key and returns. The two widths here straddle the
threshold: 40 columns is the narrowest usable terminal, 39 must show the notice.

Modes:
  reference <reference-tree> <out.json>   Python side, from the pinned checkout
  rust <binary> <out.json>               Rust side, from the built CLI
  pair <rust.json> <python.json> <out.json>
"""
import base64
import hashlib
import json
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from capture_ui import PICKER_SIZE_CASES, UPSTREAM, capture_side  # noqa: E402

# The contract is per width: 40 columns is the narrowest usable terminal (the
# reference draws the picker, clipped), 39 must show the notice. The reference
# capture settled which is which: at 40 the pinned source draws the frame, at 39
# it prints `Terminal too small` and waits.
PAIRS = (('picker-narrow', 40), ('picker-too-small', 39))
WIDTHS = tuple(width for _, width in PAIRS)
# The pinned blob this work reads the contract from; the reference tree must
# carry exactly this file.
MAIN_PY_SHA256 = '89cde75d388ae3ff0d3512c00a0b4e77fe9f55c438874032b32967b4ab867567'


def summarize(cases, side):
    """Print one line per case and raise with only the failures, so a blocked
    job log still leaves a usable annotation. A failing child's own output is
    written to `reference-child.log`, because the traceback lives in the PTY
    bytes and would otherwise be trapped in the blocked artifact."""
    for case in cases:
        record = case[side]
        print(f'{side} {case["id"]} width={record["width"]} '
              f'error={record["error"]!r} bytes={len(record["raw_base64"]) // 4 * 3}', flush=True)
    failures = [(case['id'], case[side]['error']) for case in cases if case[side]['error']]
    if failures:
        chunks = []
        for case in cases:
            if not case[side]['error']:
                continue
            raw = base64.b64decode(case[side]['raw_base64'])
            chunks.append(f'=== {side} {case["id"]}: {case[side]["error"]}\n'
                          + raw.decode('utf-8', 'replace')[-1200:])
        Path('reference-child.log').write_text('\n'.join(chunks) + '\n')
    assert not failures, failures


def write(path, bundle):
    if Path(path).exists():
        raise RuntimeError('Refusing to overwrite evidence')
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(json.dumps(bundle, indent=2) + '\n')


def reference(tree, out):
    tree = Path(tree).resolve()
    marker = (tree / '.visual-evidence-reference').read_text().strip()
    assert marker == UPSTREAM, marker
    main_py = tree / 'hermes_cli' / 'main.py'
    digest = hashlib.sha256(main_py.read_bytes()).hexdigest()
    assert digest == MAIN_PY_SHA256, digest
    cases = capture_side('python', reference=tree, names=PICKER_SIZE_CASES, widths=WIDTHS,
                         timeout=45, pairs=PAIRS)
    summarize(cases, 'python')
    write(out, {'python_reference': UPSTREAM,
                'python_version': sys.version,
                'main_py_sha256': digest,
                'capture_driver_sha256': hashlib.sha256(Path(__file__).resolve().parent.joinpath('capture_ui.py').read_bytes()).hexdigest(),
                'capture_script_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'cases': cases})


def rust(binary, out):
    binary = Path(binary).resolve()
    cases = capture_side('rust', binary=binary, names=PICKER_SIZE_CASES, widths=WIDTHS,
                         timeout=45, pairs=PAIRS)
    summarize(cases, 'rust')
    write(out, {'rust_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                'capture_driver_sha256': hashlib.sha256(Path(__file__).resolve().parent.joinpath('capture_ui.py').read_bytes()).hexdigest(),
                'capture_script_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'cases': cases})


def pair(rust_path, python_path, out):
    rust_bundle = json.loads(Path(rust_path).read_text())
    python_bundle = json.loads(Path(python_path).read_text())
    references = {c['id']: c['python'] for c in python_bundle['cases']}
    cases = []
    for case in rust_bundle['cases']:
        references.pop(case['id'])
        cases.append({**case, 'python': None})
    assert not references, 'the reference bundle has cases the Rust capture lacks'
    by_id = {c['id']: c['python'] for c in python_bundle['cases']}
    for case in cases:
        case['python'] = by_id[case['id']]
    bundle = {key: value for key, value in rust_bundle.items() if key != 'cases'}
    bundle.update({'python_origin': {
        'bundle_path': str(python_path),
        'bundle_sha256': hashlib.sha256(Path(python_path).read_bytes()).hexdigest(),
        'python_reference': python_bundle['python_reference'],
        'main_py_sha256': python_bundle['main_py_sha256'],
        'reuse_note': 'fresh reference capture in the same runner run, not a retained record',
    }, 'cases': cases})
    write(out, bundle)


if __name__ == '__main__':
    mode, args = sys.argv[1], sys.argv[2:]
    {'reference': reference, 'rust': rust, 'pair': pair}[mode](*args)
