#!/usr/bin/env python3
"""Read-only continuity-package verification; no network, commits or reset."""
import hashlib
import json
import os
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]


def verify(root=ROOT):
    aliases = {'MEMORY.md': 'agent-support/memory/MEMORY.md',
               'PROGRESS.md': 'agent-support/memory/PROGRESS.md',
               'CONTEXT.md': 'agent-support/context/CONTEXT.md',
               'docs/agents': 'agent-support/guidance',
               '.agents/skills': 'agent-support/skills',
               '.agents/README.md': 'agent-support/guidance/SKILLS-ENTRY.md',
               '.scratch/hermes-rs-agent-skills': 'agent-support/planning/agent-skills'}
    for alias, target in aliases.items():
        assert (root/alias).is_symlink(), f'Missing compatibility symlink: {alias}'
        assert (root/alias).resolve(strict=True) == (root/target).resolve(strict=True), alias
    lock = json.loads((root/'agent-support/guidance/vendor/mattpocock-skills.lock.json').read_text())
    vendor = root/'agent-support/guidance/vendor/mattpocock-skills'
    assert lock['commit'] == '3cca18b368ae95cdbdebbff572ccafa662551015'
    assert len(lock['files']) == 164
    for entry in lock['files']:
        file = vendor/entry['path']
        raw = os.readlink(file).encode() if file.is_symlink() else file.read_bytes()
        assert file.is_symlink() == (entry['mode'] == '120000'), entry['path']
        assert len(raw) == entry['size'], entry['path']
        assert hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest() == entry['git_blob'], entry['path']
        if not file.is_symlink():
            assert bool(file.stat().st_mode & 0o111) == (entry['mode'] == '100755'), entry['path']
    actual_files = {str(p.relative_to(vendor)) for p in vendor.rglob('*') if p.is_file() or p.is_symlink()}
    assert actual_files == {e['path'] for e in lock['files']}, 'Unexpected/missing vendor files'
    links = sorted((root/'agent-support/skills').iterdir())
    assert len(links) == 37
    for link in links:
        assert link.is_symlink() and link.resolve().is_relative_to(vendor.resolve()), link
        assert link.resolve().name == link.name, link
        assert (link/'SKILL.md').is_file(), link
        assert (root/'.agents/skills'/link.name/'SKILL.md').read_bytes() == (link/'SKILL.md').read_bytes()
    docs = [root/'AGENTS.md', root/'agent-support/README.md', root/'agent-support/RULES.md',
            *(root/'agent-support/handoff').glob('*.md'), root/'agent-support/automation/README.md']
    for file in docs:
        assert file.is_file(), file
        for link in re.findall(r'\]\(([^)]+)\)', file.read_text()):
            if link.startswith(('http://', 'https://', '#', 'mailto:')): continue
            assert (file.parent/link.split('#',1)[0]).exists(), (str(file),link)
    assert not (root/'AGENTS.md').is_symlink()
    return {'compatibility_aliases': len(aliases), 'original_vendor_files': len(lock['files']),
            'skill_links': len(links), 'guide_files': len(docs), 'status': 'PASS'}


if __name__ == '__main__':
    print(json.dumps(verify(), indent=2))
