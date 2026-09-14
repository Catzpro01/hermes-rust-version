"""Tests of PTY provenance and fail-closed snapshot boundaries, not UI parity."""
import base64
from pathlib import Path
import sys
import tempfile
import unittest

from capture_ui import record


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


if __name__ == '__main__':
    unittest.main()
