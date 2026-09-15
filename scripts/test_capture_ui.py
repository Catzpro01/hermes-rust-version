"""Tests of PTY provenance and fail-closed snapshot boundaries, not UI parity."""
import base64
import sqlite3
from pathlib import Path
import sys
import tempfile
import unittest

from capture_ui import (CASES, MATRIX_DEFERRED, DEFAULT_PICKER_SEED, long_list_seed,
                        picker_rows, record, section_for, seed_rust, steps_for)

GATE_MODULES = ('check_wizard_fields', 'check_completion_dropdown',
                'check_picker_browse_control')


class RecordingTests(unittest.TestCase):
    def run_child(self, code, steps):
        with tempfile.TemporaryDirectory() as tmp:
            return record([sys.executable, '-c', code], Path(tmp), 80, steps, timeout=2)

    def test_input_is_recorded_and_output_roundtrips(self):
        result = self.run_child("import os,tty;tty.setraw(0);os.write(1,b'READY');os.read(0,1);os.write(1,b'DONE')",
                                [("READY", "x"), ("DONE", None)])
        self.assertIsNone(result.get('error'))
        self.assertEqual(len(result['inputs_base64']), 1)
        raw = base64.b64decode(result['raw_base64'])
        self.assertEqual(raw, b''.join(base64.b64decode(e[1]) for e in result['events_base64']))
        self.assertIn(b'DONE', raw[:result['snapshot_end_byte']])

    def test_exit_snapshot_drains_all_output(self):
        for _ in range(8):
            result = self.run_child("import sys;sys.stdout.buffer.write(b'A'*60000+b'END');sys.stdout.flush()",
                                    [("@exit", None)])
            self.assertEqual(base64.b64decode(result['raw_base64']), b'A'*60000+b'END')

    def test_repainting_same_frame_reaches_snapshot(self):
        result = self.run_child("import os,time;[(os.write(1,b'\\x1b[HREADY'),time.sleep(.03)) for _ in range(200)]",
                                [("READY", None)])
        self.assertIsNone(result.get('error'))

    def test_missing_readiness_is_not_a_capture_pass(self):
        result = self.run_child("print('wrong screen')", [("READY", None)])
        self.assertIn('missing', result['error'])

    def test_failed_process_is_not_an_exit_snapshot_pass(self):
        result = self.run_child("raise SystemExit(3)", [("@exit", None)])
        self.assertIn('exit 3', result['error'])


class PairedMatrixTests(unittest.TestCase):
    """The four-area packet may not silently fall behind the live gates.

    T12's last open item is "complete all pairs". A case that no gate covers is
    a coverage gap; a case a gate covers but the packet omits is a lie by
    omission. Both are caught here, and every deferral has to carry a reason
    long enough to act on, so the gap stays a recorded decision.
    """

    def gate_scenarios(self):
        import importlib
        names = set()
        for module in GATE_MODULES:
            names |= set(importlib.import_module(module).SCENARIOS)
        return names

    def test_every_gate_scenario_is_paired_or_deferred(self):
        covered = set(CASES) | set(MATRIX_DEFERRED)
        for name in sorted(self.gate_scenarios()):
            with self.subTest(scenario=name):
                self.assertIn(name, covered)

    def test_a_case_is_either_paired_or_deferred_not_both(self):
        self.assertFalse(set(CASES) & set(MATRIX_DEFERRED))

    def test_deferral_reasons_are_actionable(self):
        for name, reason in MATRIX_DEFERRED.items():
            self.assertIsInstance(reason, str, name)
            self.assertGreaterEqual(len(reason), 60, f'{name}: reason too thin to act on')
            self.assertIn(' ', reason, name)

    def test_every_paired_case_has_a_keyed_plan_on_both_sides(self):
        for name in CASES:
            for side in ('python', 'rust'):
                plan = steps_for(name, side)
                with self.subTest(scenario=name, side=side):
                    self.assertTrue(plan, f'{name}/{side}: no key plan at all')
                    for marker, _ in plan:
                        self.assertIsInstance(marker, str, f'{name}/{side}: marker type')
                        self.assertTrue(marker, f'{name}/{side}: empty readiness marker')
                    self.assertTrue(any(key is not None for _, key in plan)
                                    or len(plan) == 1,
                                    f'{name}/{side}: nothing is ever typed')

    def test_section_completion_marker_follows_the_side(self):
        """The pinned wizard words completion per section, Rust words it once.

        `hermes_cli/setup.py:3210` prints f"{label} configuration complete!" for
        the requested section, while the Rust section ends with "Setup
        complete!". A single shared marker would either fail the Python side or
        be relaxed until it proves nothing, so the completion row is per-side
        while the state it names stays the same.
        """
        for name, section_done in (('wizard-terminal-local', 'Terminal Backend configuration complete!'),
                                   ('wizard-tools-accept', 'Tools configuration complete!'),
                                   ('wizard-gateway-empty',
                                    'Messaging Platforms (Gateway) configuration complete!')):
            with self.subTest(scenario=name):
                self.assertEqual(steps_for(name, 'python')[-1][0], section_done)
                self.assertEqual(steps_for(name, 'rust')[-1][0], 'Setup complete!')

    def test_wizard_cases_name_a_section_the_cli_accepts(self):
        # `hermes setup <section>` rejects anything outside this set, so an
        # unmapped wizard case would capture a usage error as if it were UI.
        for name in CASES:
            if not name.startswith('wizard'):
                continue
            with self.subTest(scenario=name):
                self.assertIn(section_for(name), (None, 'model', 'terminal', 'gateway', 'tools'))

    def test_arrow_keys_follow_the_side_that_reads_them(self):
        # curses KEY_DOWN under smkx is SS3; the Rust picker sees raw-mode CSI.
        plan_py = steps_for('picker-long-list', 'python')[0][1]
        plan_rs = steps_for('picker-long-list', 'rust')[0][1]
        self.assertEqual(plan_py, '\x1bOB' * 26)
        self.assertEqual(plan_rs, '\x1b[B' * 26)

    def test_picker_fixture_is_shared_by_both_sides(self):
        self.assertEqual(picker_rows('picker-empty'), ())
        rows = picker_rows('picker-long-list')
        self.assertEqual(len(rows), 30)
        self.assertEqual(rows, tuple(long_list_seed(30)))
        for name in ('picker-normal', 'picker-filter', 'picker-long-list', 'picker-empty'):
            if name == 'picker-long-list':
                continue
            expected = () if name == 'picker-empty' else DEFAULT_PICKER_SEED
            self.assertEqual(picker_rows(name), expected, name)

    def test_seeding_the_rust_home_uses_the_shared_rows(self):
        for name in ('picker-empty', 'picker-normal', 'picker-long-list'):
            with tempfile.TemporaryDirectory() as tmp:
                home = Path(tmp)
                seed_rust(home, picker_rows(name))
                with sqlite3.connect(home / 'state.db') as db:
                    sessions = db.execute('SELECT id FROM sessions ORDER BY started_at').fetchall()
                    messages = db.execute('SELECT COUNT(*) FROM messages').fetchone()[0]
                with self.subTest(scenario=name):
                    self.assertEqual([s[0] for s in sessions],
                                     [sid for sid, _ in picker_rows(name)])
                    self.assertEqual(messages, len(picker_rows(name)))


if __name__ == '__main__':
    unittest.main()
