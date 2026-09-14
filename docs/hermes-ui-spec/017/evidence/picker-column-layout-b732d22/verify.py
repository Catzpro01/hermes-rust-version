"""Verify retained provenance, pinned-renderer regions and column-layout parity.

No originals are rewritten and nothing is normalized. The script re-derives the
audit from the checked-in artifacts: bundle/patch hashes, the request-only
capture commit, reused Python records, per-case changed-row maps, the live
checkers re-run over the paired bundle, raw/cast round trips, PNG hashes and the
pinned-renderer region comparisons. Direct image inspection is documented in
REPORT.md, not claimed here.
"""
import base64
import contextlib
import gzip
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
BASELINE = HERE.parent / 'picker-filter-header-9cc5cb4'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
load = lambda p: json.loads(Path(p).read_text())
sys.path.insert(0, str(ROOT / 'scripts'))

FIX_COMMIT = '641c3041f6e0e94421173552cacc0adeae3c0de6'
CAPTURE_COMMIT = 'b732d2273f2e55f2674aad56ee10a7b70f512514'
RUST_BUNDLE_SHA = 'd6860b3d6038f6f3ca117ef969d601eea55fea5b2af34590465389b7c53dbad2'
TESTED_PATCH_SHA = '006bdcc75217da662ca838e1ffb2c4f27d215a21d5465b8a39605f6f68358271'
REJECTED_PATCH_SHA = '2c79b6d1181e74e75f0661071ce625ae9cb35ae20e3f150cd68844b2bfdd65f2'
RENDERER_SHA = '201091eef5007e9a45889da7b4725b658ce39df1bf977a48677e435df9f9c074'
EXPECTED_CHANGED = {
    'picker-normal-100x30': [2, 3, 4, 5],
    'picker-filter-100x30': [2, 3, 4],
    'picker-no-match-100x30': [2, 3, 4],
    'picker-delete-100x30': [2, 3, 4, 5],
    'picker-empty-100x30': [],
    'picker-normal-80x30': [2, 3, 4, 5],
    'picker-filter-80x30': [2, 3, 4],
    'picker-no-match-80x30': [2, 3, 4],
    'picker-delete-80x30': [2, 3, 4, 5],
    'picker-empty-80x30': [],
}
# The no-match message is dim in the retained reference; pyte 0.8.2 cannot
# decode the dim attribute, so those regions still differ in this slice.
OPEN_DIM_REGIONS = {'no-match message row'}

bundle = load(HERE / 'paired-bundle.json')
rust = load(HERE / 'rust-bundle.json')
assert sha(HERE / 'rust-bundle.json') == RUST_BUNDLE_SHA
assert rust['rust_commit'] == CAPTURE_COMMIT
assert len(rust['cases']) == 10
assert sha(HERE / 'tested.patch') == TESTED_PATCH_SHA
assert (HERE / 'tested.patch').stat().st_size == 12824
assert sha(HERE / 'rejected-first-attempt.patch') == REJECTED_PATCH_SHA
assert (HERE / 'first-attempt-clippy.txt').read_text().strip()
for file, field in [('scripts/capture_ui.py', 'capture_driver_sha256'),
                    ('scripts/capture_picker_diagnostic.py', 'capture_script_sha256')]:
    source = subprocess.check_output(['git', 'show', rust['rust_commit'] + ':' + file], cwd=ROOT)
    assert hashlib.sha256(source).hexdigest() == rust[field]
# The captured commit differs from the fix commit only by the diagnostic request.
diff = subprocess.check_output(
    ['git', 'diff', '--name-only', FIX_COMMIT, rust['rust_commit']], cwd=ROOT, text=True).split()
assert diff == ['.scratch/hermes-rs-total-parity/diagnostics/picker/request.json'], diff
origin = bundle['python_origin']
parent = ROOT / origin['bundle_path']
assert sha(parent) == origin['bundle_sha256']
references = {c['id']: c for c in load(parent)['cases']}
rust_cases = {c['id']: c for c in bundle['cases']}
renderer = load(HERE / 'renderer.json')
prior_renderer = load(BASELINE / 'renderer.json')
assert renderer['input_sha256'] == sha(HERE / 'paired-bundle.json')
for key in ('browser', 'xterm', 'font', 'fontSize', 'deviceScaleFactor', 'normalization', 'fonts'):
    assert renderer[key] == prior_renderer[key]
renderer_sha = sha(ROOT / 'scripts/visual-renderer/render.cjs')
assert renderer_sha == RENDERER_SHA

