"""Verify retained provenance, pinned-renderer pixels and prompt/message parity.

No originals are rewritten and nothing is normalized. The script re-derives the
audit from the checked-in artifacts: bundle/patch hashes, the request-only
capture commit, reused Python records, the per-case changed-row map against the
previous packet, attribute-level cell deltas, the live checkers re-run over the
paired bundle, raw/cast round trips, the dim windows read from the retained
stream (pyte cannot decode dim), PNG hashes and the pinned-renderer region
comparisons. Direct image inspection is documented in REPORT.md, not claimed
here.
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
BASELINE = HERE.parent / 'picker-column-layout-b732d22'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
load = lambda p: json.loads(Path(p).read_text())
sys.path.insert(0, str(ROOT / 'scripts'))

FIX_COMMIT = '549fc8d2b52eb3212cdf44c127fa910a60257c39'
CAPTURE_COMMIT = 'fd674dc3dbc0452dde75794972095f20bf277c78'
RUST_BUNDLE_SHA = 'a8f37c4613cf585cb93c15220636e24ca46d810704ecdfb614b9f1a3596b78a2'
TESTED_PATCH_SHA = 'd1ed7f9962dcd45a306f1c3bbafdcf9b3c820eadbb90a39822d917078862ab07'
RENDERER_SHA = '201091eef5007e9a45889da7b4725b658ce39df1bf977a48677e435df9f9c074'
PROMPT = "  Delete session 'second topic'? [y/N]"
# Rows whose decoded cells changed against the previous packet. The no-match row
# changes only its dim flag (pyte cannot see dim; the renderer decoder can), the
# prompt row only its foreground slot and bold flag.
EXPECTED_CHANGED = {
    'picker-normal-100x30': [], 'picker-filter-100x30': [], 'picker-no-match-100x30': [4],
    'picker-delete-100x30': [30], 'picker-empty-100x30': [],
    'picker-normal-80x30': [], 'picker-filter-80x30': [], 'picker-no-match-80x30': [4],
    'picker-delete-80x30': [30], 'picker-empty-80x30': [],
}

bundle = load(HERE / 'paired-bundle.json')
rust = load(HERE / 'rust-bundle.json')
assert sha(HERE / 'rust-bundle.json') == RUST_BUNDLE_SHA
assert rust['rust_commit'] == CAPTURE_COMMIT
assert len(rust['cases']) == 10
assert sha(HERE / 'tested.patch') == TESTED_PATCH_SHA
assert (HERE / 'tested.patch').stat().st_size == 2216
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
attribute_deltas = {'dim': 0, 'ink': 0}
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
        if case['scenario'] == 'picker-no-match':
            assert all(delta == (('dim', False, True),) for delta in deltas), deltas[:2]
            attribute_deltas['dim'] += len(deltas)
        else:
            # Rust writes palette slot 1 through its 256-colour form (fg mode
            # 33554432), exactly like the palette3/6/8 slices already shipped.
            assert all(delta == (('bold', False, True), ('fg', -1, 1), ('fm', 0, 33554432))
                       for delta in deltas), deltas[:2]
            attribute_deltas['ink'] += len(deltas)
    if case['scenario'] == 'picker-no-match':
        assert cells['rust'][3]['text'] == cells['python'][3]['text'] == \
            '  No sessions match the filter.'
    if case['scenario'] == 'picker-delete':
        assert cells['rust'][29]['text'] == cells['python'][29]['text'] == PROMPT
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
        cast = json.loads((HERE / f'{ident}-{side}.cast').read_text().splitlines()[0])
        assert cast['width'] == record['width']
        roundtrips += 1

# The live checkers re-run over the same paired bundle: the new gate must pass on
# both sides, the older gates on the Rust capture (the reference side of those
# gates was already verified by their own packets).
import check_picker_column_layout  # noqa: E402
import check_picker_counter  # noqa: E402
import check_picker_filter_header  # noqa: E402
import check_picker_filter_hint  # noqa: E402
import check_picker_footer_color  # noqa: E402
import check_picker_footer_position  # noqa: E402
import check_picker_message_style  # noqa: E402
import check_picker_normal_header  # noqa: E402
checker_runs = []
for module, sides in [(check_picker_message_style, ('python', 'rust')),
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

# Dim cannot be decoded by pyte, so the retained byte streams are read directly:
# the Rust capture opens dim on row 4 column 1 over the message in place, the
# reference opens it there too and closes it at the footer redraw.
dim_windows = {}
for case in bundle['cases']:
    if case['scenario'] != 'picker-no-match':
        continue
    for side in ('python', 'rust'):
        windows = check_picker_message_style.dim_windows(case[side])
        assert windows, (case['id'], side)
        assert all((w['on']['row'], w['on']['col']) == (4, 1) for w in windows)
        assert windows[0]['text'].rstrip() == '  No sessions match the filter.'
    dim_windows[case['id']] = {'python': len(check_picker_message_style.dim_windows(case['python'])),
                               'rust': len(check_picker_message_style.dim_windows(case['rust']))}

# Pinned-renderer regions: every retained region must now be pixel-identical.
equal_regions = []
for entry in load(HERE / 'region-pixels.json')['results']:
    assert entry['png_sha256'] == sha(HERE / f'{entry["id"]}-bottom.png')
    for region in entry['regions']:
        assert region['equal'] and region['differing_pixels'] == 0, (entry['id'], region)
        equal_regions.append(f'{entry["id"]}:{region["label"]}')

audit = {
    'status': 'MESSAGE_STYLE_FIXED_ALL_PINNED_REGIONS_EQUAL',
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
    'tested_patch': {'sha256': TESTED_PATCH_SHA, 'bytes': 2216},
    'changed_rows_1_based': changes,
    'attribute_deltas': attribute_deltas,
    'dim_windows_per_case': dim_windows,
    'unchanged_paired_pngs': sorted(unchanged_pngs),
    'pixel_equal_regions': sorted(equal_regions),
    'pixel_differing_regions': [],
    'still_open_outside_regions': ['cursor-row palette2 green + bold (reverse video today)',
                                   'status-column ink has no pinned evidence for the Rust `done` value'],
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
                  'attribute_deltas': attribute_deltas,
                  'dim_windows_per_case': dim_windows}, indent=2))
