"""CI gate regression checks, without a Rust toolchain or GitHub writes.

Run: python3 -m pip install PyYAML && python3 scripts/test_ci_workflow.py
PyYAML is a review/test dependency only, not a Hermes runtime dependency.
"""

import base64
import gzip
import hashlib
import itertools
import re
import random
import string
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
    def test_visual_gate_requires_exact_selected_regression(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/visual-evidence.yml").read_text())
        step = next(s for s in workflow["jobs"]["capture"]["steps"] if s.get("id") == "regression")
        self.assertEqual(step["env"]["TEST"], "tui::welcome::tests::${{ steps.plan.outputs.test }}")
        selected = "tui::welcome::tests::banner_ansi_long_session_columns_match_python"
        for phase, name, result, code, accepted in [
            ("red", selected, "FAILED", 101, True),
            ("red", "other_test", "FAILED", 101, False),
            ("red", "no_matching_test", "ok", 0, False),
            ("green", selected, "ok", 0, True),
            ("green", "other_test", "ok", 0, False),
            ("green", selected, "FAILED", 101, False),
        ]:
            with self.subTest(phase=phase, name=name, result=result), tempfile.TemporaryDirectory() as tmp:
                cargo = Path(tmp) / "cargo"
                cargo.write_text('#!/bin/bash\nprintf "%s\\n" "$@" > args\nprintf "test %s ... %s\\n" "$CASE_NAME" "$CASE_RESULT"\nexit "$CASE_CODE"\n')
                cargo.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", TEST=selected,
                           PHASE=phase, CASE_NAME=name, CASE_RESULT=result, CASE_CODE=str(code))
                run = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp, env=env,
                                     capture_output=True, text=True, check=False)
                self.assertEqual(run.returncode == 0, accepted, run.stdout + run.stderr)
                args = (Path(tmp) / "args").read_text().splitlines()
                self.assertIn(selected, args)
                self.assertEqual(args[-2:], ["--", "--exact"])

    def test_workflows_use_approved_readonly_artifact_policy(self):
        allowed = {
            "actions/checkout", "actions/setup-python", "actions/upload-artifact",
            "dtolnay/rust-toolchain", "Swatinem/rust-cache",
        }
        for name in ("ci.yml", "visual-evidence.yml"):
            with self.subTest(workflow=name):
                workflow = yaml.safe_load((ROOT / ".github/workflows" / name).read_text())
                self.assertEqual(workflow.get("permissions"), {"contents": "read"})
                uploads = 0
                for job in workflow["jobs"].values():
                    if "permissions" in job:
                        self.assertEqual(job["permissions"], {"contents": "read"})
                    for step in job["steps"]:
                        if "uses" not in step:
                            continue
                        self.assertRegex(step["uses"], r"^[^@]+@[0-9a-f]{40}$")
                        action = step["uses"].split("@", 1)[0]
                        self.assertIn(action, allowed)
                        if action == "actions/upload-artifact":
                            uploads += 1
                            self.assertEqual(step.get("with", {}).get("retention-days"), 90)
                self.assertGreater(uploads, 0, "Artifact policy must cover an actual upload")

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
                (Path(tmp) / "test.log").write_text(
                    "error: interrupted\ntest smoke_sigint_returns_130 ... ok\n"
                    "test result: ok. 1 passed; 0 failed;\n"
                )
                env = dict(os.environ, FMT=status, CLIPPY="success", TEST="success",
                           GITHUB_STEP_SUMMARY=str(Path(tmp) / "summary.md"))
                result = subprocess.run(
                    ["bash", "-e", "-c", step["run"]],
                    cwd=tmp, env=env, capture_output=True, text=True, check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertNotIn("::error title=test-build::", result.stdout)
                self.assertIn(f"fmt={status} clippy=success test=success", result.stdout)
                self.assertEqual("::error title=rustfmt::" in result.stdout, status != "success")
                if status == "failure":
                    self.assertIn("100%25", result.stdout)
                if status == "skipped":
                    self.assertIn("Formatting check did not succeed.", result.stdout)


    def test_diagnostics_still_report_a_real_test_build_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "test.log").write_text("error[E0425]: unknown value\n")
            env = dict(os.environ, FMT="success", CLIPPY="success", TEST="failure",
                       GITHUB_STEP_SUMMARY=str(Path(tmp) / "summary.md"))
            result = subprocess.run(
                ["bash", "-e", "-c", STEPS["Publish diagnostics"]["run"]],
                cwd=tmp, env=env, capture_output=True, text=True, check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("::error title=test-build::error[E0425]", result.stdout)

    def test_format_patch_roundtrip_excludes_non_rust_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            subprocess.run(["git", "init", "--quiet", tmp], check=True)
            root = Path(tmp)
            (root / "main.rs").write_text("fn main(){}\n")
            (root / "config.yaml").write_text("old: value\n")
            subprocess.run(["git", "add", "."], cwd=tmp, check=True)
            # Force several incompressible chunks to catch annotation/API
            # truncation that a tiny one-line formatting fixture misses.
            noise = "".join(random.Random(42).choices(string.ascii_letters, k=30000))
            (root / "main.rs").write_text("fn main() {}\n// " + noise + "\n")
            (root / "config.yaml").write_text("private: do-not-export\n")
            output = ""
            for group in range(5):
                result = subprocess.run(
                    ["python3", str(ROOT / "scripts/export_format_patch.py"), "--group", str(group)],
                    cwd=tmp, capture_output=True, text=True, check=True,
                )
                self.assertLessEqual(result.stdout.count("::notice"), 9)
                output += result.stdout
            chunks = re.findall(r"::notice title=rustfmt patch \d+/\d+::(.*)", output)
            self.assertGreater(len(chunks), 8)
            self.assertTrue(all(len(chunk) <= 3000 for chunk in chunks))
            patch = gzip.decompress(base64.b64decode("".join(chunks)))
            self.assertEqual(patch, (root / "fmt.patch").read_bytes())
            self.assertIn(hashlib.sha256(patch).hexdigest(), output)
            self.assertIn(b"main.rs", patch)
            self.assertNotIn(b"config.yaml", patch)
            self.assertNotIn(b"do-not-export", patch)
            subprocess.run(["git", "apply", "--check", "--reverse", "fmt.patch"], cwd=tmp, check=True)


if __name__ == "__main__":
    unittest.main()
