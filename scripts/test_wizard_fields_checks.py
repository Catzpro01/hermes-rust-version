"""Unit tests for the Spec 017 Lane 2 wizard-field checker (synthetic
streams: the checker judges byte presence and order only)."""
import base64
import hashlib
import unittest

from check_wizard_fields import case_problems


def make_record(chunks):
    raw = ''.join(chunks).encode()
    events = [[0.1 + i, base64.b64encode(chunk.encode()).decode()]
              for i, chunk in enumerate(chunks)]
    return {'width': 100, 'height': 30,
            'raw_base64': base64.b64encode(raw).decode(),
            'raw_sha256': hashlib.sha256(raw).hexdigest(),
            'events_base64': events, 'inputs_base64': [], 'error': None}


MODEL_FIELDS = ['Configuration Location\nSelect provider:\n',
                'API base URL: \n',
                'Environment variable holding the API key: NOUS_API_KEY\n',
                'Model name: parity-fixture\n',
                'Setup complete! You\'re ready to go.\n']


class WizardFieldsCheckerTests(unittest.TestCase):
    def test_model_fields_reference_sequence_passes(self):
        self.assertEqual(case_problems(make_record(MODEL_FIELDS), 'wizard-model-fields'), [])

    def test_model_fields_missing_key_env_prompt_fails(self):
        record = make_record([MODEL_FIELDS[0], MODEL_FIELDS[1],
                              MODEL_FIELDS[3], MODEL_FIELDS[4]])
        problems = case_problems(record, 'wizard-model-fields')
        self.assertTrue(any('Environment variable holding the API key' in p
                            for p in problems), problems)

    def test_model_fields_out_of_order_fails(self):
        record = make_record([MODEL_FIELDS[0], MODEL_FIELDS[2],
                              MODEL_FIELDS[1], MODEL_FIELDS[3], MODEL_FIELDS[4]])
        problems = case_problems(record, 'wizard-model-fields')
        self.assertTrue(any('out of sequence or missing' in p for p in problems), problems)

    def test_model_fields_typed_value_must_be_echoed(self):
        chunks = list(MODEL_FIELDS)
        chunks[3] = 'Model name: \n'  # the prompt renders, the typed value does not
        problems = case_problems(make_record(chunks), 'wizard-model-fields')
        self.assertTrue(any('parity-fixture' in p for p in problems), problems)

    def test_model_cancel_reference_sequence_passes(self):
        record = make_record(['Select provider:\n', 'API base URL: \n',
                              'Setup cancelled.\n'])
        self.assertEqual(case_problems(record, 'wizard-model-cancel'), [])

    def test_model_cancel_that_completes_anyway_fails(self):
        record = make_record(['Select provider:\n', 'API base URL: \n',
                              'Setup cancelled.\n', 'Setup complete! You\'re ready to go.\n'])
        problems = case_problems(record, 'wizard-model-cancel')
        self.assertTrue(any('must not reach it' in p for p in problems), problems)

    def test_docker_image_reference_sequence_passes(self):
        record = make_record(['Select terminal backend:\n',
                              'Docker not found in PATH!\n',
                              'Docker image: nikolaik/python-nodejs:python3.11-nodejs20\n',
                              'Setup complete! You\'re ready to go.\n'])
        self.assertEqual(case_problems(record, 'wizard-docker-image'), [])

    def test_docker_image_skipping_the_image_field_fails(self):
        record = make_record(['Select terminal backend:\n',
                              'Docker not found in PATH!\n',
                              'Setup complete! You\'re ready to go.\n'])
        problems = case_problems(record, 'wizard-docker-image')
        self.assertTrue(any("'Docker image'" in p for p in problems), problems)

    def test_gateway_cancel_reference_sequence_passes(self):
        record = make_record(['Select platforms to configure:\n',
                              'Setup cancelled.\n'])
        self.assertEqual(case_problems(record, 'wizard-gateway-cancel'), [])

    def test_capture_error_is_surfaced_not_masked(self):
        record = make_record(['anything'])
        record['error'] = 'missing readiness at stage 1: API base URL'
        self.assertEqual(case_problems(record, 'wizard-model-fields'),
                         ['capture error: missing readiness at stage 1: API base URL'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
