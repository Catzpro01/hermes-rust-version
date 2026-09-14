"""Verify retained provenance, style isolation and normal help-header parity.

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
BASELINE = HERE.parent / 'picker-color-ae220ff'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
load = lambda p: json.loads(p.read_text())

bundle = load(HERE / 'paired-bundle.json')
rust = load(HERE / 'rust-bundle.json')
assert sha(HERE / 'rust-bundle.json') == 'f41d50ce422ee795a1fc7b1c5981b739a808389f97c4309e36df05970842b58f'
assert rust['rust_commit'] == '7fef5140c9242588ddb46749aafcdd7542a559c9'
assert sha(HERE / 'tested.patch') == '90fa6de6e8c260cb783260bafb4bc1502fad98ec6a332fc42c7fa5363c4dfd54'
assert sha(HERE / 'primary-red.json') == '56ecf3af0ea8fc93b369d0f23120277926f6ae2d9b6605c957a8b0cb754d5e7e'
red = load(HERE / 'primary-red.json')
assert red['checkout'] == '4aa085888278a9e6ddb0735ba70e7971895f9de5'
assert red['rust_diff_sha256'] == hashlib.sha256(b'').hexdigest()
for file, field in [('scripts/capture_ui.py', 'capture_driver_sha256'),
                    ('scripts/capture_picker_diagnostic.py', 'capture_script_sha256')]:
    source = subprocess.check_output(['git', 'show', rust['rust_commit'] + ':' + file], cwd=ROOT)
    assert hashlib.sha256(source).hexdigest() == rust[field]
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
    changed = [y+1 for y in range(30) if cells['rust'][y] != previous['rust'][y]]
    normal_header = case['scenario'] in ('picker-normal', 'picker-delete')
    assert changed == ([1] if normal_header else []), (ident, changed)
    if normal_header:
        reference, actual = cells['python'][0], cells['rust'][0]
        assert actual['text'] == reference['text'] and actual['wrapped'] == reference['wrapped']
        for side in ('python', 'rust'):
            encoding_modes[side].update(c['fm'] for c in cells[side][0]['cells'] if c['s'])
        for ref, current, before in zip(reference['cells'], actual['cells'], previous['rust'][0]['cells']):
            assert {k: v for k, v in ref.items() if k != 'fm'} == {k: v for k, v in current.items() if k != 'fm'}
            assert {k: v for k, v in before.items() if k not in ('fg', 'fm', 'bold')} == {k: v for k, v in current.items() if k not in ('fg', 'fm', 'bold')}
            if current['s']:
                assert (current['fg'], current['bold'], current['dim']) == (3, True, False)
                assert ref['fm'] in (16777216, 33554432) and current['fm'] in (16777216, 33554432)
            else:
                assert ref == current == before
    changes[ident] = changed
    for image in rendered['images']:
        assert sha(HERE / image['file']) == image['sha256']
        if not changed:
            assert sha(HERE / image['file']) == sha(BASELINE / image['file'])
            unchanged_pngs.append(image['file'])
    for side in ('python', 'rust'):
        record = case[side]
        raw = base64.b64decode(record['raw_base64'], validate=True)
        assert record['error'] is None and 0 < record['snapshot_end_byte'] <= len(raw)
        assert hashlib.sha256(raw).hexdigest() == record['raw_sha256']
        assert raw == (HERE / f'{ident}-{side}.ansi').read_bytes()
        assert raw == b''.join(base64.b64decode(e[1], validate=True) for e in record['events_base64'])
        cast = [json.loads(line) for line in (HERE / f'{ident}-{side}.cast').read_text().splitlines()]
        assert cast[0]['width'] == record['width'] and cast[0]['height'] == record['height'] == 30
        assert raw == ''.join(e[2] for e in cast[1:] if e[1] == 'o').encode()
        roundtrips += 1
assert encoding_modes == {'python': {16777216}, 'rust': {33554432}}
pixels = load(HERE / 'header-pixels.json')
assert pixels['normalization'] == 'none' and len(pixels['cases']) == 4
assert {c['id'] for c in pixels['cases']} == {c['id'] for c in bundle['cases'] if c['scenario'] in ('picker-normal', 'picker-delete')}
for comparison in pixels['cases']:
    assert comparison['equal'] is True
    assert (comparison['comparison_y'], comparison['comparison_height']) == (31, 19)
    assert comparison['png_sha256'] == sha(HERE / (comparison['id'] + '-bottom.png'))
result = {'status': 'NORMAL_HELP_HEADER_FIXED_NOT_WHOLE_SCREEN_PARITY',
          'source': rust['rust_commit'], 'raw_cast_roundtrips': roundtrips,
          'png_hash_checks': 10, 'python_records_and_cells_unchanged': True,
          'renderer_script_sha256': renderer_sha, 'renderer_settings_match_prior': True,
          'normal_header_palette3_bold_checks': 4, 'header_pixel_comparisons': 4,
          'encoding_modes': {'python': 'palette16', 'rust': 'palette256'},
          'only_normal_header_foreground_and_bold_changed': True,
          'all_other_rows_and_filter_no_match_empty_unchanged': True,
          'unchanged_paired_pngs': unchanged_pngs, 'rust_changed_rows_1_based': changes,
          'normalization': 'none'}
output = HERE / 'audit.json'
if output.exists():
    assert load(output) == result
else:
    output.write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
