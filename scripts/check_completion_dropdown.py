"""Spec 017 Lane 3 (W4) completion evidence gate.

W4 contract: completion is proven by the Rust inline dropdown plus pinned
evidence at the actual CLI/PTY seam. Observed rustyline semantics (run
34921676422): Tab inserts the FIRST registry-order candidate into the
line; the dropdown menu is not part of the captured surface. The REPL's
welcome frame must render first, then the completion surface:

* unique completion — Tab on `/mod` completes the line to `/model`
  (picker commands get no trailing space, per the registry contract);
* prefix-filtered candidates — Tab on `/s` inserts the first registry
  s-command (`/save `); clearing and Tabbing on `/ski` narrows the set
  and inserts `/skin` (picker command, no trailing space): two different
  insertions from one registry prove the candidate set;
* subcommand + skill completion — after `/skills `, Tab inserts the
  first declared subcommand (`search`); then `/skills de` completes the
  seeded `demo-skill` from `$HERMES_HOME/skills/`;
* ghost text — the hinter renders the remainder `nality` of the unique
  completion `/personality` while only `/perso` is typed, without Tab;
* accepted completion opens the next UI — Enter on the completed
  `/sessions` line opens the browse picker (`Browse sessions`).

Byte order matters: the ordered markers must appear sequentially in the
recorded stream. Python-side comparison lands with the user's reference
recording (W4-Q1); this gate pins the Rust side.
"""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import sys

SCENARIOS = ('completion-command', 'completion-alternatives',
             'completion-subcommand', 'completion-ghost',
             'completion-picker-open')
WELCOME = 'Welcome to Hermes Agent!'

# Ordered rendered markers per scenario (welcome frame first, then the
# completion surface, then what accepting it produces).
FIELDS = {
    'completion-command': [WELCOME, '/model'],
    'completion-alternatives': [WELCOME, '/save', '/skin'],
    'completion-subcommand': [WELCOME, 'search', 'demo-skill'],
    'completion-ghost': [WELCOME, '/perso', 'nality'],
    'completion-picker-open': [WELCOME, '/sessions', 'Browse sessions'],
}
# Unordered frame requirements (currently none: the menu surface is not
# part of the captured evidence — insertion into the line is the proof).
PRESENT = {}
# Outcomes that must never render in these scenarios.
FORBIDDEN = {
    'completion-command': ['Browse sessions'],
    'completion-alternatives': ['Browse sessions'],
    'completion-subcommand': ['Browse sessions'],
    'completion-ghost': ['Browse sessions'],
    'completion-picker-open': [],
}


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
    position = -1
    for marker in FIELDS[scenario]:
        index = raw.find(marker.encode(), position + 1)
        if index < 0:
            problems.append(f'the REPL never rendered {marker!r}'
                            + ('' if position < 0 else
                               f' after byte {position} (out of sequence or missing)'))
            position = len(raw)
            continue
        position = index
    for needed in PRESENT.get(scenario, ()):
        if needed.encode() not in raw:
            problems.append(f'dropdown candidate {needed!r} never appeared in the frame')
    for banned in FORBIDDEN[scenario]:
        if banned.encode() in raw:
            problems.append(f'{banned!r} appeared although this scenario must not reach it')
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
              + ('completion evidence complete' if not problems else '; '.join(problems)))
        failures.extend(problems)
    sys.exit(bool(failures))
