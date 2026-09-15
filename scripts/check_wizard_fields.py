"""Spec 017 Lane 2 (W3) wizard-field evidence gate.

W3 contract: every prompt/label/value the wizard RENDERS per section frame
must actually appear in captured evidence; nested data that only lives in the
answer model is proven by unit tests, and secrets never enter captures. This
gate pins the rendered field sequence of the implemented wizard steps at the
byte level, in order:

* model section — provider choice, then the three common model fields
  (`API base URL`, `Environment variable holding the API key`, `Model name`)
  through `Setup complete!`, plus a cancel at the first field;
* terminal section — the docker-unavailable notice followed by the
  `Docker image` field through `Setup complete!`; plus the NORMAL local
  path, which must complete without any docker prompt (T12 matrix);
* gateway section — cancel straight from the platform multiselect; plus
  the NORMAL empty selection, which must render `No platforms selected`
  and still complete (T12 matrix);
* tools section — NORMAL (confirm the default toolset selection) and
  CANCEL from the multiselect (T12 matrix closes every wizard section).

Byte order matters: the markers must appear sequentially in the recorded
stream, proving each prompt was drawn in its own frame before the next one.
"""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import sys

SCENARIOS = ('wizard-model-fields', 'wizard-model-cancel',
             'wizard-docker-image', 'wizard-gateway-cancel',
             'wizard-terminal-local', 'wizard-gateway-empty',
             'wizard-tools-accept', 'wizard-tools-cancel')
COMPLETE = 'Setup complete!'
CANCELLED = 'Setup cancelled.'
PLATFORMS_NONE = 'No platforms selected'

# Ordered rendered markers per scenario (W3: rendered fields must be visible).
FIELDS = {
    'wizard-model-fields': ['Select provider', 'API base URL',
                            'Environment variable holding the API key',
                            'Model name', COMPLETE],
    'wizard-model-cancel': ['Select provider', 'API base URL', CANCELLED],
    'wizard-docker-image': ['Select terminal backend', 'Docker not found',
                            'Docker image', COMPLETE],
    'wizard-gateway-cancel': ['Select platforms to configure', CANCELLED],
    # Spec017 T12 matrix completion — terminal NORMAL: choosing Local
    # completes the section without any docker prompt.
    'wizard-terminal-local': ['Select terminal backend', COMPLETE],
    # Gateway NORMAL: an untoggled multiselect renders the empty-selection
    # notice and still completes.
    'wizard-gateway-empty': ['Select platforms to configure', PLATFORMS_NONE,
                             COMPLETE],
    # Tools NORMAL: confirming the default toolset selection completes the
    # section; Tools CANCEL: ESC straight from the multiselect.
    'wizard-tools-accept': ['Select toolsets to enable:', COMPLETE],
    'wizard-tools-cancel': ['Select toolsets to enable:', CANCELLED],
}
# What must NOT appear (a cancel never completes; a full run never cancels).
FORBIDDEN = {
    'wizard-model-fields': [CANCELLED],
    'wizard-model-cancel': [COMPLETE],
    'wizard-docker-image': [CANCELLED],
    'wizard-gateway-cancel': [COMPLETE],
    # The local path must never touch the docker branch.
    'wizard-terminal-local': [CANCELLED, 'Docker not found', 'Docker image'],
    'wizard-gateway-empty': [CANCELLED],
    'wizard-tools-accept': [CANCELLED],
    'wizard-tools-cancel': [COMPLETE],
}
# Value entered at a prompt must be echoed back by the rendered frame.
TYPED_AFTER = {'wizard-model-fields': ('Model name', 'parity-fixture')}


def raw_bytes(record):
    raw = base64.b64decode(record['raw_base64'], validate=True)
    if hashlib.sha256(raw).hexdigest() != record['raw_sha256']:
        raise RuntimeError('Corrupt raw capture: the recorded hash does not match')
    events = b''.join(base64.b64decode(entry[1], validate=True)
                      for entry in record['events_base64'])
    if events != raw:
        raise RuntimeError('Corrupt raw capture: events do not rebuild the byte stream')
    return raw


def case_problems(record, scenario):
    if record.get('error'):
        return [f'capture error: {record["error"]}']
    raw = raw_bytes(record)
    problems = []
    markers = FIELDS[scenario]
    position = -1
    for marker in markers:
        index = raw.find(marker.encode(), position + 1)
        if index < 0:
            problems.append(f'the wizard never rendered {marker!r}'
                            + ('' if position < 0 else
                               f' after byte {position} (out of sequence or missing)'))
            position = len(raw)
            continue
        position = index
    for banned in FORBIDDEN[scenario]:
        if banned.encode() in raw:
            problems.append(f'{banned!r} appeared although this scenario must not reach it')
    if scenario in TYPED_AFTER:
        prompt, value = TYPED_AFTER[scenario]
        anchor = raw.find(prompt.encode())
        typed = raw.find(value.encode(), anchor + 1 if anchor >= 0 else 0)
        if anchor < 0 or typed < 0:
            problems.append(f'the value {value!r} typed at {prompt!r} was never echoed '
                            'by a rendered frame')
    return problems


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('recording', type=Path)
    args = parser.parse_args()
    bundle = json.loads(args.recording.read_text())
    failures = []
    for case in bundle['cases']:
        problems = case_problems(case['rust'], case['scenario'])
        status = 'PASS' if not problems else 'FAIL'
        print(f'{status} {case["id"]}: '
              + ('rendered field sequence complete' if not problems else '; '.join(problems)))
        failures.extend(problems)
    sys.exit(bool(failures))
