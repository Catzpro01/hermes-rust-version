"""Pair a fresh Rust picker capture with retained Python records.

The Python side is never recaptured here: it is copied unchanged from an
existing retained bundle, and the new bundle records where it came from so the
provenance can be re-verified. Cases are matched by id and must be identical
sets; the Rust side is copied verbatim from the runner-produced capture.
"""
import argparse
import hashlib
import json
from pathlib import Path

PROVENANCE_KEYS = ('rust_commit', 'binary_sha256', 'capture_driver_sha256',
                   'scope', 'rustc_version', 'cargo_version',
                   'capture_script_sha256')


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rust', type=Path, required=True)
    parser.add_argument('--python', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--reference', required=True,
                        help='Pinned Python source revision the records came from')
    parser.add_argument('--original-capture-checkout', required=True,
                        help='Commit whose capture produced the retained Python records')
    parser.add_argument('--reuse-note', default='unchanged original Python records, not a new capture')
    args = parser.parse_args()
    assert not args.out.exists(), 'never overwrite evidence'

    rust = json.loads(args.rust.read_text())
    python = json.loads(args.python.read_text())
    references = {c['id']: c for c in python['cases']}
    rust_cases = {c['id']: c for c in rust['cases']}
    assert rust_cases, 'empty Rust capture'
    assert set(references) >= set(rust_cases), 'Rust capture has unknown case ids'

    bundle = {key: rust[key] for key in PROVENANCE_KEYS if key in rust}
    bundle['python_origin'] = {
        'reference': args.reference,
        'original_capture_checkout': args.original_capture_checkout,
        'bundle_path': str(args.python),
        'bundle_sha256': sha256(args.python),
        'reuse': args.reuse_note,
    }
    cases = []
    for case in rust['cases']:
        reference = references[case['id']]
        assert reference['scenario'] == case['scenario'], case['id']
        assert case['fixture'] == reference['fixture'], case['id']
        cases.append({'id': case['id'], 'scenario': case['scenario'],
                      'fixture': case['fixture'], 'rust': case['rust'],
                      'python': reference['python']})
    bundle['cases'] = cases
    bundle['scope'] = rust['scope']
    args.out.write_text(json.dumps(bundle, indent=2) + '\n')
    print(f'{args.out}: {len(cases)} cases, sha256={sha256(args.out)}')


if __name__ == '__main__':
    main()
