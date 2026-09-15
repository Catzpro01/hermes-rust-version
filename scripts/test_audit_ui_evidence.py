"""Behavior checks against the retained, real UI packet (never alter originals)."""
from pathlib import Path
import tempfile
import unittest

from audit_ui_evidence import audit

PACKET = Path(__file__).resolve().parents[1] / 'docs/hermes-ui-spec/017/evidence/ui-3b39bd7'


class AuditTests(unittest.TestCase):
    def test_retained_packet_roundtrips(self):
        self.assertEqual(audit(PACKET)['raw_cast_roundtrips'], 96)

    def test_frozen_packet_is_not_judged_by_todays_matrix(self):
        """The matrix grows as lanes settle; a retained packet must still audit.

        Before this was separated, extending the paired matrix made the audit of
        `ui-3b39bd7` fail with 'missing or duplicate cases' — a capture defect
        reported against a packet that had none.
        """
        result = audit(PACKET)
        coverage = result['coverage']
        self.assertLess(coverage['packet_scenarios'], coverage['current_matrix_scenarios'])
        self.assertTrue(coverage['in_matrix_but_not_in_packet'])
        self.assertTrue(coverage['recorded_deferred'])

    def test_retained_cases_still_have_plans(self):
        """No retained case may lose its capture plan.

        The packet is re-pairable and re-auditable only while every scenario it
        contains still resolves to a keyed plan on both sides, so removing or
        renaming a plan is caught here rather than at recapture time.
        """
        import json
        from capture_ui import MATRIX_DEFERRED, CASES, steps_for
        scenarios = {c['id'].rsplit('-', 1)[0] for c in
                     json.loads((PACKET / 'paired-bundle.json').read_text())['cases']}
        for name in sorted(scenarios):
            with self.subTest(scenario=name):
                self.assertIn(name, set(CASES) | set(MATRIX_DEFERRED))
                for side in ('python', 'rust'):
                    plan = steps_for(name, side)
                    self.assertTrue(any(key is not None for _, key in plan) or len(plan) == 1,
                                    f'{name}/{side}: no typed key at all')

    def assert_rejects_changed_file(self, filename):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for original in PACKET.iterdir():
                if original.is_file():
                    (root / original.name).symlink_to(original)
            changed = root / filename
            changed.unlink()  # Remove temporary symlink, never the original.
            changed.write_bytes(b'TAMPERED')
            with self.assertRaises(AssertionError):
                audit(root)

    def test_rejects_changed_ansi(self):
        self.assert_rejects_changed_file('wizard-mode-100x30-python.ansi')

    def test_rejects_changed_png(self):
        self.assert_rejects_changed_file('wizard-mode-100x30-bottom.png')


if __name__ == '__main__':
    unittest.main()
