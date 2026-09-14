"""Offline tests: temporary repositories and bare origin, never GitHub writes."""
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
BRANCH = 'arena/01a09c1e-hermes-rust-version'
spec = importlib.util.spec_from_file_location('checkpoint', HERE/'checkpoint.py')
checkpoint = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checkpoint)


class CheckpointTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.root = self.base/'work'
        self.remote = self.base/'remote.git'
        self.env = {k:v for k,v in os.environ.items() if not k.startswith('GIT_')}
        self.env.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull,
                        GIT_AUTHOR_NAME='Fixture', GIT_AUTHOR_EMAIL='fixture@example.invalid',
                        GIT_COMMITTER_NAME='Fixture', GIT_COMMITTER_EMAIL='fixture@example.invalid')
        self.run_git('init','--bare',str(self.remote),cwd=self.base)
        self.run_git('init','-b',BRANCH,str(self.root),cwd=self.base)
        self.run_git('remote','add','origin',str(self.remote))
        target = self.root/'agent-support/automation/checkpoint.py'
        target.parent.mkdir(parents=True)
        shutil.copyfile(HERE/'checkpoint.py',target)
        self.script = target

    def run_git(self,*args,cwd=None,check=True):
        return subprocess.run(['git',*args],cwd=cwd or self.root,env=self.env,
                              capture_output=True,text=True,check=check)

    def tool(self,*args):
        return subprocess.run([sys.executable,str(self.script),*args],cwd=self.root,
                              env=self.env,capture_output=True,text=True)

    def commit(self):
        self.run_git('add','agent-support/automation/checkpoint.py')
        return self.run_git('commit','-m','Fixture checkpoint')

    def test_post_commit_pushes_without_staging_unreviewed_files(self):
        other = self.root/'.git/hooks/commit-msg'
        other.write_text('#!/bin/sh\n# existing hook\nexit 0\n');other.chmod(0o755)
        original = other.read_bytes()
        self.assertEqual(self.tool('install','--branch',BRANCH).returncode,0)
        (self.root/'unreviewed.txt').write_text('not staged')
        self.commit()
        self.assertEqual(other.read_bytes(),original)
        self.assertIn('?? unreviewed.txt',self.run_git('status','--short').stdout)
        remote = self.run_git('ls-remote','origin','refs/heads/'+BRANCH).stdout.split()[0]
        self.assertEqual(remote,self.run_git('rev-parse','HEAD').stdout.strip())
        self.assertEqual(self.tool('status','--require-pushed').returncode,0)
        self.assertEqual(self.run_git('show','HEAD:unreviewed.txt',check=False).returncode,128)

    def test_does_not_follow_tags_even_when_git_config_requests_it(self):
        self.commit()
        self.run_git('tag','-a','private-fixture-tag','-m','Must stay local')
        self.run_git('config','push.followTags','true')
        self.assertEqual(self.tool('install','--branch',BRANCH).returncode,0)
        self.assertEqual(self.tool('push').returncode,0)
        self.assertEqual(self.run_git('ls-remote','origin','refs/tags/*').stdout,'')

    def test_refuses_other_or_protected_install_branch(self):
        for branch in ('main','master','arena/not-this-session'):
            self.assertNotEqual(self.tool('install','--branch',branch).returncode,0)
        self.assertFalse((self.root/'.git/hooks/post-commit').exists())

    def test_preserves_unknown_post_commit(self):
        hook = self.root/'.git/hooks/post-commit'
        hook.write_text('#!/bin/sh\necho user-hook\n')
        self.assertNotEqual(self.tool('install','--branch',BRANCH).returncode,0)
        self.assertEqual(hook.read_text(),'#!/bin/sh\necho user-hook\n')

    def test_preserves_custom_hook_system(self):
        self.run_git('config','core.hooksPath','custom-hooks')
        self.assertNotEqual(self.tool('install','--branch',BRANCH).returncode,0)
        self.assertEqual(self.run_git('config','--get','core.hooksPath').stdout.strip(),'custom-hooks')

    def test_failed_push_leaves_commit_and_can_retry(self):
        self.assertEqual(self.tool('install','--branch',BRANCH).returncode,0)
        self.run_git('remote','set-url','origin',str(self.base/'missing.git'))
        result = self.commit()
        self.assertIn('auto-push FAILED',result.stderr)
        self.assertEqual(self.run_git('rev-parse','HEAD').returncode,0)
        self.assertNotEqual(self.tool('status','--require-pushed').returncode,0)
        self.run_git('remote','set-url','origin',str(self.remote))
        self.assertEqual(self.tool('push').returncode,0)
        self.assertEqual(self.tool('status','--require-pushed').returncode,0)

    def test_wrong_or_detached_branch_never_reaches_push(self):
        # Simulate external Git state; no actual checkout or protected branch created.
        for branch in ('', 'main', 'arena/other'):
            with self.subTest(branch=branch), patch.object(checkpoint,'load_state',return_value={
                'enabled':True,'branch':BRANCH,'remote':'origin'}), \
                patch.object(checkpoint,'branch_name',return_value=branch), \
                patch.object(checkpoint,'git') as git:
                with self.assertRaisesRegex(RuntimeError,'branch'):
                    checkpoint.push(self.root)
                git.assert_not_called()

    def test_disable_removes_only_managed_hook(self):
        self.assertEqual(self.tool('install','--branch',BRANCH).returncode,0)
        self.assertEqual(self.tool('disable').returncode,0)
        self.assertFalse((self.root/'.git/hooks/post-commit').exists())
        self.assertNotEqual(self.tool('push').returncode,0)


if __name__ == '__main__':
    unittest.main()
