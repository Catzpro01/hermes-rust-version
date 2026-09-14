"""CI gate regression checks, without a Rust toolchain or GitHub writes.

Run: python3 -m pip install PyYAML && python3 scripts/test_ci_workflow.py
PyYAML is a review/test dependency only, not a Hermes runtime dependency.
"""

import base64
import gzip
import hashlib
import itertools
import json
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
    def test_visual_capture_can_wait_for_review(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/visual-evidence.yml").read_text())
        steps = workflow["jobs"]["capture"]["steps"]
        plan = next(s for s in steps if s.get("id") == "plan")
        for capture in (False, True, "false"):
            with self.subTest(capture=capture), tempfile.TemporaryDirectory() as tmp:
                folder = Path(tmp) / ".scratch/hermes-rs-total-parity/runner-candidate"
                folder.mkdir(parents=True)
                (folder / "request.json").write_text(json.dumps({"phase": "none", "capture": capture}))
                output = Path(tmp) / "output"
                env = dict(os.environ, GITHUB_EVENT_NAME="push", GITHUB_OUTPUT=str(output))
                run = subprocess.run(["bash", "-e", "-c", plan["run"]], cwd=tmp, env=env,
                                     capture_output=True, text=True, check=False)
                self.assertEqual(run.returncode == 0, isinstance(capture, bool), run.stderr)
                if isinstance(capture, bool):
                    self.assertIn(f"capture={str(capture).lower()}\n", output.read_text())
        for step in steps:
            if step.get("name") in ("Build real CLI", "Capture offline startup at four terminal sizes",
                                    "Export verifiable raw capture group 1", "Export verifiable raw capture group 2"):
                self.assertEqual(step["if"], "steps.plan.outputs.phase != 'red' && steps.plan.outputs.capture == 'true'")

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

    def test_picker_plan_allows_only_named_regressions(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        steps = workflow["jobs"]["diagnose"]["steps"]
        plan = next(s for s in steps if s.get("id") == "plan")
        regression = next(s for s in steps if s.get("id") == "regression")
        self.assertEqual(regression["env"]["TEST"], "session_picker::tests::${{ steps.plan.outputs.test }}")
        for selected, accepted in [
            ("picker_footer_tracks_delete_availability", True),
            ("picker_no_match_counter_preserves_total", True),
            ("picker_footer_position", True),
            ("picker_footer_color", True),
            ("picker_normal_header", True),
            ("picker_column_layout", True),
            ("picker_message_style", True),
            ("picker_selection", True),
            ("picker_status_ink", True),
            ("picker_redraw_on_input", True),
            ("picker_terminal_size", True),
            ("unknown_test", False),
        ]:
            with self.subTest(selected=selected), tempfile.TemporaryDirectory() as tmp:
                folder = Path(tmp) / ".scratch/hermes-rs-total-parity/diagnostics/picker"
                folder.mkdir(parents=True)
                (folder / "request.json").write_text(json.dumps({"phase": "capture", "test": selected}))
                output = Path(tmp) / "output"
                env = dict(os.environ, GITHUB_OUTPUT=str(output))
                result = subprocess.run(["bash", "-e", "-c", plan["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)
                if accepted:
                    self.assertIn(f"test={selected}\n", output.read_text())

    def test_picker_position_gate_rejects_setup_errors(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "position")
        failure = "FAIL: test_footer_is_on_terminal_last_row\nRan 1 test in 0.1s\nFAILED (failures=1)"
        success = "test_footer_is_on_terminal_last_row ... ok\nRan 1 test in 0.1s\nOK"
        for phase, output, code, accepted in [
            ("red", failure, 1, True),
            ("red", "ERROR: missing dependency", 1, False),
            ("red", failure + "\nERROR: setup also failed", 1, False),
            ("green", success, 0, True),
            ("green", "Ran 0 tests\nOK", 0, False),
        ]:
            with self.subTest(phase=phase, output=output), tempfile.TemporaryDirectory() as tmp:
                python = Path(tmp) / "python3"
                python.write_text('#!/bin/bash\nprintf "%s\\n" "$FAKE_OUTPUT"\nexit "$FAKE_CODE"\n')
                python.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", PHASE=phase,
                           FAKE_OUTPUT=output, FAKE_CODE=str(code))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)

    def test_ordinary_picker_gate_requires_all_live_tests(self):
        step = STEPS["Picker terminal regressions"]
        cases = [(0, 0, 0, 0, 0, 0, 0, 0, 0, 0), (1, 0, 0, 0, 0, 0, 0, 0, 0, 0),
                 (0, 1, 0, 0, 0, 0, 0, 0, 0, 0), (0, 0, 1, 0, 0, 0, 0, 0, 0, 0),
                 (0, 0, 0, 1, 0, 0, 0, 0, 0, 0), (0, 0, 0, 0, 1, 0, 0, 0, 0, 0),
                 (0, 0, 0, 0, 0, 1, 0, 0, 0, 0), (0, 0, 0, 0, 0, 0, 1, 0, 0, 0),
                 (0, 0, 0, 0, 0, 0, 0, 1, 0, 0), (0, 0, 0, 0, 0, 0, 0, 0, 1, 0),
                 (0, 0, 0, 0, 0, 0, 0, 0, 0, 1)]
        for (position, color, header, filter_header, layout, message, selection, status,
             redraw, size) in cases:
            with self.subTest(position=position, color=color, header=header,
                              filter_header=filter_header, layout=layout,
                              message=message, selection=selection,
                              status=status, redraw=redraw,
                              size=size), tempfile.TemporaryDirectory() as tmp:
                cargo = Path(tmp) / "cargo"
                cargo.write_text("#!/bin/sh\necho build-ok\n")
                cargo.chmod(0o700)
                python = Path(tmp) / "python3"
                python.write_text('#!/bin/bash\ncase "$*" in\n  *test_picker_footer_position.py*) echo position-test; exit "$POSITION";;\n  *test_picker_footer_color.py*) echo color-test; exit "$COLOR";;\n  *test_picker_normal_header.py*) echo header-test; exit "$HEADER";;\n  *test_picker_filter_header.py*) echo filter-header-test; exit "$FILTER_HEADER";;\n  *test_picker_column_layout.py*) echo column-layout-test; exit "$LAYOUT";;\n  *test_picker_message_style.py*) echo message-style-test; exit "$MESSAGE";;\n  *test_picker_selection.py*) echo selection-test; exit "$SELECTION";;\n  *test_picker_status_ink.py*) echo status-ink-test; exit "$STATUS";;\n  *test_picker_redraw_on_input.py*) echo redraw-test; exit "$REDRAW";;\n  *test_picker_terminal_size.py*) echo size-test; exit "$SIZE";;\n  *) exit 0;;\nesac\n')
                python.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}",
                           POSITION=str(position), COLOR=str(color), HEADER=str(header),
                           FILTER_HEADER=str(filter_header), LAYOUT=str(layout),
                           MESSAGE=str(message), SELECTION=str(selection), STATUS=str(status),
                           REDRAW=str(redraw), SIZE=str(size))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode,
                                 int(bool(position or color or header or filter_header or layout
                                          or message or selection or status or redraw or size)),
                                 result.stdout + result.stderr)
                log = (Path(tmp) / "picker-position.log").read_text()
                self.assertIn("position-test", log)
                if not position:
                    self.assertIn("color-test", log)
                    if not color:
                        self.assertIn("header-test", log)
                        if not header:
                            self.assertIn("filter-header-test", log)
                            if not filter_header:
                                self.assertIn("column-layout-test", log)
                                self.assertIn("test_picker_column_layout.py", step["run"])
                                if not layout:
                                    self.assertIn("message-style-test", log)
                                    self.assertIn("test_picker_message_style.py", step["run"])
                                    if not message:
                                        self.assertIn("selection-test", log)
                                        self.assertIn("test_picker_selection.py", step["run"])
                                        if not selection:
                                            self.assertIn("status-ink-test", log)
                                            self.assertIn("test_picker_status_ink.py", step["run"])
                                            if not status:
                                                self.assertIn("redraw-test", log)
                                                self.assertIn("test_picker_redraw_on_input.py", step["run"])
                                                if not redraw:
                                                    self.assertIn("size-test", log)
                                                    self.assertIn("test_picker_terminal_size.py", step["run"])

    def test_picker_size_gate_rejects_setup_errors(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "size")
        self.assertIn("!= 'reference'", step["if"],
                      "the reference phase must not be judged by the live Rust gate")
        failure = ("FAIL: test_size_contract_follows_the_pinned_threshold\n"
                   "Ran 1 test in 0.1s\nFAILED (failures=1)")
        success = "test_size_contract_follows_the_pinned_threshold ... ok\nRan 1 test in 0.1s\nOK"
        for phase, output, code, accepted in [
            ("red", failure, 1, True),
            ("red", "ERROR: missing dependency", 1, False),
            ("red", failure + "\nERROR: setup also failed", 1, False),
            ("green", success, 0, True),
            ("green", "Ran 0 tests\nOK", 0, False),
        ]:
            with self.subTest(phase=phase, output=output), tempfile.TemporaryDirectory() as tmp:
                python = Path(tmp) / "python3"
                python.write_text('#!/bin/bash\nprintf "%s\\n" "$FAKE_OUTPUT"\nexit "$FAKE_CODE"\n')
                python.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", PHASE=phase,
                           FAKE_OUTPUT=output, FAKE_CODE=str(code))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)

    def test_runner_environment_is_not_assumed_to_be_fresh(self):
        """Self-hosted runners persist /tmp and may already have a toolchain."""
        for name in ("ci.yml", "picker-diagnostic.yml"):
            text = (ROOT / ".github/workflows" / name).read_text()
            self.assertNotIn("/tmp/picker-qa", text, name)
            self.assertIn("RUNNER_TEMP", text, name)
            self.assertIn("rm -rf \"$QA\"", text, name)
            self.assertIn("--break-system-packages", text, name)
            self.assertIn('command -v "$tool"', text, name)
            workflow = yaml.safe_load(text)
            for job_name, job in workflow["jobs"].items():
                for step in job["steps"]:
                    script = step.get("run") or ""
                    if "$QA" in script:
                        self.assertIn("RUNNER_TEMP", script,
                                      f"{name}:{job_name}:{step.get('name')} uses $QA without defining it")

    def test_step_expressions_are_balanced(self):
        """GitHub rejects the whole file for one malformed expression, and the run
        then appears named after the file path with no jobs (runs 34879063271,
        34878422065). Cheap local guard for the two mistakes actually made:
        unbalanced parentheses and an odd number of single quotes."""
        for name in ("ci.yml", "picker-diagnostic.yml", "ui-evidence.yml", "visual-evidence.yml"):
            workflow = yaml.safe_load((ROOT / ".github/workflows" / name).read_text())
            for job_name, job in workflow["jobs"].items():
                for step in job["steps"]:
                    expression = str(step.get("if", ""))
                    label = f"{name}:{job_name}:{step.get('name')}"
                    self.assertEqual(expression.count("("), expression.count(")"),
                                     f"{label}: unbalanced parentheses in if")
                    self.assertEqual(expression.count("'") % 2, 0,
                                     f"{label}: odd number of quotes in if")

    def test_workflows_avoid_contexts_github_rejects(self):
        """`runner` is not available at job level; using it invalidates the file
        (both workflows were rejected in run 34878423059/34878422065)."""
        import re
        allowed = {"github", "needs", "strategy", "matrix", "vars", "secrets", "inputs"}
        for name in ("ci.yml", "picker-diagnostic.yml", "ui-evidence.yml", "visual-evidence.yml"):
            workflow = yaml.safe_load((ROOT / ".github/workflows" / name).read_text())
            for job_name, job in workflow["jobs"].items():
                texts = [str(job.get("env", "")), str(job.get("if", ""))]
                for text in texts:
                    for context in re.findall(r"\$\{\{\s*([a-z_]+)\.", text):
                        self.assertIn(context, allowed,
                                      f"{name}:{job_name} uses the {context!r} context at job level")

    def test_only_green_runs_the_validation_and_patch_export(self):
        """A reference/capture phase must not validate GREEN or export an empty patch."""
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        steps = workflow["jobs"]["diagnose"]["steps"]
        for name in ("Full GREEN validation", "Export exactly tested proposal"):
            step = next(s for s in steps if s.get("name") == name)
            self.assertIn("== 'green'", step["if"], name)
        export = next(s for s in steps if s.get("name") == "Export exactly tested proposal")
        # Live gates used to be excluded, so the tree they verified (post-`cargo
        # fmt --all`) was never exported; cycle 7 GREEN 2 needed exactly that.
        for live in ("picker_status_ink", "picker_redraw_on_input", "picker_terminal_size"):
            self.assertNotIn(live, export["if"], live)
        log = next(s for s in steps if s.get("name") == "Export exact regression log")
        self.assertIn("REF_PREPARE", log["env"])
        self.assertIn("reference_failed", log["run"])

    def test_reference_steps_log_and_are_exported(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        steps = workflow["jobs"]["diagnose"]["steps"]
        prepare = next(s for s in steps if s.get("name") == "Prepare the pinned Python reference")
        record = next(s for s in steps if s.get("name") == "Record the reference side of the size contract")
        export = next(s for s in steps if s.get("name") == "Export exact regression log")
        for step in (prepare, record):
            self.assertIn("== 'reference'", step["if"])
            self.assertIn("tee", step["run"], step["name"])
            self.assertIn("89cde75d388ae3ff0d3512c00a0b4e77fe9f55c438874032b32967b4ab867567",
                          step["run"] + prepare["run"])
        self.assertIn("reference-prepare.log", export["run"])
        self.assertIn("reference-child.log", export["run"])
        self.assertIn("requirements-banner.txt", record["run"],
                      "the reference child needs the minimal upstream environment")
        upload = next(s for s in steps if s.get("uses", "").startswith("actions/upload-artifact"))
        for name in ("reference-prepare.log", "reference-child.log"):
            self.assertIn(name, upload["with"]["path"])

    def test_reference_phase_is_limited_to_the_size_gate(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        plan = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "plan")
        script = plan["run"].split("<<'PY'", 1)[1].split("\nPY", 1)[0]
        script = "\n".join(line[10:] if line.startswith(" " * 10) else line
                            for line in script.splitlines())
        for test, phase, accepted in [("picker_terminal_size", "reference", True),
                                      ("picker_redraw_on_input", "reference", False),
                                      ("picker_terminal_size", "capture", True)]:
            with self.subTest(test=test, phase=phase), tempfile.TemporaryDirectory() as tmp:
                folder = Path(tmp) / ".scratch/hermes-rs-total-parity/diagnostics/picker"
                folder.mkdir(parents=True)
                (folder / "request.json").write_text(json.dumps({"phase": phase, "test": test}))
                output = Path(tmp) / "gh_output"
                env = dict(os.environ, GITHUB_OUTPUT=str(output))
                result = subprocess.run(["python3", "-c", script], cwd=tmp, env=env,
                                        capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)
                if accepted:
                    # The step conditions key off these outputs; a reference phase that
                    # returns before writing them selects the wrong steps (run 34876400782).
                    self.assertEqual(output.read_text(), f"phase={phase}\ntest={test}\n")

    def test_picker_color_gate_rejects_setup_errors(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "color")
        failure = "FAIL: test_footer_uses_reference_grey\nRan 1 test in 0.1s\nFAILED (failures=1)"
        success = "test_footer_uses_reference_grey ... ok\nRan 1 test in 0.1s\nOK"
        for phase, output, code, accepted in [
            ("red", failure, 1, True),
            ("red", "ERROR: missing dependency", 1, False),
            ("red", failure + "\nERROR: setup also failed", 1, False),
            ("green", success, 0, True),
            ("green", "Ran 0 tests\nOK", 0, False),
        ]:
            with self.subTest(phase=phase, output=output), tempfile.TemporaryDirectory() as tmp:
                python = Path(tmp) / "python3"
                python.write_text('#!/bin/bash\nprintf "%s\\n" "$FAKE_OUTPUT"\nexit "$FAKE_CODE"\n')
                python.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", PHASE=phase,
                           FAKE_OUTPUT=output, FAKE_CODE=str(code))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)

    def test_picker_header_gate_rejects_setup_errors(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "header")
        failure = "FAIL: test_normal_help_header_matches_reference_style\nRan 1 test in 0.1s\nFAILED (failures=1)"
        success = "test_normal_help_header_matches_reference_style ... ok\nRan 1 test in 0.1s\nOK"
        for phase, output, code, accepted in [
            ("red", failure, 1, True),
            ("red", "ERROR: missing dependency", 1, False),
            ("red", failure + "\nERROR: setup also failed", 1, False),
            ("green", success, 0, True),
            ("green", "Ran 0 tests\nOK", 0, False),
        ]:
            with self.subTest(phase=phase, output=output), tempfile.TemporaryDirectory() as tmp:
                python = Path(tmp) / "python3"
                python.write_text('#!/bin/bash\nprintf "%s\\n" "$FAKE_OUTPUT"\nexit "$FAKE_CODE"\n')
                python.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", PHASE=phase,
                           FAKE_OUTPUT=output, FAKE_CODE=str(code))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)

    def test_picker_filter_header_gate_rejects_setup_errors(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "filter_header")
        failure = "FAIL: test_filter_help_header_matches_reference_style\nRan 1 test in 0.1s\nFAILED (failures=1)"
        success = "test_filter_help_header_matches_reference_style ... ok\nRan 1 test in 0.1s\nOK"
        for phase, output, code, accepted in [
            ("red", failure, 1, True),
            ("red", "ERROR: missing dependency", 1, False),
            ("red", failure + "\nERROR: setup also failed", 1, False),
            ("green", success, 0, True),
            ("green", "Ran 0 tests\nOK", 0, False),
        ]:
            with self.subTest(phase=phase, output=output), tempfile.TemporaryDirectory() as tmp:
                python = Path(tmp) / "python3"
                python.write_text('#!/bin/bash\nprintf "%s\\n" "$FAKE_OUTPUT"\nexit "$FAKE_CODE"\n')
                python.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", PHASE=phase,
                           FAKE_OUTPUT=output, FAKE_CODE=str(code))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)

    def test_picker_redraw_gate_rejects_setup_errors(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "redraw")
        failure = "FAIL: test_picker_draws_on_input_not_on_its_poll_timeout\nRan 1 test in 0.1s\nFAILED (failures=1)"
        success = "test_picker_draws_on_input_not_on_its_poll_timeout ... ok\nRan 1 test in 0.1s\nOK"
        for phase, output, code, accepted in [
            ("red", failure, 1, True),
            ("red", "ERROR: missing dependency", 1, False),
            ("red", failure + "\nERROR: setup also failed", 1, False),
            ("green", success, 0, True),
            ("green", "Ran 0 tests\nOK", 0, False),
        ]:
            with self.subTest(phase=phase, output=output), tempfile.TemporaryDirectory() as tmp:
                python = Path(tmp) / "python3"
                python.write_text('#!/bin/bash\nprintf "%s\\n" "$FAKE_OUTPUT"\nexit "$FAKE_CODE"\n')
                python.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", PHASE=phase,
                           FAKE_OUTPUT=output, FAKE_CODE=str(code))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)

    def test_picker_status_ink_gate_rejects_setup_errors(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "status_ink")
        failure = "FAIL: test_status_tag_ink_matches_the_pinned_mapping\nRan 1 test in 0.1s\nFAILED (failures=1)"
        success = "test_status_tag_ink_matches_the_pinned_mapping ... ok\nRan 1 test in 0.1s\nOK"
        for phase, output, code, accepted in [
            ("red", failure, 1, True),
            ("red", "ERROR: missing dependency", 1, False),
            ("red", failure + "\nERROR: setup also failed", 1, False),
            ("green", success, 0, True),
            ("green", "Ran 0 tests\nOK", 0, False),
        ]:
            with self.subTest(phase=phase, output=output), tempfile.TemporaryDirectory() as tmp:
                python = Path(tmp) / "python3"
                python.write_text('#!/bin/bash\nprintf "%s\\n" "$FAKE_OUTPUT"\nexit "$FAKE_CODE"\n')
                python.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", PHASE=phase,
                           FAKE_OUTPUT=output, FAKE_CODE=str(code))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)

    def test_picker_selection_gate_rejects_setup_errors(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "selection")
        failure = "FAIL: test_selected_row_matches_reference_style\nRan 1 test in 0.1s\nFAILED (failures=1)"
        success = "test_selected_row_matches_reference_style ... ok\nRan 1 test in 0.1s\nOK"
        for phase, output, code, accepted in [
            ("red", failure, 1, True),
            ("red", "ERROR: missing dependency", 1, False),
            ("red", failure + "\nERROR: setup also failed", 1, False),
            ("green", success, 0, True),
            ("green", "Ran 0 tests\nOK", 0, False),
        ]:
            with self.subTest(phase=phase, output=output), tempfile.TemporaryDirectory() as tmp:
                python = Path(tmp) / "python3"
                python.write_text('#!/bin/bash\nprintf "%s\\n" "$FAKE_OUTPUT"\nexit "$FAKE_CODE"\n')
                python.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", PHASE=phase,
                           FAKE_OUTPUT=output, FAKE_CODE=str(code))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)

    def test_picker_message_style_gate_rejects_setup_errors(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "message_style")
        failure = "FAIL: test_prompt_and_no_match_match_reference_style\nRan 1 test in 0.1s\nFAILED (failures=1)"
        success = "test_prompt_and_no_match_match_reference_style ... ok\nRan 1 test in 0.1s\nOK"
        for phase, output, code, accepted in [
            ("red", failure, 1, True),
            ("red", "ERROR: missing dependency", 1, False),
            ("red", failure + "\nERROR: setup also failed", 1, False),
            ("green", success, 0, True),
            ("green", "Ran 0 tests\nOK", 0, False),
        ]:
            with self.subTest(phase=phase, output=output), tempfile.TemporaryDirectory() as tmp:
                python = Path(tmp) / "python3"
                python.write_text('#!/bin/bash\nprintf "%s\\n" "$FAKE_OUTPUT"\nexit "$FAKE_CODE"\n')
                python.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", PHASE=phase,
                           FAKE_OUTPUT=output, FAKE_CODE=str(code))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)

    def test_picker_column_layout_gate_rejects_setup_errors(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "column_layout")
        failure = "FAIL: test_column_layout_matches_pinned_reference\nRan 1 test in 0.1s\nFAILED (failures=1)"
        success = "test_column_layout_matches_pinned_reference ... ok\nRan 1 test in 0.1s\nOK"
        for phase, output, code, accepted in [
            ("red", failure, 1, True),
            ("red", "ERROR: missing dependency", 1, False),
            ("red", failure + "\nERROR: setup also failed", 1, False),
            ("green", success, 0, True),
            ("green", "Ran 0 tests\nOK", 0, False),
        ]:
            with self.subTest(phase=phase, output=output), tempfile.TemporaryDirectory() as tmp:
                python = Path(tmp) / "python3"
                python.write_text('#!/bin/bash\nprintf "%s\\n" "$FAKE_OUTPUT"\nexit "$FAKE_CODE"\n')
                python.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", PHASE=phase,
                           FAKE_OUTPUT=output, FAKE_CODE=str(code))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stdout + result.stderr)

    def test_picker_gate_rejects_compile_errors_and_missing_regression(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"] if s.get("id") == "regression")
        selected = "session_picker::tests::picker_footer_tracks_delete_availability"
        for phase, output, code, accepted in [
            ("red", f"test {selected} ... FAILED", 101, True),
            ("red", "error: could not compile", 101, False),
            ("red", "0 tests", 0, False),
            ("green", f"test {selected} ... ok", 0, True),
            ("green", "0 tests", 0, False),
            ("capture", f"test {selected} ... ok", 0, True),
        ]:
            with self.subTest(phase=phase, output=output), tempfile.TemporaryDirectory() as tmp:
                cargo = Path(tmp) / "cargo"
                cargo.write_text('#!/bin/bash\nprintf "%s\\n" "$FAKE_OUTPUT"\nexit "$FAKE_CODE"\n')
                cargo.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}", PHASE=phase,
                           TEST=selected, FAKE_OUTPUT=output, FAKE_CODE=str(code))
                run = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                     env=env, capture_output=True, text=True)
                self.assertEqual(run.returncode == 0, accepted, run.stdout + run.stderr)

    def test_workflows_use_approved_readonly_artifact_policy(self):
        allowed = {
            "actions/checkout", "actions/setup-python", "actions/upload-artifact",
            "dtolnay/rust-toolchain", "Swatinem/rust-cache",
        }
        for name in ("ci.yml", "visual-evidence.yml", "ui-evidence.yml", "picker-diagnostic.yml"):
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
            "steps.clippy.outcome != 'success' || steps.test.outcome != 'success' || "
            "steps.picker.outcome != 'success')",
        )
        statuses = ("success", "failure", "cancelled", "skipped", "")
        for outcomes in itertools.product(statuses, repeat=4):
            with self.subTest(outcomes=outcomes):
                # The pinned expression uses only !=, && and ||, so evaluate
                # its equivalent Bash condition against 625 outcome tuples.
                condition = step["if"].replace("always()", "true")
                for name, value in zip(("fmt", "clippy", "test", "picker"), outcomes):
                    condition = condition.replace(f"steps.{name}.outcome", f"'{value}'")
                result = subprocess.run(
                    ["bash", "-c", f"if [[ {condition} ]]; then {step['run']}; fi"],
                    capture_output=True, text=True, check=False,
                )
                self.assertEqual(result.returncode, int(outcomes != ("success",) * 4))

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
                env = dict(os.environ, FMT=status, CLIPPY="success", TEST="success", PICKER="success",
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


    def test_picker_type_check_reports_the_first_error(self):
        # The diagnostic job's `cargo check` used to fail with an invisible exit
        # code 101 (cycle-7 GREEN attempt, run 34879520834). Whatever the cause,
        # the annotation must carry the first compiler error line.
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        step = next(s for s in workflow["jobs"]["diagnose"]["steps"]
                    if s.get("name") == "Format and type check")
        for code, log, expected in (
            (0, "    Finished dev profile target(s)\n", None),
            (101, "error[E0308]: mismatched types\n  --> src/lib.rs:1:1\n", "error[E0308]: mismatched types"),
            (101, "warning: unused\n", "cargo check failed"),
        ):
            with self.subTest(code=code), tempfile.TemporaryDirectory() as tmp:
                cargo = Path(tmp) / "cargo"
                cargo.write_text("#!/bin/sh\n[ \"$1\" = fmt ] && exit 0\n"
                                 "printf '%s' \"$CARGO_LOG\"\nexit $CARGO_CODE\n")
                cargo.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}",
                           CARGO_LOG=log, CARGO_CODE=str(code))
                result = subprocess.run(["bash", "-e", "-c", step["run"]],
                                        cwd=tmp, env=env, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, code, result.stderr)
                if expected is None:
                    self.assertNotIn("::error title=cargo check failed::", result.stdout)
                else:
                    self.assertIn(f"::error title=cargo check failed::{expected}", result.stdout)
                self.assertEqual((Path(tmp) / "picker-check.log").read_text(), log)

    def test_capture_scripts_get_the_qa_path(self):
        # Cycle-7 capture attempt 1 (run 34880380735) died with `No module named
        # 'pyte'`: the size capture calls were the only capture invocations missing
        # the venv prefix, and `capture_ui.record` imports pyte at that point.
        workflow = yaml.safe_load((ROOT / ".github/workflows/picker-diagnostic.yml").read_text())
        steps = workflow["jobs"]["diagnose"]["steps"]
        seen = 0
        for step in steps:
            for line in str(step.get("run", "")).splitlines():
                for script in ("capture_picker_sizes.py", "capture_picker_diagnostic.py"):
                    if f"scripts/{script}" in line:
                        seen += 1
                        self.assertIn('PYTHONPATH="$QA" python3', line, f"{step['name']}: {line.strip()}")
        self.assertGreaterEqual(seen, 4, "the capture invocations disappeared")

    def test_diagnostics_still_report_a_real_test_build_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "test.log").write_text("error[E0425]: unknown value\n")
            env = dict(os.environ, FMT="success", CLIPPY="success", TEST="failure", PICKER="success",
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

    def test_sccache_enabled_only_when_present(self):
        """The wrapper must be exported exactly when sccache exists on the
        runner, so hosted runners without it keep building unchanged."""
        step = STEPS["Enable sccache if available"]
        for present in (True, False):
            with self.subTest(present=present), tempfile.TemporaryDirectory() as tmp:
                if present:
                    sccache = Path(tmp) / "sccache"
                    sccache.write_text("#!/bin/sh\nexit 0\n")
                    sccache.chmod(0o700)
                env_file = Path(tmp) / "gh_env"
                out_file = Path(tmp) / "gh_output"
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}",
                           GITHUB_ENV=str(env_file), GITHUB_OUTPUT=str(out_file))
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                written = env_file.read_text() if env_file.exists() else ""
                self.assertEqual("RUSTC_WRAPPER=sccache" in written, present, written)
                if present:
                    self.assertIn("SCCACHE_IDLE_TIMEOUT=0", written)
                    self.assertIn("::notice title=sccache enabled::", result.stdout)
                else:
                    self.assertIn("::notice title=sccache::", result.stdout)
                output = out_file.read_text() if out_file.exists() else ""
                self.assertIn(f"sccache={'present' if present else 'absent'}", output)

    def test_memory_diagnostics_report_oom_only_when_present(self):
        """The small self-hosted VPS needs OOM visibility, but unreadable
        dmesg or missing tools must never fail the job."""
        step = STEPS["Record memory and OOM diagnostics"]
        self.assertEqual(step["if"], "always()")
        for oom in (True, False):
            with self.subTest(oom=oom), tempfile.TemporaryDirectory() as tmp:
                dmesg_line = ("echo '[91234.5] Out of memory: Killed process 4242 (hermes-rs)'"
                              if oom else "echo '[1.0] normal boot message'")
                for name, body in (
                        ("dmesg", f"#!/bin/sh\n{dmesg_line}\n"),
                        ("free", "#!/bin/sh\necho '              total        used        free'\necho 'Mem:           1977        1400         200'\n"),
                        ("swapon", "#!/bin/sh\necho 'NAME      TYPE SIZE USED PRIO'\necho '/swapfile file   4G   0B  -2'\n"),
                        ("sccache", "#!/bin/sh\necho 'Cache hits: 12'\n")):
                    tool = Path(tmp) / name
                    tool.write_text(body)
                    tool.chmod(0o700)
                env = dict(os.environ, PATH=f"{tmp}:{os.environ['PATH']}")
                result = subprocess.run(["bash", "-e", "-c", step["run"]], cwd=tmp,
                                        env=env, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                log = (Path(tmp) / "memory-oom.log").read_text()
                self.assertIn("== free -m ==", log)
                self.assertIn("== dmesg OOM kills ==", log)
                self.assertIn("== linker ==", log)
                self.assertEqual("::error title=OOM kill detected::" in result.stdout, oom,
                                 result.stdout)
                self.assertIn("::notice title=memory::mem 1400MiB used of 1977MiB",
                              result.stdout)

    def test_memory_log_is_included_in_the_uploaded_logs(self):
        step = STEPS["Upload logs"]
        self.assertIn("memory-oom.log", step["with"]["path"])

    def test_jobs_run_on_the_tuned_self_hosted_vps(self):
        """User instruction 2026-09-15: this repo's jobs must always run on
        the tuned self-hosted VPS runner (sccache + mold), never the shared
        ubuntu-latest pool."""
        for name in ("ci.yml", "picker-diagnostic.yml", "ui-evidence.yml", "visual-evidence.yml"):
            with self.subTest(workflow=name):
                workflow = yaml.safe_load((ROOT / ".github/workflows" / name).read_text())
                for job_name, job in workflow["jobs"].items():
                    self.assertEqual(job.get("runs-on"), ["self-hosted", "vps", "hermes"],
                                     f"{name}:{job_name} must target the tuned VPS runner")


if __name__ == "__main__":
    unittest.main()
