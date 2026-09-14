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
assert sha(HERE / 'rust-bundle.json') == 'b82134515d7b59fe40346a38ef136fb6c69c63861c9fbab3aad261f999022db1'
assert rust['rust_commit'] == 'ae220ff1607f122103af1eb006b6fc4e326531e6'
assert sha(HERE / 'tested.patch') == 'ea56c272de0cf151168aee03e85803662f9ffe12bc563de97e6e512e73b941a3'
assert sha(HERE / 'primary-red.json') == 'f14e3920154956dd0d68cf072166b8044ad621b751c0e8e4c6abb6884d977f97'
red = load(HERE / 'primary-red.json')
assert red['checkout'] == '5515789a3d2285fbb25773b63a8e524992740214'
assert red['rust_diff_sha256'] == hashlib.sha256(b'').hexdigest()
origin = bundle['python_origin']
parent = ROOT / origin['bundle_path']
assert sha(parent) == origin['bundle_sha256']
references = {c['id']: c for c in load(parent)['cases']}
rust_cases = {c['id']: c for c in rust['cases']}
renderer = load(HERE / 'renderer.json')
prior_renderer = load(EVIDENCE / 'picker-position-0c0704d/renderer.json')
assert renderer['input_sha256'] == sha(HERE / 'paired-bundle.json')
for key in ('browser', 'xterm', 'font', 'fontSize', 'deviceScaleFactor', 'normalization', 'fonts'):
    assert renderer[key] == prior_renderer[key]
renderer_sha = sha(ROOT / 'scripts/visual-renderer/render.cjs')
assert renderer_sha == '201091eef5007e9a45889da7b4725b658ce39df1bf977a48677e435df9f9c074'
assert len(bundle['cases']) == len(renderer['cases']) == 10
changes = {}
encoding_modes = {"python": set(), "rust": set()}
unchanged_pngs = []
roundtrips = 0
for case, rendered in zip(bundle['cases'], renderer['cases']):
    ident = case['id']
    assert ident == rendered['id']
    assert case['python'] == references[ident]['python']
    assert case['rust'] == rust_cases[ident]['rust']
    baseline = EVIDENCE / 'picker-position-0c0704d'
    previous = json.loads(gzip.decompress((baseline / f'{ident}-cells.json.gz').read_bytes()))
    cells = json.loads(gzip.decompress((HERE / f'{ident}-cells.json.gz').read_bytes()))
    assert cells['python'] == previous['python']
    assert len(cells['rust']) == len(previous['rust']) == 30
    changed = [y+1 for y in range(30) if cells['rust'][y] != previous['rust'][y]]
    is_footer = case['scenario'] in ('picker-normal', 'picker-filter', 'picker-no-match')
    assert changed == ([30] if is_footer else []), (ident, changed)
    if is_footer:
        # Both palette16 and palette256 select slot8; truecolor is NOT accepted.
        # Keep the encoding-mode distinction in evidence, never rewrite it.
        reference, actual = cells['python'][29], cells['rust'][29]
        for side in ('python', 'rust'):
            encoding_modes[side].update(c['fm'] for c in cells[side][29]['cells'] if c['s'])
        assert actual['text'] == reference['text'] and actual['wrapped'] == reference['wrapped']
        for ref, current in zip(reference['cells'], actual['cells']):
            assert {k: v for k, v in ref.items() if k != 'fm'} == {k: v for k, v in current.items() if k != 'fm'}, ident
            if current['s']:
                assert ref['fm'] in (16777216, 33554432)
                assert current['fm'] in (16777216, 33554432)
            else:
                assert ref == current
        for before, after in zip(previous['rust'][29]['cells'], cells['rust'][29]['cells']):
            assert {k: v for k, v in before.items() if k not in ('fg', 'fm')} == {k: v for k, v in after.items() if k not in ('fg', 'fm')}
            if before['s']:
                assert (after['fg'], after['dim']) == (8, False)
                assert after['fm'] in (16777216, 33554432)
            else:
                assert after == before
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
assert encoding_modes == {'python': {16777216}, 'rust': {33554432}}
pixels = load(HERE / 'footer-pixels.json')
assert pixels['normalization'] == 'none' and len(pixels['cases']) == 6
assert {c['id'] for c in pixels['cases']} == {c['id'] for c in bundle['cases'] if c['scenario'] in ('picker-normal', 'picker-filter', 'picker-no-match')}
for comparison in pixels['cases']:
    assert comparison['equal'] is True
    assert comparison['png_sha256'] == sha(HERE / (comparison['id'] + '-bottom.png'))
result = {'status': 'FOOTER_COLOR_FIXED_NOT_WHOLE_SCREEN_PARITY',
          'source': rust['rust_commit'], 'raw_cast_roundtrips': roundtrips,
          'png_hash_checks': 10, 'python_records_and_cells_unchanged': True,
          'renderer_script_sha256': renderer_sha, 'renderer_settings_match_prior': True,
          'footer_rows_same_content_attributes_and_palette_index': 6,
          'encoding_modes': {'python': 'palette16', 'rust': 'palette256'},
          'footer_lower_half_pixel_comparisons': 6, 'only_footer_foreground_changed': True,
          'all_other_rows_and_delete_empty_unchanged': True,
          'unchanged_paired_pngs': unchanged_pngs, 'rust_cell_changes': changes,
          'normalization': 'none'}
output = HERE / 'audit.json'
if output.exists():
    assert load(output) == result
else:
    output.write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
