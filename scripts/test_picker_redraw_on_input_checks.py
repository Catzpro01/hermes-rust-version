"""Supporting evidence/decoder checks; not a substitute for actual CLI RED."""
import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import unittest

from check_picker_redraw_on_input import (DEFAULT_MARKER, SCENARIOS, case_problems, check,
                                          key_events, scenario_keys, windows)

ROOT = Path(__file__).resolve().parents[1]


def record(events, inputs=(), width=100, height=30):
    raw = b''.join(data for _, data in events)
    encoded = base64.b64encode(raw).decode()
    return {'width': width, 'height': height, 'error': None,
            'snapshot_end_byte': len(raw), 'raw_base64': encoded,
            'raw_sha256': hashlib.sha256(raw).hexdigest(),
            'events_base64': [[time, base64.b64encode(data).decode()] for time, data in events],
            'inputs_base64': [[time, base64.b64encode(text.encode()).decode(), kind]
                              for time, text, kind in inputs]}


FRAME = b'\x1b[1;1H  Browse sessions \xe2\x80\x94 up/down navigate\x1b[1E'
FOOTER = b'\x1b[30;1H  1/2 sessions   d delete'


class RedrawChecks(unittest.TestCase):
    def setUp(self):
        self.bundle = json.loads((ROOT / 'docs/hermes-ui-spec/017/evidence/ui-3b39bd7/python-bundle.json').read_text())

    def test_reference_records_pass(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check(self.bundle, 'python'), [])

    def test_key_events_counts_presses(self):
        self.assertEqual(key_events('topic'), 5)
        self.assertEqual(key_events('d'), 1)
        self.assertEqual(key_events('\x1b[B'), 1)
        self.assertEqual(key_events('\x1bOB'), 1)
        self.assertEqual(key_events('\x1b[1;1R'), 1)
        self.assertEqual(key_events(' \r'), 2)

    def test_single_frame_then_idle_passes(self):
        record_ = record([(0.1, FRAME), (0.2, FOOTER)])
        self.assertEqual(case_problems(record_, 'picker-normal'), [])

    def test_repaint_while_idle_is_reported(self):
        record_ = record([(0.1, FRAME), (0.2, FOOTER), (0.3, FRAME), (0.4, FRAME)])
        problems = case_problems(record_, 'picker-normal')
        self.assertTrue(any('drew 3 frame(s) but at most 1' in p for p in problems), problems)

    def test_one_frame_per_typed_key_passes(self):
        record_ = record([(0.1, FRAME)] + [(0.21 + i / 100, FRAME) for i in range(5)],
                         inputs=[(0.2, 'topic', 'scenario key')])
        self.assertEqual(case_problems(record_, 'picker-filter'), [])

    def test_extra_repaint_after_a_key_is_reported(self):
        record_ = record([(0.1, FRAME)] + [(0.21 + i / 100, FRAME) for i in range(9)],
                         inputs=[(0.2, 'topic', 'scenario key')])
        problems = case_problems(record_, 'picker-filter')
        self.assertTrue(any("'topic'" in p and 'drew 9 frame(s) but at most 5' in p
                            for p in problems), problems)

    def test_terminal_replies_do_not_open_a_window(self):
        record_ = record([(0.1, FRAME)], inputs=[(0.15, '\x1b[1;1R', 'terminal response (pyte 0.8.2)')])
        self.assertEqual(scenario_keys(record_), [])
        spans, _ = windows(record_)
        self.assertEqual(len(spans), 1)

    def test_windows_split_at_every_key_write(self):
        record_ = record([(0.1, FRAME), (0.3, FRAME), (0.5, FRAME)],
                         inputs=[(0.2, '\x1bOB', 'scenario key'), (0.4, 'zzzz', 'scenario key')])
        spans, keys = windows(record_)
        self.assertEqual([allowance for allowance, _ in spans], [1, 1, 4])
        self.assertEqual([chunk.count(DEFAULT_MARKER) for _, chunk in spans], [1, 1, 1])
        self.assertEqual([text for _, text in keys], ['\x1bOB', 'zzzz'])

    def test_empty_store_uses_its_own_marker(self):
        record_ = record([(0.1, b'No sessions found.\n')])
        self.assertEqual(case_problems(record_, 'picker-empty'), [])
        problems = case_problems(record_, 'picker-normal')
        self.assertTrue(any('never drew' in p for p in problems), problems)

    def test_missing_matrix_is_error(self):
        cases = [c for c in self.bundle['cases'] if c['scenario'] in SCENARIOS]
        with self.assertRaisesRegex(RuntimeError, 'redraw-on-input matrix'):
            check({'cases': cases[:4]}, 'python')

    def test_corrupt_trace_is_error(self):
        case = next(c for c in self.bundle['cases'] if c['scenario'] == 'picker-normal')
        broken = {**case['python'], 'raw_sha256': '0' * 64}
        with self.assertRaisesRegex(RuntimeError, 'Corrupt raw capture'):
            case_problems(broken, 'picker-normal')


if __name__ == '__main__':
    unittest.main()