changes = {}
encoding_modes = {'python': set(), 'rust': set()}
unchanged_pngs = []
roundtrips = 0
for case, rendered in zip(bundle['cases'], renderer['cases']):
    ident = case['id']
    assert ident == rendered['id']
    assert case['python'] == references[ident]['python']
    assert case['rust'] == rust_cases[ident]['rust']
    previous = json.loads(gzip.decompress((BASELINE / f'{ident}-cells.json.gz').read_bytes()))
    cells = json.loads(gzip.decompress((HERE / f'{ident}-cells.json.gz').read_bytes()))
    assert cells['python'] == previous['python']
    assert len(cells['rust']) == len(previous['rust']) == 30
    changed = [y + 1 for y in range(30) if cells['rust'][y] != previous['rust'][y]]
    assert changed == EXPECTED_CHANGED[ident], (ident, changed)
    # The header text is now the reference text at the reference coordinates.
    assert cells['rust'][1]['text'] == cells['python'][1]['text']
    assert cells['rust'][2]['text'] == ''
    if case['scenario'] in ('picker-normal', 'picker-filter', 'picker-delete'):
        assert cells['rust'][3]['text'].startswith(('   ', ' → ')), ident
    elif case['scenario'] == 'picker-no-match':
        assert cells['rust'][3]['text'] == cells['python'][3]['text'] == \
            '  No sessions match the filter.'
    for side in ('python', 'rust'):
        encoding_modes[side].update(c['fm'] for c in cells[side][4]['cells'] if c['s'])
    changes[ident] = changed
    for image in rendered['images']:
        assert sha(HERE / image['file']) == image['sha256']
        if not changed:
            assert sha(HERE / image['file']) == sha(BASELINE / image['file'])
            unchanged_pngs.append(image['file'])
    # Raw/cast round trip and retained-stream integrity, both sides.
    for side in ('python', 'rust'):
        record = case[side]
        raw = base64.b64decode(record['raw_base64'], validate=True)
        assert hashlib.sha256(raw).hexdigest() == record['raw_sha256']
        events = b''.join(base64.b64decode(e[1], validate=True) for e in record['events_base64'])
        assert events == raw
        assert (HERE / f'{ident}-{side}.ansi').read_bytes() == raw
        cast = (HERE / f'{ident}-{side}.cast').read_text().splitlines()
        assert json.loads(cast[0])['width'] == record['width']
        roundtrips += 1

# The live checkers re-run over the same paired bundle; the new gate must pass on
# both sides, the older gates on the Rust capture.
import check_picker_column_layout  # noqa: E402
import check_picker_counter  # noqa: E402
import check_picker_filter_header  # noqa: E402
import check_picker_filter_hint  # noqa: E402
import check_picker_footer_color  # noqa: E402
import check_picker_footer_position  # noqa: E402
import check_picker_normal_header  # noqa: E402
checker_runs = []
for module, sides in [(check_picker_column_layout, ('python', 'rust')),
                      (check_picker_footer_position, ('rust',)),
                      (check_picker_footer_color, ('rust',)),
                      (check_picker_normal_header, ('rust',)),
                      (check_picker_filter_header, ('rust',)),
                      (check_picker_counter, ('rust',)),
                      (check_picker_filter_hint, ('rust',))]:
    for side in sides:
        with contextlib.redirect_stdout(io.StringIO()) as captured:
            assert module.check(bundle, side) == []
        checker_runs.append(f'{module.__name__.split(".")[-1]}:{side}:'
                            f'{len(captured.getvalue().splitlines())}')

# Pinned-renderer regions: the reference geometry must be pixel-identical, and the
# dim no-match message is recorded as the open item it is.
equal_regions, differing_regions = [], []
for name in ('region-pixels.json', 'footer-pixels.json'):
    for entry in load(HERE / name)['results']:
        assert entry['png_sha256'] == sha(HERE / f'{entry["id"]}-bottom.png')
        for region in entry['regions']:
            label = f'{entry["id"]}:{region["label"]}'
            if region['equal']:
                assert region['differing_pixels'] == 0
                assert region['label'] not in OPEN_DIM_REGIONS, label
                equal_regions.append(label)
            else:
                assert region['label'] in OPEN_DIM_REGIONS, label
                differing_regions.append(label)
assert len(differing_regions) == 4, differing_regions
assert len(set(differing_regions)) == 2, differing_regions
assert len(equal_regions) == 14, equal_regions

audit = {
    'status': 'COLUMN_LAYOUT_FIXED_NO_MATCH_DIM_STILL_OPEN',
    'source': rust['rust_commit'],
    'fix_commit': FIX_COMMIT,
    'raw_cast_roundtrips': roundtrips,
    'png_hash_checks': 10,
    'python_records_and_cells_unchanged': True,
    'renderer_script_sha256': renderer_sha,
    'renderer_settings_match_prior': True,
    'checker_runs': checker_runs,
    'checker_failures': 0,
    'encoding_modes': {'python': 'palette16', 'rust': 'palette256'},
    'tested_patch': {'sha256': TESTED_PATCH_SHA, 'bytes': 12824},
    'rejected_first_attempt': {'sha256': REJECTED_PATCH_SHA,
                               'reason': 'clippy::too_many_arguments (8/7) in format_row'},
    'changed_rows_1_based': changes,
    'unchanged_paired_pngs': sorted(unchanged_pngs),
    'pixel_equal_regions': sorted(equal_regions),
    'pixel_differing_regions': sorted(differing_regions),
    'open_finding': ('reference renders "No sessions match the filter." with the dim attribute; '
                     'pyte 0.8.2 cannot decode dim, so the fix and its gate follow in the next cycle'),
    'normalization': 'none',
}
current = HERE / 'audit.json'
text = json.dumps(audit, indent=2) + '\n'
if current.exists():
    assert current.read_text() == text, 'Existing audit disagrees'
else:
    current.write_text(text)
print(json.dumps({'status': audit['status'], 'roundtrips': roundtrips,
                  'checker_runs': len(checker_runs), 'equal_regions': len(equal_regions),
                  'differing_regions': differing_regions,
                  'unchanged_pngs': len(unchanged_pngs)}, indent=2))
