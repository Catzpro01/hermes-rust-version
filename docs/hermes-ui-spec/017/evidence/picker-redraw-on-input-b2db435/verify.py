"""Verify the redraw-on-input fix, the pinned regions and the unchanged frame content.

No originals are rewritten and nothing is normalized. The script re-derives the
audit from the checked-in artifacts: bundle/patch hashes, the capture commit's
tooling provenance, the reused Python records, the redraw cadence on both the
previous and the current capture, the byte streams (which shrank because the
repaints are gone), the ten live checkers, raw/cast round trips, PNG hashes and
the pinned pixel regions. Direct image inspection is documented in REPORT.md,
not claimed here.
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
BASELINE = HERE.parent / 'picker-status-ink-19bbbd5'
PARENT_PYTHON = ROOT / 'docs/hermes-ui-spec/017/evidence/ui-3b39bd7/python-bundle.json'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
load = lambda p: json.loads(Path(p).read_text())
sys.path.insert(0, str(ROOT / 'scripts'))

FIX_COMMIT = 'bbd943c'
CAPTURE_COMMIT = 'b2db435f6071d9baab13d7347b7452c9e38ffef0'
RUST_BUNDLE_SHA = '491713dd563bbb322ae2f4ca043637a93cb5070a71336632097abb0b0b4d0083'
TESTED_PATCH_SHA = '925b963e0d3385109cbffa74527798c7d173d1439184cab3602c82b05d8b6a8b'
RENDERER_SHA = '201091eef5007e9a45889da7b4725b658ce39df1bf977a48677e435df9f9c074'

bundle = load(HERE / 'paired-bundle.json')
rust = load(HERE / 'rust-bundle.json')
assert sha(HERE / 'rust-bundle.json') == RUST_BUNDLE_SHA
assert rust['rust_commit'] == CAPTURE_COMMIT
assert len(rust['cases']) == 10
assert sha(HERE / 'tested.patch') == TESTED_PATCH_SHA
assert (HERE / 'tested.patch').stat().st_size == 2801
for file, field in [('scripts/capture_ui.py', 'capture_driver_sha256'),
                    ('scripts/capture_picker_diagnostic.py', 'capture_script_sha256')]:
    source = subprocess.check_output(['git', 'show', rust['rust_commit'] + ':' + file], cwd=ROOT)
    assert hashlib.sha256(source).hexdigest() == rust[field], file
assert subprocess.check_output(['git', 'diff', '--name-only', FIX_COMMIT, rust['rust_commit'],
                                '--', '*.rs'], cwd=ROOT, text=True).strip() == ''
text = subprocess.check_output(['git', 'show', FIX_COMMIT + ':crates/hermes-cli/src/session_picker.rs'],
                               cwd=ROOT, text=True)
for needle in ['let mut dirty = true;', 'if dirty {', 'dirty = false;',
               'Event::Resize(_, _) => {\n                dirty = true;']:
    assert needle in text, needle
# The applied patch must be exactly the tested one.
assert subprocess.check_output(['git', 'diff', FIX_COMMIT, rust['rust_commit'], '--',
                                'crates/hermes-cli/src/session_picker.rs'], cwd=ROOT,
                               text=True).strip() == ''
origin = bundle['python_origin']
parent = ROOT / origin['bundle_path']
assert sha(parent) == origin['bundle_sha256'] == sha(PARENT_PYTHON)
references = {c['id']: c for c in load(parent)['cases']}
rust_cases = {c['id']: c for c in bundle['cases']}
renderer = load(HERE / 'renderer.json')
prior_renderer = load(BASELINE / 'renderer.json')
assert renderer['input_sha256'] == sha(HERE / 'paired-bundle.json')
for key in ('browser', 'xterm', 'font', 'fontSize', 'deviceScaleFactor', 'normalization', 'fonts'):
    assert renderer[key] == prior_renderer[key]
renderer_sha = sha(ROOT / 'scripts/visual-renderer/render.cjs')
assert renderer_sha == RENDERER_SHA

import check_picker_column_layout  # noqa: E402
import check_picker_counter  # noqa: E402
import check_picker_filter_header  # noqa: E402
import check_picker_filter_hint  # noqa: E402
import check_picker_footer_color  # noqa: E402
import check_picker_footer_position  # noqa: E402
import check_picker_message_style  # noqa: E402
import check_picker_normal_header  # noqa: E402
import check_picker_redraw_on_input  # noqa: E402
import check_picker_selection  # noqa: E402
import check_picker_status_ink  # noqa: E402

# The gate must reject the previous capture (the behaviour this cycle fixes) and
# accept the new one: the same checker, three states apart.
previous_bundle = load(BASELINE / 'paired-bundle.json')
with contextlib.redirect_stdout(io.StringIO()) as rejected:
    previous_failures = check_picker_redraw_on_input.check(previous_bundle, 'rust')
assert previous_failures, 'the gate no longer rejects the repainting capture'
assert len(rejected.getvalue().splitlines()) == 10, rejected.getvalue()
with contextlib.redirect_stdout(io.StringIO()):
    assert check_picker_redraw_on_input.check(previous_bundle, 'python') == []

checker_runs = []
for module, sides in [(check_picker_redraw_on_input, ('python', 'rust')),
                      (check_picker_status_ink, ('python', 'rust')),
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

def windows(record, scenario):
    return check_picker_redraw_on_input.windows(record)

frames_before, frames_after, bytes_before, bytes_after, shrunk = {}, {}, {}, {}, []
roundtrips = 0
for case, rendered in zip(bundle['cases'], renderer['cases']):
    ident = case['id']
    assert ident == rendered['id']
    assert case['python'] == references[ident]['python']
    assert case['rust'] == rust_cases[ident]['rust']
    previous = json.loads(gzip.decompress((BASELINE / f'{ident}-cells.json.gz').read_bytes()))
    cells = json.loads(gzip.decompress((HERE / f'{ident}-cells.json.gz').read_bytes()))
    # Nothing on screen may move: this cycle changes when the frame is drawn.
    assert cells == previous, ident
    for image in rendered['images']:
        assert sha(HERE / image['file']) == image['sha256']
        assert sha(HERE / image['file']) == sha(BASELINE / image['file']), ident
    old_record = next(c['rust'] for c in previous_bundle['cases']
                      if c['id'] == ident and c['scenario'] == case['scenario'])
    old_spans, _ = windows(old_record, case['scenario'])[0], None
    new_spans, _ = windows(case['rust'], case['scenario'])
    frames_before[ident] = [chunk.count(b'Browse sessions') if case['scenario'] != 'picker-empty'
                            else chunk.count(b'No sessions found.') for _, chunk in old_spans]
    frames_after[ident] = [chunk.count(b'Browse sessions') if case['scenario'] != 'picker-empty'
                           else chunk.count(b'No sessions found.') for _, chunk in new_spans]
    bytes_before[ident] = len(base64.b64decode(old_record['raw_base64']))
    bytes_after[ident] = len(base64.b64decode(case['rust']['raw_base64']))
    if bytes_after[ident] < bytes_before[ident]:
        shrunk.append(ident)
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
# Every picker case except the empty store lost bytes; the empty store has no loop.
assert len(shrunk) == 8, shrunk

equal_regions = []
for entry in load(HERE / 'region-pixels.json')['results']:
    assert entry['png_sha256'] == sha(HERE / f'{entry["id"]}-bottom.png')
    for region in entry['regions']:
        assert region['equal'] and region['differing_pixels'] == 0, (entry['id'], region)
        equal_regions.append(f'{entry["id"]}:{region["label"]}')
assert len(equal_regions) == 34, len(equal_regions)

declared = []
for entry in load(HERE / 'region-pixels-tag.json')['results']:
    assert entry['png_sha256'] == sha(HERE / f'{entry["id"]}-bottom.png')
    for region in entry['regions']:
        assert region['row'] == 5 and region['colEnd'] - region['colStart'] == 5, region
        assert region['differing_pixels'] == 270, (entry['id'], region)
        declared.append(f'{entry["id"]}:{region["label"]}')
assert len(declared) == 4, len(declared)

audit = {
    'status': 'REDRAW_ON_INPUT_FIXED_FRAME_CONTENT_UNCHANGED_ALL_REGIONS_EQUAL',
    'source': rust['rust_commit'],
    'fix_commit': FIX_COMMIT,
    'raw_cast_roundtrips': roundtrips,
    'png_hash_checks': 10,
    'all_ten_pngs_identical_to_previous_packet': True,
    'all_ten_cell_maps_identical_to_previous_packet': True,
    'python_records_and_cells_unchanged': True,
    'renderer_script_sha256': renderer_sha,
    'renderer_settings_match_prior': True,
    'checker_runs': checker_runs,
    'checker_failures': 0,
    'gate_rejects_previous_capture': {'side': 'rust', 'failing_cases': previous_failures},
    'gate_accepts_reference_records': True,
    'frames_per_window_before': frames_before,
    'frames_per_window_after': frames_after,
    'raw_bytes_before': bytes_before,
    'raw_bytes_after': bytes_after,
    'cases_with_smaller_record': sorted(shrunk),
    'tested_patch': {'sha256': TESTED_PATCH_SHA, 'bytes': 2801,
                     'request_patch_identical': True},
    'pixel_equal_regions': sorted(equal_regions),
    'pixel_differing_regions': [],
    'declared_differing_regions': sorted(declared),
    'declared_difference_reason': 'status tag word is fixture data (retained Python session is '
                                  'interrupted, Rust fixture complete); the ink at this span is '
                                  'pinned by check_picker_status_ink.py on both sides',
    'still_open_outside_regions': ['the live capture exercises only the complete->done status '
                                   'mapping',
                                   'Active/ID column content is a documented adaptation',
                                   'resize, long lists and clear-filter are unproved by these '
                                   'fixed-size fixtures'],
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
                  'declared_differing_regions': len(declared),
                  'rejected_cases_in_previous_capture': len(previous_failures),
                  'cases_with_smaller_record': len(shrunk)}, indent=2))
