"""Behavior checks against the retained, real UI packet (never alter originals)."""
from pathlib import Path
import tempfile
import unittest

from audit_ui_evidence import audit

PACKET = Path(__file__).resolve().parents[1] / 'docs/hermes-ui-spec/017/evidence/ui-3b39bd7'


class AuditTests(unittest.TestCase):
    def test_retained_packet_roundtrips(self):
        self.assertEqual(audit(PACKET)['raw_cast_roundtrips'], 96)

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
