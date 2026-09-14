"""Verify retained provenance, the selected-row fix and pinnes pixel regions.

No originals are rewritten and nothing is normalized. The script re-derives the
audit from the checked-in artifacts: bundle/patch hashes, the capture commit's
tooling provenance, reused Python records, the per-case changed-row map against
the previous packet, attribute-level cell deltas, the nine live checkers re-run
over the paired bundle, raw/cast round trips, PNG hashes and the pinned-renderer
region comparisons. Direct image inspection is documented in REPORT.md, not
claimed here.
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
BASELINE = HERE.parent / 'picker-message-style-fd674dc'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
load = lambda p: json.loads(Path(p).read_text())
sys.path.insert(0, str(ROOT / 'scripts'))

FIX_COMMIT = 'ee115414e012'
CAPTURE_COMMIT = 'b4cb408a3bc4a0a35131206a7dc3e902fc1c6928'
RUST_BUNDLE_SHA = 'fe8bc2c2d1fb5b41a8b2f4e5e0c73cedd8b3789ad7fc8e61a91aabd3f7e102bd'
TESTED_PATCH_SHA = '8eed758f33bb38b6409517f31f0faccaded907706be97dbe7443f2f064253ee5'
RENDERER_SHA = '201091eef5007e9a45889da7b4725b658ce39df1bf977a48677e435df9f9c074'
MARKER = ' → '
# Only the cursor row changed, at both widths, in the three scenarios that have a
# cursor row; the no-match and empty screens have none.
EXPECTED_CHANGED = {
    'picker-normal-100x30': [4], 'picker-filter-100x30': [4], 'picker-delete-100x30': [4],
    'picker-no-match-100x30': [], 'picker-empty-100x30': [],
    'picker-normal-80x30': [4], 'picker-filter-80x30': [4], 'picker-delete-80x30': [4],
    'picker-no-match-80x30': [], 'picker-empty-80x30': [],
}

bundle = load(HERE / 'paired-bundle.json')
rust = load(HERE / 'rust-bundle.json')
assert sha(HERE / 'rust-bundle.json') == RUST_BUNDLE_SHA
assert rust['rust_commit'] == CAPTURE_COMMIT
assert len(rust['cases']) == 10
assert sha(HERE / 'tested.patch') == TESTED_PATCH_SHA
assert (HERE / 'tested.patch').stat().st_size == 1171
for file, field in [('scripts/capture_ui.py', 'capture_driver_sha256'),
                    ('scripts/capture_picker_diagnostic.py', 'capture_script_sha256')]:
    source = subprocess.check_output(['git', 'show', rust['rust_commit'] + ':' + file], cwd=ROOT)
    assert hashlib.sha256(source).hexdigest() == rust[field], file
# The capture commit may carry tooling-only changes, but no Rust source may differ
# from the commit whose tested patch was applied.
assert subprocess.check_output(['git', 'diff', '--name-only', FIX_COMMIT, rust['rust_commit'],
                                '--', '*.rs'], cwd=ROOT, text=True).strip() == ''
# ... and the applied patch is exactly the tested one.
text = subprocess.check_output(['git', 'show', FIX_COMMIT + ':crates/hermes-cli/src/session_picker.rs'],
                               cwd=ROOT, text=True)
assert 'SetForegroundColor(Color::DarkGreen)' in text
assert 'Attribute::Reverse' not in text
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
attribute_deltas = {'selection_ink': 0}
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
    for row in changed:
        before, after = previous['rust'][row - 1]['cells'], cells['rust'][row - 1]['cells']
        deltas = [tuple(sorted((key, cell[key], other[key]) for key in cell if cell[key] != other[key]))
                  for cell, other in zip(before, after) if cell != other]
        # reverse video off, palette slot 2 on, bold on, palette256 mode set
        assert all(delta == (('bold', False, True), ('fg', -1, 2), ('fm', 0, 33554432),
                             ('inverse', True, False)) for delta in deltas), deltas[:2]
        attribute_deltas['selection_ink'] += len(deltas)
        assert cells['rust'][row - 1]['text'] == previous['rust'][row - 1]['text']
    if case['scenario'] in ('picker-normal', 'picker-filter', 'picker-delete'):
        assert cells['rust'][3]['text'].startswith(MARKER)
        assert cells['python'][3]['text'].startswith(MARKER)
    for side in ('python', 'rust'):
        encoding_modes[side].update(c['fm'] for c in cells[side][4]['cells'] if c['s'])
    changes[ident] = changed
    for image in rendered['images']:
        assert sha(HERE / image['file']) == image['sha256']
        if not changed:
            assert sha(HERE / image['file']) == sha(BASELINE / image['file'])
            unchanged_pngs.append(image['file'])
    for side in ('python', 'rust'):
        record = case[side]
        raw = base64.b64decode(record['raw_base64'], validate=True)
        assert hashlib.sha256(raw).hexdigest() == record['raw_sha256']
        events = b''.join(base64.b64decode(e[1], validate=True) for e in record['events_base64'])
        assert events == raw
        assert (HERE / f'{ident}-{side}.ansi').read_bytes() == raw
        cast = json.loads((HERE / f'{ident}-{side}.cast').read_text().splitlines()[0])
        assert cast['width'] == record['width']
        roundtrips += 1

import check_picker_column_layout  # noqa: E402
import check_picker_counter  # noqa: E402
import check_picker_filter_header  # noqa: E402
import check_picker_filter_hint  # noqa: E402
import check_picker_footer_color  # noqa: E402
import check_picker_footer_position  # noqa: E402
import check_picker_message_style  # noqa: E402
import check_picker_normal_header  # noqa: E402
import check_picker_selection  # noqa: E402
checker_runs = []
for module, sides in [(check_picker_selection, ('python', 'rust')),
                      (check_picker_message_style, ('python', 'rust')),
                      (check_picker_column_layout, ('python', 'rust')),
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

equal_regions = []
for entry in load(HERE / 'region-pixels.json')['results']:
    assert entry['png_sha256'] == sha(HERE / f'{entry["id"]}-bottom.png')
    for region in entry['regions']:
        assert region['equal'] and region['differing_pixels'] == 0, (entry['id'], region)
        equal_regions.append(f'{entry["id"]}:{region["label"]}')
assert len(equal_regions) == 34, len(equal_regions)

audit = {
    'status': 'SELECTION_ROW_FIXED_ALL_PINNED_REGIONS_EQUAL',
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
    'tested_patch': {'sha256': TESTED_PATCH_SHA, 'bytes': 1171},
    'changed_rows_1_based': changes,
    'attribute_deltas': attribute_deltas,
    'unchanged_paired_pngs': sorted(unchanged_pngs),
    'pixel_equal_regions': sorted(equal_regions),
    'pixel_differing_regions': [],
    'still_open_outside_regions': ['status-column ink has no pinned evidence for the Rust `done` value',
                                   'Active/ID column content is a documented adaptation',
                                   'resize, long lists and clear-filter are unproved by these fixed-size fixtures'],
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
                  'differing_regions': [], 'unchanged_pngs': len(unchanged_pngs),
                  'attribute_deltas': attribute_deltas}, indent=2))
