"""Recheck packet provenance, raw/cast roundtrips and bounded cell differences.

Direct image inspection is documented separately in REPORT.md, not automated here.
This verifier never modifies originals; existing audit.json must agree exactly.
"""
import base64
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
EVIDENCE = HERE.parent
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
load = lambda p: json.loads(p.read_text())

bundle = load(HERE / 'paired-bundle.json')
rust = load(HERE / 'rust-bundle.json')
assert sha(HERE / 'rust-bundle.json') == '5ab4a98ea7d1cc38b8977e76994b7a885290d04d5bf95407d0ba267fba9d900f'
assert rust['rust_commit'] == '0c0704d08d367444e780a0951506ba28ce125b2d'
assert sha(HERE / 'tested.patch') == 'e7ad21d2a7baab771e3b0a5742d5d741720302c1ed082e70448911406208fede'
assert sha(HERE / 'primary-red.json') == 'fc71b1f359b1ea7d40ee06e359b26d8a3308340db9298dfd1163b8aa2c6c43c7'
red = load(HERE / 'primary-red.json')
assert red['checkout'] == 'eedd9a0fb639a92e43b587b4874e04ab0e93e787'
assert red['rust_diff_sha256'] == hashlib.sha256(b'').hexdigest()
origin = bundle['python_origin']
parent = ROOT / origin['bundle_path']
assert sha(parent) == origin['bundle_sha256']
references = {c['id']: c for c in load(parent)['cases']}
rust_cases = {c['id']: c for c in rust['cases']}
renderer = load(HERE / 'renderer.json')
prior_renderer = load(EVIDENCE / 'picker-counter-a8d5e8c/renderer.json')
assert renderer['input_sha256'] == sha(HERE / 'paired-bundle.json')
for key in ('browser', 'xterm', 'font', 'fontSize', 'deviceScaleFactor', 'normalization', 'fonts'):
    assert renderer[key] == prior_renderer[key]
renderer_sha = sha(ROOT / 'scripts/visual-renderer/render.cjs')
assert renderer_sha == '201091eef5007e9a45889da7b4725b658ce39df1bf977a48677e435df9f9c074'
assert len(bundle['cases']) == len(renderer['cases']) == 10
changes = {}
unchanged_pngs = []
roundtrips = 0
for case, rendered in zip(bundle['cases'], renderer['cases']):
    ident = case['id']
    assert ident == rendered['id']
    assert case['python'] == references[ident]['python']
    assert case['rust'] == rust_cases[ident]['rust']
    baseline = EVIDENCE / ('ui-3b39bd7' if case['scenario'] in ('picker-empty', 'picker-delete') else 'picker-counter-a8d5e8c')
    previous = json.loads(gzip.decompress((baseline / f'{ident}-cells.json.gz').read_bytes()))
    cells = json.loads(gzip.decompress((HERE / f'{ident}-cells.json.gz').read_bytes()))
    assert cells['python'] == previous['python']
    assert len(cells['rust']) == len(previous['rust']) == 30
    changed = [y+1 for y in range(30) if cells['rust'][y] != previous['rust'][y]]
    expected = {'picker-normal': [5, 30], 'picker-filter': [4, 30],
                'picker-no-match': [4, 30], 'picker-delete': [5], 'picker-empty': []}[case['scenario']]
    assert changed == expected, (ident, changed)
    if case['scenario'] in ('picker-normal', 'picker-filter', 'picker-no-match'):
        # Footer moves intact, including all cell attributes; no palette fix.
        assert cells['rust'][29] == previous['rust'][expected[0]-1]
    changes[ident] = {'baseline': baseline.name, 'rows_1_based': changed}
    for image in rendered['images']:
        assert sha(HERE / image['file']) == image['sha256']
        if not changed:
            assert sha(HERE / image['file']) == sha(baseline / image['file'])
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
result = {'status': 'FOOTER_POSITION_FIXED_NOT_WHOLE_SCREEN_PARITY',
          'source': rust['rust_commit'], 'raw_cast_roundtrips': roundtrips,
          'png_hash_checks': 10, 'python_records_and_cells_unchanged': True,
          'renderer_script_sha256': renderer_sha, 'renderer_settings_match_prior': True,
          'footer_cells_moved_without_style_change': True,
          'unchanged_paired_pngs': unchanged_pngs, 'rust_cell_changes': changes,
          'normalization': 'none'}
output = HERE / 'audit.json'
if output.exists():
    assert load(output) == result
else:
    output.write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
