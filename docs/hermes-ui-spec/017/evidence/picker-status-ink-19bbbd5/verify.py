"""Verify retained provenance, the status-tag ink fix and the pinned pixel regions.

No originals are rewritten and nothing is normalized. The script re-derives the
audit from the checked-in artifacts: bundle/patch hashes, the capture commit's
tooling provenance, the reused Python records, the per-case changed-row map
against the previous packet, attribute-level cell deltas, the live checkers
re-run over the paired bundle, raw/cast round trips, PNG hashes, the unchanged
control regions and the declared tag-span difference. Direct image inspection is
documented in REPORT.md, not claimed here.
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
BASELINE = HERE.parent / 'picker-selection-b4cb408'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
load = lambda p: json.loads(Path(p).read_text())
sys.path.insert(0, str(ROOT / 'scripts'))

FIX_COMMIT = '1781404'
CAPTURE_COMMIT = '19bbbd5c3309efc50b4d9af0ee02641193f59c92'
RUST_BUNDLE_SHA = '119e299b47dbe6878162ba394e686a2619a95bec70935e1246fdc883f6c24d30'
TESTED_PATCH_SHA = '73d0322e913ecf5b80493f4f6725eb802ae3542b0912bf9dc40a8bb55312ffed'
REQUEST_PATCH_SHA = 'e88347c88d89f19dcc48f97d1ff7ec46dc9420c07875d076c91f7e44a2fd56fb'
RENDERER_SHA = '201091eef5007e9a45889da7b4725b658ce39df1bf977a48677e435df9f9c074'
# Only the unselected row changed, at both widths, in the two scenarios that have
# an unselected row; the filter screen shows a single (cursor) row, and the
# no-match and empty screens have no body row at all.
EXPECTED_CHANGED = {
    'picker-normal-100x30': [5], 'picker-filter-100x30': [], 'picker-delete-100x30': [5],
    'picker-no-match-100x30': [], 'picker-empty-100x30': [],
    'picker-normal-80x30': [5], 'picker-filter-80x30': [], 'picker-delete-80x30': [5],
    'picker-no-match-80x30': [], 'picker-empty-80x30': [],
}
# Each changed cell is the same attribute pair: palette slot 2 on, palette256 mode
# set. The tag is not bold and not inverse.
TAG_DELTA = (('fg', -1, 2), ('fm', 0, 33554432))
TAG_PIXELS_PER_CELL_RUN = 270

bundle = load(HERE / 'paired-bundle.json')
rust = load(HERE / 'rust-bundle.json')
assert sha(HERE / 'rust-bundle.json') == RUST_BUNDLE_SHA
assert rust['rust_commit'] == CAPTURE_COMMIT
assert len(rust['cases']) == 10
assert sha(HERE / 'tested.patch') == TESTED_PATCH_SHA
assert (HERE / 'tested.patch').stat().st_size == 6109
assert sha(HERE / 'request.patch') == REQUEST_PATCH_SHA
assert (HERE / 'request.patch').stat().st_size == 6007
for file, field in [('scripts/capture_ui.py', 'capture_driver_sha256'),
                    ('scripts/capture_picker_diagnostic.py', 'capture_script_sha256')]:
    source = subprocess.check_output(['git', 'show', rust['rust_commit'] + ':' + file], cwd=ROOT)
    assert hashlib.sha256(source).hexdigest() == rust[field], file
# The capture commit may carry tooling-only changes, but no Rust source may differ
# from the commit whose tested patch was applied.
assert subprocess.check_output(['git', 'diff', '--name-only', FIX_COMMIT, rust['rust_commit'],
                                '--', '*.rs'], cwd=ROOT, text=True).strip() == ''
# ... and the applied source carries the pinned mapping, so the tag ink cannot be
# claimed by a comment alone.
text = subprocess.check_output(['git', 'show', FIX_COMMIT + ':crates/hermes-cli/src/session_picker.rs'],
                               cwd=ROOT, text=True)
for needle in ['pub fn status_ink(status: &str) -> Color',
               '"done" => Color::DarkGreen', '"intr" => Color::DarkYellow',
               '"err" => Color::DarkRed', '"empty" => Color::DarkGrey',
               '_ => Color::Reset', 'pub fn status_tag_span(name_width: usize)',
               'let start = 3 + name_width + 2', 'if n < 3']:
    assert needle in text, needle
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
attribute_deltas = {'status_tag_ink': 0}
changed_cells = 0
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
        # palette slot 2 on, palette256 mode set; the tag text and the rest of the
        # row are byte-identical to the previous packet.
        assert len(deltas) == 5, deltas
        assert all(delta == TAG_DELTA for delta in deltas), deltas[:1]
        attribute_deltas['status_tag_ink'] += len(deltas)
        changed_cells += len(deltas)
        assert cells['rust'][row - 1]['text'] == previous['rust'][row - 1]['text']
    if case['scenario'] in ('picker-normal', 'picker-delete'):
        # The tag span is inside the header row's own alignment, so the row above
        # must be untouched, and the tag must stay left-aligned in its five cells.
        assert cells['rust'][4]['text'] == previous['rust'][4]['text']
        tag = cells['rust'][4]['text'][43 if case['rust']['width'] == 100 else 25:][:5]
        assert tag.strip() == 'done', (ident, tag)
    if case['scenario'] == 'picker-filter':
        # Only the cursor row exists; its whole-row palette2 + bold paint is the
        # selection cycle's contract and must not have moved.
        assert cells['rust'][3]['text'].startswith(' → ')
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
import check_picker_status_ink  # noqa: E402
checker_runs = []
for module, sides in [(check_picker_status_ink, ('python', 'rust')),
                      (check_picker_selection, ('python', 'rust')),
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

# The tag span itself is a declared difference, not a parity region: the tag word
# is fixture data (the retained Python fixtures carry an interrupted session, the
# Rust fixtures a complete one), so its glyphs cannot match while the ink is
# pinned by check_picker_status_ink.py on both sides.
declared = []
for entry in load(HERE / 'region-pixels-tag.json')['results']:
    assert entry['png_sha256'] == sha(HERE / f'{entry["id"]}-bottom.png')
    for region in entry['regions']:
        assert region['row'] == 5 and region['colEnd'] - region['colStart'] == 5, region
        columns = next(e['columns'] for e in load(HERE / 'region-spec-tag.json')['cases']
                       if e['id'] == entry['id'])
        assert region['colStart'] == max(20, columns - 62) + 5, (entry['id'], region)
        assert region['differing_pixels'] == TAG_PIXELS_PER_CELL_RUN, (entry['id'], region)
        declared.append(f'{entry["id"]}:{region["label"]}')
assert len(declared) == 4, len(declared)
tag_span_sources = [c['id'] for c in load(HERE / 'region-spec-tag.json')['cases']]
assert tag_span_sources == ['picker-normal-100x30', 'picker-delete-100x30',
                            'picker-normal-80x30', 'picker-delete-80x30']

audit = {
    'status': 'STATUS_TAG_INK_FIXED_ALL_CONTROL_REGIONS_EQUAL',
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
    'tested_patch': {'sha256': TESTED_PATCH_SHA, 'bytes': 6109},
    'request_patch': {'sha256': REQUEST_PATCH_SHA, 'bytes': 6007,
                      'note': 'bound request patch; tested.patch is the same diff after the '
                              'runner\'s cargo fmt --all'},
    'changed_rows_1_based': changes,
    'status_tag_cells_changed': changed_cells,
    'attribute_deltas': attribute_deltas,
    'unchanged_paired_pngs': sorted(unchanged_pngs),
    'pixel_equal_regions': sorted(equal_regions),
    'pixel_differing_regions': [],
    'declared_differing_regions': sorted(declared),
    'declared_difference_reason': 'status tag word is fixture data (retained Python session is '
                                  'interrupted, Rust fixture complete); the ink at this span is '
                                  'pinned by check_picker_status_ink.py on both sides and by the '
                                  'unit tests for the other three mappings',
    'still_open_outside_regions': ['the live capture exercises only the complete->done mapping; '
                                   'interrupted/error/empty inks are pinned by the upstream '
                                   'source, the unit tests and the checker fixtures',
                                   'Active/ID column content is a documented adaptation',
                                   'resize, long lists and clear-filter are unproved by these '
                                   'fixed-size fixtures',
                                   'the picker repaints every poll timeout instead of only after '
                                   'a key; queued as the next picker cycle'],
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
                  'differing_regions': [], 'declared_differing_regions': len(declared),
                  'unchanged_pngs': len(unchanged_pngs),
                  'status_tag_cells_changed': changed_cells,
                  'attribute_deltas': attribute_deltas}, indent=2))
