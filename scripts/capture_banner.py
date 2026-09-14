"""Capture real banner output over a PTY; no UI strings are reconstructed.

Initial evidence slice only: not wizard/picker/completion proof or a parity gate.
Rust uses its real offline REPL; Python uses the upstream public banner renderer
with explicit equivalent data, not a full Python CLI launch. Originals are kept.
"""
import argparse
import base64
import errno
import fcntl
import gzip
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import signal
import sqlite3
import struct
import subprocess
import sys
import tempfile
import termios
import time

UPSTREAM = "63279301bcbdc185c1b07b98a9312eb0c862f26d"
WIDTHS = (100, 80, 94, 95)
TOOLS = ["list_dir", "read_file", "shell_readonly", "write_file"]
WELCOME = b"Welcome to Hermes Agent! Type your message or /help for commands."


def capture(command, home, cwd, width, extra_env=None, ready=None):
    env = {"PATH": os.defpath, "HOME": str(home), "HERMES_HOME": str(home),
           "TERM": "xterm-256color", "COLORTERM": "truecolor", "LANG": "C.UTF-8",
           "LC_ALL": "C.UTF-8", "PYTHONDONTWRITEBYTECODE": "1"}
    env.update(extra_env or {})
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, width, 0, 0))

    def controlling_terminal():
        os.setsid()
        fcntl.ioctl(slave, termios.TIOCSCTTY, 0)

    proc = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave,
                            cwd=cwd, env=env, preexec_fn=controlling_terminal)
    os.close(slave)
    start = time.monotonic()
    events, output = [], bytearray()
    reached = False
    try:
        while time.monotonic() - start < 20:
            if select.select([master], [], [], 0.1)[0]:
                try:
                    data = os.read(master, 65536)
                except OSError as error:
                    if error.errno == errno.EIO:
                        break
                    raise
                if not data:
                    break
                events.append([round(time.monotonic() - start, 6), base64.b64encode(data).decode()])
                output.extend(data)
                if len(output) > 2_000_000:
                    raise RuntimeError("Capture output limit exceeded")
                if ready and ready in output:
                    reached = True
                    break
            elif proc.poll() is not None:
                break
        if ready and not reached:
            raise RuntimeError("REPL readiness marker missing: " + bytes(output[-2000:]).decode(errors="replace"))
        if not ready:
            if proc.wait(timeout=2) != 0:
                raise RuntimeError("Python renderer failed: " + bytes(output[-2000:]).decode(errors="replace"))
        end = bytes(output).index(ready) if ready else len(output)
        return {"command": command, "environment": env, "cwd": str(cwd),
                "width": width, "height": 30, "events_base64": events,
                "raw_base64": base64.b64encode(output).decode(),
                "raw_sha256": hashlib.sha256(output).hexdigest(),
                "snapshot_end_byte": end,
                "snapshot_rule": "prefix before REPL welcome" if ready else "renderer process exit",
                "termination": "SIGTERM after snapshot" if ready else "exit 0"}
    finally:
        if proc.poll() is None:
            os.killpg(proc.pid, signal.SIGTERM)
            try:
                proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait()
        os.close(master)


def rust_capture(binary):
    cases = []
    for width in WIDTHS:
        with tempfile.TemporaryDirectory(prefix="hermes-visual-") as temp:
            home = Path(temp) / "home"
            cwd = Path(temp) / "demo"
            home.mkdir()
            cwd.mkdir()
            (home / "config.yaml").write_text("model:\n  provider: auto\n  name: parity-fixture\n")
            result = capture([str(binary), "--provider", "fake"], home, cwd, width, ready=WELCOME)
            with sqlite3.connect(f"file:{home / 'state.db'}?mode=ro", uri=True) as db:
                session_id, = db.execute("SELECT id FROM sessions").fetchone()
            # Explicit input fixture for the Python display component. Tool names
            # are the four registrations in repl.rs, not inferred from a screenshot.
            fixture = {"model": "parity-fixture", "cwd": str(cwd), "session_id": session_id,
                       "tools": TOOLS, "skills": {}, "availability": {}}
            cases.append({"id": f"banner-{width}x30", "fixture": fixture, "rust": result})
    return {"schema": 1, "status": "CAPTURED_NOT_REVIEWED", "scope": "banner first slice",
            "rust_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
            "rust_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            "rustc_version": subprocess.check_output(["rustc", "--version"], text=True).strip(),
            "cargo_version": subprocess.check_output(["cargo", "--version"], text=True).strip(),
            "rust_worktree_diff_sha256": hashlib.sha256(subprocess.check_output(
                ["git", "diff", "--binary", "--", "*.rs"])).hexdigest(),
            "rust_banner_source_sha256": hashlib.sha256(Path(
                "crates/hermes-cli/src/tui/welcome.rs").read_bytes()).hexdigest(),
            "python_reference": UPSTREAM, "cases": cases}


