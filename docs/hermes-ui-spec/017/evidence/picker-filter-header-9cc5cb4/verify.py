"""Verify retained provenance, palette-slot correctness and filter-header parity.

No originals are rewritten. Existing audit.json must agree exactly. Pixel
comparison is independently reproducible via verify-pixels.cjs; direct image
inspection is documented in REPORT.md, not claimed by this script.
"""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
BASELINE = HERE.parent / 'picker-header-7fef514'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
load = lambda p: json.loads(p.read_text())

bundle = load(HERE / 'paired-bundle.json')
rust = load(HERE / 'rust-bundle.json')
assert sha(HERE / 'rust-bundle.json') == 'a9dd557366f2f391df1daaff4ab75fb88b26c0f034ab72c30ddf646c339dd85c'
assert rust['rust_commit'] == '9cc5cb4c416df66f43cd30c33adf8055b82269ab'
assert sha(HERE / 'tested.patch') == '21fcdead245179fb4eda6032fd39f5f5593b3ae8c4c5efe01b78bb676ef2a6d1'
assert (HERE / 'tested.patch').stat().st_size == 598
for file, field in [('scripts/capture_ui.py', 'capture_driver_sha256'),
                    ('scripts/capture_picker_diagnostic.py', 'capture_script_sha256')]:
    source = subprocess.check_output(['git', 'show', rust['rust_commit'] + ':' + file], cwd=ROOT)
    assert hashlib.sha256(source).hexdigest() == rust[field]
# The captured commit differs from the fix commit only by the diagnostic request.
diff = subprocess.check_output(
    ['git', 'diff', '--name-only', '48cf58715f2164c53cbab115df34e7cbe3c47fbc', rust['rust_commit']],
    cwd=ROOT, text=True).split()
assert diff == ['.scratch/hermes-rs-total-parity/diagnostics/picker/request.json'], diff
origin = bundle['python_origin']
parent = ROOT / origin['bundle_path']
assert sha(parent) == origin['bundle_sha256']
references = {c['id']: c for c in load(parent)['cases']}
rust_cases = {c['id']: c for c in rust['cases']}
renderer = load(HERE / 'renderer.json')
prior_renderer = load(BASELINE / 'renderer.json')
assert renderer['input_sha256'] == sha(HERE / 'paired-bundle.json')
for key in ('browser', 'xterm', 'font', 'fontSize', 'deviceScaleFactor', 'normalization', 'fonts'):
    assert renderer[key] == prior_renderer[key]
renderer_sha = sha(ROOT / 'scripts/visual-renderer/render.cjs')
assert renderer_sha == '201091eef5007e9a45889da7b4725b658ce39df1bf977a48677e435df9f9c074'
assert len(bundle['cases']) == len(renderer['cases']) == 10
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
    filter_header = case['scenario'] in ('picker-filter', 'picker-no-match')
    assert changed == ([1] if filter_header else []), (ident, changed)
    if filter_header:
        reference, actual = cells['python'][0], cells['rust'][0]
        assert actual['text'] == reference['text'] and actual['wrapped'] == reference['wrapped']
        for side in ('python', 'rust'):
            encoding_modes[side].update(c['fm'] for c in cells[side][0]['cells'] if c['s'])
        for ref, current, before in zip(reference['cells'], actual['cells'], previous['rust'][0]['cells']):
            assert {k: v for k, v in ref.items() if k != 'fm'} == {k: v for k, v in current.items() if k != 'fm'}
            assert {k: v for k, v in before.items() if k not in ('fg', 'fm', 'bold')} == {k: v for k, v in current.items() if k not in ('fg', 'fm', 'bold')}
            if current['s']:
                assert (current['fg'], current['bold'], current['dim']) == (6, True, False)
                assert ref['fm'] == 16777216 and current['fm'] == 33554432
            else:
                assert ref == current == before
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
pixels = load(HERE / 'filter-header-pixels.json')
assert pixels['normalization'] == 'none'
assert len(pixels['cases']) == 4
for entry in pixels['cases']:
    assert entry['equal'] and entry['differing_pixels'] == 0
    assert entry['png_sha256'] == sha(HERE / f'{entry["id"]}-bottom.png')
    assert entry['id'].startswith(('picker-filter', 'picker-no-match'))
    assert sha(HERE / f'{entry["id"]}-bottom.png') != sha(BASELINE / f'{entry["id"]}-bottom.png')
audit = {
    'status': 'FILTER_HELP_HEADER_FIXED_NOT_WHOLE_SCREEN_PARITY',
    'source': rust['rust_commit'],
    'fix_commit': '48cf58715f2164c53cbab115df34e7cbe3c47fbc',
    'raw_cast_roundtrips': roundtrips,
    'png_hash_checks': 10,
    'python_records_and_cells_unchanged': True,
    'renderer_script_sha256': renderer_sha,
    'renderer_settings_match_prior': True,
    'filter_header_palette6_bold_checks': 4,
    'filter_header_pixel_comparisons': 4,
    'encoding_modes': {'python': 'palette16', 'rust': 'palette256'},
    'rejected_attempt_recorded': 'first-attempt-bright-variant.txt',
    'only_filter_header_foreground_and_bold_changed': True,
    'all_other_rows_and_cases_unchanged': True,
    'unchanged_paired_pngs': sorted(unchanged_pngs),
    'rust_changed_rows_1_based': changes,
    'normalization': 'none',
}
current = HERE / 'audit.json'
text = json.dumps(audit, indent=2) + '\n'
if current.exists():
    assert current.read_text() == text, 'Existing audit disagrees'
else:
    current.write_text(text)
print(json.dumps({'status': audit['status'], 'roundtrips': roundtrips,
                  'pixel_cases': len(pixels['cases']), 'unchanged_pngs': len(unchanged_pngs)}, indent=2))
