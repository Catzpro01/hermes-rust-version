"""CI gate regression checks, without a Rust toolchain or GitHub writes.

Run: python3 -m pip install PyYAML && python3 scripts/test_ci_workflow.py
PyYAML is a review/test dependency only, not a Hermes runtime dependency.
"""

import itertools
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = yaml.safe_load((ROOT / ".github/workflows/ci.yml").read_text())
STEPS = {step["name"]: step for step in WORKFLOW["jobs"]["test"]["steps"] if "name" in step}


class CiGateTests(unittest.TestCase):
    def test_format_pipeline_preserves_exit_status_and_log(self):
        step = STEPS["cargo fmt --check"]
        self.assertEqual(step["id"], "fmt")
        self.assertTrue(step["continue-on-error"])
        for code in (0, 1, 42):
            with self.subTest(code=code), tempfile.TemporaryDirectory() as tmp:
                cargo = Path(tmp) / "cargo"
                cargo.write_text(f"#!/bin/sh\necho format-check-output\nexit {code}\n")
                cargo.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}")
                result = subprocess.run(
                    ["bash", "-e", "-c", step["run"]],
                    cwd=tmp, env=env, capture_output=True, text=True, check=False,
                )
                self.assertEqual(result.returncode, code, result.stderr)
                self.assertIn("format-check-output", (Path(tmp) / "fmt.log").read_text())

    def test_final_gate_rejects_every_non_success_outcome(self):
        step = STEPS["Fail if any step failed"]
        # Pin the GitHub expression's grammar rather than implement a partial
        # Actions expression interpreter. This also guards against accidentally
        # restoring the implicit success() guard after a prior step failure.
        self.assertEqual(
            step["if"],
            "always() && (steps.fmt.outcome != 'success' || "
            "steps.clippy.outcome != 'success' || steps.test.outcome != 'success')",
        )
        statuses = ("success", "failure", "cancelled", "skipped", "")
        for outcomes in itertools.product(statuses, repeat=3):
            with self.subTest(outcomes=outcomes):
                # The pinned expression uses only !=, && and ||, so evaluate
                # its equivalent Bash condition against 125 outcome tuples.
                condition = step["if"].replace("always()", "true")
                for name, value in zip(("fmt", "clippy", "test"), outcomes):
                    condition = condition.replace(f"steps.{name}.outcome", f"'{value}'")
                result = subprocess.run(
                    ["bash", "-c", f"if [[ {condition} ]]; then {step['run']}; fi"],
                    capture_output=True, text=True, check=False,
                )
                self.assertEqual(result.returncode, int(outcomes != ("success",) * 3))

    def test_diagnostics_report_format_outcome_even_without_log(self):
        step = STEPS["Publish diagnostics"]
        self.assertEqual(step["env"]["FMT"], "${{ steps.fmt.outcome }}")
        for status, log in (("success", ""), ("failure", "Diff in file: 100%\n"), ("skipped", None)):
            with self.subTest(status=status), tempfile.TemporaryDirectory() as tmp:
                if log is not None:
                    (Path(tmp) / "fmt.log").write_text(log)
                env = dict(os.environ, FMT=status, CLIPPY="success", TEST="success",
                           GITHUB_STEP_SUMMARY=str(Path(tmp) / "summary.md"))
                result = subprocess.run(
                    ["bash", "-e", "-c", step["run"]],
                    cwd=tmp, env=env, capture_output=True, text=True, check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(f"fmt={status} clippy=success test=success", result.stdout)
                self.assertEqual("::error title=rustfmt::" in result.stdout, status != "success")
                if status == "failure":
                    self.assertIn("100%25", result.stdout)
                if status == "skipped":
                    self.assertIn("Formatting check did not succeed.", result.stdout)


if __name__ == "__main__":
    unittest.main()