def python_child(reference, fixture_path):
    sys.path.insert(0, str(reference))
    from rich.console import Console
    from hermes_cli.banner import build_welcome_banner
    fixture = json.loads(fixture_path.read_text())
    build_welcome_banner(Console(), model=fixture["model"], cwd=fixture["cwd"],
                         session_id=fixture["session_id"],
                         tools=[{"function": {"name": name}} for name in fixture["tools"]],
                         get_toolset_for_tool=lambda name: "other",
                         availability=fixture["availability"],
                         skills_by_category=fixture["skills"])


def pair_with_python(bundle, reference):
    # This is a newly downloaded isolated archive, NEVER the installed Python home.
    # Its supported build metadata identifies the verified archive commit without
    # changing Python source or pretending an archive contains live git refs.
    lock = json.loads((Path(__file__).resolve().parents[1] /
                       "docs/hermes-ui-spec/017/evidence/reference.json").read_text())
    marker = reference / ".visual-evidence-reference"
    if not marker.is_file() or marker.read_text().strip() != UPSTREAM:
        raise RuntimeError("Not a prepared isolated reference; refusing to write metadata")
    for name, expected in lock["source_files_sha256"].items():
        if hashlib.sha256((reference / name).read_bytes()).hexdigest() != expected:
            raise RuntimeError("Reference source mismatch: " + name)
    metadata = reference / ".hermes_build_sha"
    if metadata.exists() and metadata.read_text().strip() != UPSTREAM:
        raise RuntimeError("Conflicting reference build metadata")
    metadata.write_text(UPSTREAM + "\n")
    bundle["python_build_metadata"] = ".hermes_build_sha added to isolated archive only"
    bundle["python_capture_scope"] = "upstream build_welcome_banner component; not full Python CLI"
    bundle["python_source_sha256"] = hashlib.sha256((reference / "hermes_cli/banner.py").read_bytes()).hexdigest()
    for case in bundle["cases"]:
        with tempfile.TemporaryDirectory(prefix="hermes-visual-python-") as temp:
            home = Path(temp)
            (home / "config.yaml").write_text("model:\n  provider: auto\n")
            fixture = home / "fixture.json"
            fixture.write_text(json.dumps(case["fixture"]))
            case["python"] = capture(
                [sys.executable, str(Path(__file__).resolve()), "python-child", str(reference), str(fixture)],
                home, home, case["rust"]["width"],
                {"PYTHONPATH": os.environ.get("PYTHONPATH", "")})
    return bundle


def export_annotations(path, group, label):
    raw = path.read_bytes()
    encoded = base64.b64encode(gzip.compress(raw, mtime=0)).decode()
    chunks = [encoded[i:i + 3000] for i in range(0, len(encoded), 3000)]
    if len(chunks) > 16:
        raise RuntimeError("Bundle exceeds annotation budget; use artifact, do not truncate")
    if group == 0:
        print(f"::notice title={label} digest::sha256={hashlib.sha256(raw).hexdigest()}; bytes={len(raw)}")
    for i in range(group * 8, min((group + 1) * 8, len(chunks))):
        print(f"::notice title={label} {i + 1}/{len(chunks)}::{chunks[i]}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="mode", required=True)
    rust = sub.add_parser("rust")
    rust.add_argument("binary", type=Path)
    rust.add_argument("output", type=Path)
    pair = sub.add_parser("pair")
    pair.add_argument("bundle", type=Path)
    pair.add_argument("reference", type=Path)
    pair.add_argument("output", type=Path)
    child = sub.add_parser("python-child")
    child.add_argument("reference", type=Path)
    child.add_argument("fixture", type=Path)
    export = sub.add_parser("export")
    export.add_argument("bundle", type=Path)
    export.add_argument("--group", type=int, choices=(0, 1), default=0)
    export.add_argument("--label", choices=("visual bundle", "candidate patch"), default="visual bundle")
    args = parser.parse_args()
    if args.mode == "python-child":
        python_child(args.reference.resolve(), args.fixture.resolve())
    elif args.mode == "export":
        export_annotations(args.bundle, args.group, args.label)
    else:
        if args.output.exists():
            parser.error("Refusing to overwrite existing evidence")
        result = rust_capture(args.binary.resolve()) if args.mode == "rust" else pair_with_python(
            json.loads(args.bundle.read_text()), args.reference.resolve())
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
