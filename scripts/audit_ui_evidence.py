"""Read-only audit of a retained remaining-UI packet; no normalization/writes.

Usage: python3 scripts/audit_ui_evidence.py PACKET_DIRECTORY
The packet uses paired-bundle.json and the pinned renderer's raw/cast/cell files.
Image inspection and adaptation decisions remain explicit human/agent review.
"""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import sys

from capture_ui import validate_bundle


def audit(root):
    bundle = json.loads((root / 'paired-bundle.json').read_text())
    validate_bundle(bundle)
    assert bundle['rust_worktree_diff_sha256'] == hashlib.sha256(b'').hexdigest()
    renderer = json.loads((root / 'renderer.json').read_text())
    assert renderer['input_sha256'] == hashlib.sha256((root / 'paired-bundle.json').read_bytes()).hexdigest()
    assert {c['id'] for c in renderer['cases']} == {c['id'] for c in bundle['cases']}
    for rendered in renderer['cases']:
        assert len(rendered['images']) == 1
        image = rendered['images'][0]
        assert image['file'] == rendered['id'] + '-bottom.png'
        assert hashlib.sha256((root / image['file']).read_bytes()).hexdigest() == image['sha256']
    results = []
    for case in bundle['cases']:
        name = case['id']
        for side in ('python', 'rust'):
            stream = case[side]
            raw = base64.b64decode(stream['raw_base64'], validate=True)
            assert (root / f'{name}-{side}.ansi').read_bytes() == raw
            cast = [json.loads(line) for line in (root / f'{name}-{side}.cast').read_text().splitlines()]
            assert ''.join(e[2] for e in cast[1:]).encode() == raw
            assert (cast[0]['width'], cast[0]['height']) == (stream['width'], stream['height'])
        cells = json.loads(gzip.decompress((root / f'{name}-cells.json.gz').read_bytes()))
        glyph_diffs, attrs, background = [], 0, 0
        py, rs = cells['python'], cells['rust']
        for y in range(max(len(py), len(rs))):
            a = py[y] if y < len(py) else {'text': '', 'cells': []}
            b = rs[y] if y < len(rs) else {'text': '', 'cells': []}
            title = 'Hermes Agent' in a['text'] and 'Hermes-RS' in b['text']
            glyphs = lambda row: [(x, c['s']) for x, c in enumerate(row['cells']) if c['s'] not in ('', ' ')]
            if not title and glyphs(a) != glyphs(b):
                glyph_diffs.append(y+1)
            for ca, cb in zip(a['cells'], b['cells']):
                if ca['s'] == cb['s'] and ca['s'].strip() and ca != cb:
                    attrs += 1
                if any(ca[k] != cb[k] for k in ('bg', 'bm', 'inverse', 'underline')):
                    background += 1
        info = {'id': name, 'rows': {s: len(cells[s]) for s in cells},
                'non_branding_glyph_difference_rows_1_based': glyph_diffs,
                'same_coordinate_same_glyph_attribute_differences': attrs,
                'background_inverse_underline_differences': background}
        if case['scenario'].startswith('summary'):
            label = '0 tools' if case['scenario'] == 'summary-zero' else '3 tools'
            info['summary'] = {s: [{'row': y+1, 'column': row['text'].index(label)+1,
                                    'text': row['text'][row['text'].index(label):].rstrip(' │')}
                                   for y, row in enumerate(cells[s]) if label in row['text']]
                               for s in cells}
            assert all(len(v) == 1 for v in info['summary'].values()), 'summary not rendered exactly once'
        results.append(info)
    return {'status': 'AUDITED_NOT_ACCEPTED', 'normalization': 'none',
            'raw_cast_roundtrips': 2*len(results), 'cases': results,
            'boundary': 'Glyph comparison excludes space/empty cells and explicit title branding only. Metrics do not decide adaptation acceptance or whole-screen parity.'}


if __name__ == '__main__':
    print(json.dumps(audit(Path(sys.argv[1])), indent=2, ensure_ascii=False))
