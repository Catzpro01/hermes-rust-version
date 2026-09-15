"""Unit tests for the Spec 017 Lane 3 completion checker (synthetic
streams: the checker judges byte presence and order only)."""
import base64
import hashlib
import unittest

from check_completion_dropdown import case_problems


def make_record(chunks):
    raw = ''.join(chunks).encode()
    events = [[0.1 + i, base64.b64encode(chunk.encode()).decode()]
              for i, chunk in enumerate(chunks)]
    return {'width': 100, 'height': 30,
            'raw_base64': base64.b64encode(raw).decode(),
            'raw_sha256': hashlib.sha256(raw).hexdigest(),
            'events_base64': events, 'inputs_base64': [], 'error': None}


WELCOME = 'Welcome to Hermes Agent! Type your message or /help for commands.\n\u276f '


class CompletionDropdownCheckerTests(unittest.TestCase):
    def test_unique_completion_reference_sequence_passes(self):
        record = make_record([WELCOME, '/mod', '\x1b[1m', 'el', '/model', '\n'])
        self.assertEqual(case_problems(record, 'completion-command'), [])

    def test_unique_completion_missing_completed_word_fails(self):
        record = make_record([WELCOME, '/mod\n'])  # Tab never completed the line
        problems = case_problems(record, 'completion-command')
        self.assertTrue(any("'/model'" in p for p in problems), problems)

    def test_alternatives_prefix_filtered_insertions_pass(self):
        record = make_record([WELCOME, '/s\t', '/save \n',
                              '\x15/ski\t', '/skin\n'])
        self.assertEqual(case_problems(record, 'completion-alternatives'), [])

    def test_alternatives_second_insertion_missing_fails(self):
        record = make_record([WELCOME, '/s\t', '/save \n', '\x15/ski\t\n'])
        problems = case_problems(record, 'completion-alternatives')
        self.assertTrue(any("'/skin'" in p for p in problems), problems)

    def test_alternatives_out_of_sequence_fails(self):
        record = make_record(['/skin\n', WELCOME, '/save \n'])  # order broken
        problems = case_problems(record, 'completion-alternatives')
        self.assertTrue(any('out of sequence or missing' in p for p in problems), problems)

    def test_subcommand_then_seeded_skill_passes(self):
        record = make_record([WELCOME, '/skills \t', 'search \n',
                              '\x15/skills de\t', 'demo-skill \n'])
        self.assertEqual(case_problems(record, 'completion-subcommand'), [])

    def test_subcommand_without_skill_fails(self):
        record = make_record([WELCOME, '/skills \t', 'search \n'])
        problems = case_problems(record, 'completion-subcommand')
        self.assertTrue(any('demo-skill' in p for p in problems), problems)

    def test_ghost_text_renders_remainder_without_tab(self):
        record = make_record([WELCOME, '/perso', '\x1b[90m', 'nality', '\x1b[0m\n'])
        self.assertEqual(case_problems(record, 'completion-ghost'), [])

    def test_ghost_text_missing_remainder_fails(self):
        record = make_record([WELCOME, '/perso\n'])
        problems = case_problems(record, 'completion-ghost')
        self.assertTrue(any('nality' in p for p in problems), problems)

    def test_picker_open_reference_sequence_passes(self):
        record = make_record([WELCOME, '/sessio', 'ns ', '\r',
                              '\x1b[?1049h', 'Browse sessions\n'])
        self.assertEqual(case_problems(record, 'completion-picker-open'), [])

    def test_picker_open_without_picker_frame_fails(self):
        record = make_record([WELCOME, '/sessio', 'ns ', '\r\n'])
        problems = case_problems(record, 'completion-picker-open')
        self.assertTrue(any('Browse sessions' in p for p in problems), problems)

    def test_scenarios_must_not_open_unrelated_ui(self):
        record = make_record([WELCOME, '/mod', 'el', '/model',
                              'Browse sessions\n'])  # wrong UI leaked in
        problems = case_problems(record, 'completion-command')
        self.assertTrue(any('must not reach it' in p for p in problems), problems)

    def test_capture_error_is_reported_not_judged(self):
        problems = case_problems({'error': 'timeout waiting for marker'},
                                 'completion-ghost')
        self.assertEqual(problems, ['capture error: timeout waiting for marker'])

    def test_corrupt_capture_hash_raises(self):
        record = make_record([WELCOME, '/perso', 'nality'])
        record['raw_sha256'] = '0' * 64
        with self.assertRaises(RuntimeError):
            case_problems(record, 'completion-ghost')


if __name__ == '__main__':
    unittest.main(verbosity=2)
