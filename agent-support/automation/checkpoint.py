#!/usr/bin/env python3
"""Opt-in auto-push AFTER reviewed commits; never stages, commits or merges.

A clone must install explicitly for its assigned current Arena branch. Local
state and hooks are deliberately not versioned. Git post-commit cannot roll
back a commit: failures are recorded and must be reported/retried explicitly.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import sys

STATE = 'hermes-agent-checkpoints.json'
HOOK = '''#!/bin/sh
# hermes-agent-autopush-v1
root=$(git rev-parse --show-toplevel) || exit 1
exec python3 "$root/agent-support/automation/checkpoint.py" push
'''


def git(*args, cwd=None, timeout=120):
    env = dict(os.environ, GIT_TERMINAL_PROMPT='0')
    return subprocess.run(['git', *args], cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                          capture_output=True, text=True, timeout=timeout)


def root_path():
    result = git('rev-parse', '--show-toplevel')
    if result.returncode:
        raise RuntimeError('Run from a non-bare working repository')
    return Path(result.stdout.strip()).resolve()


def local_path(root, name):
    result = git('rev-parse', '--git-path', name, cwd=root)
    if result.returncode:
        raise RuntimeError('Cannot find local Git state path')
    path = Path(result.stdout.strip())
    return path if path.is_absolute() else root / path


def branch_name(root):
    result = git('symbolic-ref', '--quiet', '--short', 'HEAD', cwd=root)
    return result.stdout.strip() if result.returncode == 0 else ''


def safe_branch(branch):
    # Main/master and detached HEAD are never destinations for this helper.
    return branch.startswith('arena/') and not any(c.isspace() for c in branch)


def load_state(root):
    path = local_path(root, STATE)
    if not path.exists():
        raise RuntimeError('Auto-push not installed; follow START-NEXT-SESSION.md')
    state = json.loads(path.read_text())
    if state.get('version') != 1:
        raise RuntimeError('Unsupported local auto-push state')
    return state


def write_state(root, state):
    path = local_path(root, STATE)
    tmp = path.with_suffix('.tmp')
    tmp.write_text(json.dumps(state, indent=2)+'\n')
    tmp.replace(path)


def default_hooks(root):
    configured = git('config', '--get', 'core.hooksPath', cwd=root)
    if configured.returncode == 0:
        raise RuntimeError('Custom core.hooksPath exists; refuse to replace its hook system')
    if configured.returncode != 1:
        raise RuntimeError('Cannot inspect hook configuration')
    return local_path(root, 'hooks')


def install(root, branch):
    if not safe_branch(branch) or branch_name(root) != branch:
        raise RuntimeError('Bind only to the current platform-assigned arena/ branch')
    if git('check-ref-format', '--branch', branch, cwd=root).returncode:
        raise RuntimeError('Invalid branch name')
    if git('remote', 'get-url', 'origin', cwd=root).returncode:
        raise RuntimeError('Origin remote is required; no URL or credential is stored')
    hook = default_hooks(root) / 'post-commit'
    if hook.exists() and (hook.is_symlink() or hook.read_text() != HOOK):
        raise RuntimeError('Existing post-commit hook is not ours; refuse to overwrite it')
    if hook.is_symlink():
        raise RuntimeError('Refuse a symlink post-commit hook')
    hook.parent.mkdir(parents=True, exist_ok=True)
    hook.write_text(HOOK)
    hook.chmod(0o755)
    write_state(root, {'version': 1, 'enabled': True, 'branch': branch,
                       'remote': 'origin', 'last_push': None})
    print(f'Installed post-commit auto-push for {branch}; existing other hooks preserved.')
    return 0


def push(root):
    state = load_state(root)
    branch = branch_name(root)
    if not state.get('enabled'):
        raise RuntimeError('Auto-push is disabled')
    if (not safe_branch(branch) or branch != state.get('branch')
            or state.get('remote') != 'origin'):
        raise RuntimeError('Wrong/protected/detached branch; no push attempted')
    commit = git('rev-parse', 'HEAD', cwd=root)
    if commit.returncode:
        raise RuntimeError('No committed checkpoint to push')
    receipt = {'branch': branch, 'commit': commit.stdout.strip(),
               'at': datetime.now(timezone.utc).isoformat()}
    try:
        result = git('push', '--no-follow-tags', 'origin', branch, cwd=root)
        success = result.returncode == 0
        receipt.update(outcome='success' if success else 'failure', exit_code=result.returncode)
    except subprocess.TimeoutExpired:
        success = False
        receipt.update(outcome='failure', reason='timeout')
    # Do not retain remote output: a configured URL could contain credentials.
    state['last_push'] = receipt
    write_state(root, state)
    if success:
        print(f'Auto-pushed {receipt["commit"][:12]} to origin/{branch}; no force or merge.')
        return 0
    print('Checkpoint remains committed locally, but auto-push FAILED. '
          'Check connection/remote divergence and retry this tool; never force-push. '
          'If authentication failed, reconnect GitHub in Arena. '
          'Remote output withheld to avoid exposing credentials.', file=sys.stderr)
    return 1


def status(root, require_pushed=False):
    state = load_state(root)
    current = git('rev-parse', 'HEAD', cwd=root).stdout.strip()
    receipt = state.get('last_push') or {}
    hook = default_hooks(root) / 'post-commit'
    hook_ok = hook.is_file() and not hook.is_symlink() and hook.read_text() == HOOK and os.access(hook, os.X_OK)
    result = {'enabled': state.get('enabled'), 'bound_branch': state.get('branch'),
              'current_branch': branch_name(root), 'hook_installed': hook_ok,
              'last_push': state.get('last_push'), 'receipt_is_current':
              receipt.get('outcome') == 'success' and receipt.get('commit') == current
              and receipt.get('branch') == branch_name(root)}
    print(json.dumps(result, indent=2))
    return int(require_pushed and not (result['enabled'] and hook_ok and result['receipt_is_current']))


def disable(root):
    state = load_state(root)
    hook = default_hooks(root) / 'post-commit'
    if hook.exists():
        if hook.is_symlink() or hook.read_text() != HOOK:
            raise RuntimeError('Refuse to remove an unrecognized hook')
        hook.unlink()
    state['enabled'] = False
    write_state(root, state)
    print('Managed auto-push disabled; all other hooks unchanged.')
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('install').add_argument('--branch', required=True)
    sub.add_parser('push')
    sub.add_parser('status').add_argument('--require-pushed', action='store_true')
    sub.add_parser('disable')
    args = parser.parse_args()
    try:
        root = root_path()
        if args.command == 'install': return install(root, args.branch)
        if args.command == 'push': return push(root)
        if args.command == 'status': return status(root, args.require_pushed)
        return disable(root)
    except (RuntimeError, ValueError, OSError, subprocess.TimeoutExpired) as error:
        # Error text comes only from our checks/local exceptions, not Git remotes.
        print(f'Auto-push blocked: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
