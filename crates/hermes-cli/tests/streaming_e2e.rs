//! Spec 013 Ticket 04 — Streaming, Reasoning & Spinner: piped E2E proof.
//!
//! These drive the real binary with piped (non-TTY) stdin and the offline
//! `fake` provider (which streams its answer as a `Chunk` event), locking the
//! Ticket 04 render-boundary behavior end-to-end: the §5.1 box wraps the
//! streamed text exactly once (no duplication of the final answer), tool
//! activity is a plain `  [tool]` line (spec §6 non-TTY), and piped stdout
//! stays ANSI-free.

use assert_cmd::Command;
use tempfile::tempdir;

/// Runs the binary once against a fresh temp home with the offline `fake`
/// provider, feeding `stdin`.
fn run(stdin: &str) -> (std::process::Output, tempfile::TempDir) {
    let home = tempdir().unwrap();
    std::fs::write(
        home.path().join("config.yaml"),
        "model:\n  provider: auto\n",
    )
    .unwrap();
    let out = Command::cargo_bin("hermes-rs")
        .unwrap()
        .args([
            "--provider",
            "fake",
            "--hermes-home",
            home.path().to_str().unwrap(),
        ])
        .write_stdin(stdin)
        .output()
        .unwrap();
    (out, home)
}

#[test]
fn streamed_answer_is_boxed_exactly_once() {
    let (out, _home) = run("hello\n/exit\n");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // the §5.1 streaming box with the verbatim ` ⚕ Hermes ` label
    assert!(
        stdout.contains("╭─ ⚕ Hermes "),
        "streaming frame header missing, stdout={stdout:?}"
    );
    assert!(stdout.contains('╯'), "streaming frame footer missing");
    // the streamed chunk and the final answer must NOT be printed twice
    let count = stdout.matches("echo: hello").count();
    assert_eq!(count, 1, "answer duplicated, stdout={stdout:?}");
    // the iteration line still follows the box
    assert!(stdout.contains("[iter 1/10]"));
    // piped stdout stays ANSI-free
    assert!(!stdout.contains('\x1b'));
}

#[test]
fn tool_activity_is_plain_when_piped() {
    let (out, _home) = run("tool\n/exit\n");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // spec §6 non-TTY: `  [tool] {message}` — and the raw `<tool_call ...>`
    // markup the fake provider streams must never reach stdout
    assert!(
        stdout.contains("[tool]"),
        "tool activity line missing, stdout={stdout:?}"
    );
    assert!(
        !stdout.contains("<tool_call"),
        "raw tool_call markup leaked, stdout={stdout:?}"
    );
    // the follow-up answer still lands in a box
    assert!(stdout.contains("tool completed"));
    assert!(stdout.contains("╭─ ⚕ Hermes "));
    assert!(!stdout.contains('\x1b'));
}

#[test]
fn piped_reasoning_block_gets_its_own_box() {
    // The fake provider echoes its input verbatim, so feeding a reasoning
    // block exercises the splitter end-to-end: the tags are peeled into the
    // (piped, plain) reasoning box and only the inner text is displayed.
    let (out, _home) = run("<think>plan it</think>answer\n/exit\n");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("plan it"), "stdout={stdout:?}");
    assert!(stdout.contains("answer"), "stdout={stdout:?}");
    // The fake provider prefixes "echo: " to the input, so the first normal
    // text opens the response box, the reasoning box comes next, and the
    // answer re-opens a fresh response box (Python parity: the next normal
    // token after reasoning re-opens the response box).
    let first_box = stdout.find("╭─ ⚕ Hermes ").unwrap();
    let reasoning_at = stdout.find("┌─ Reasoning ").unwrap();
    let plan_at = stdout.find("plan it").unwrap();
    let answer_box = reasoning_at + stdout[reasoning_at..].find("╭─ ⚕ Hermes ").unwrap();
    let answer_at = stdout.find("answer").unwrap();
    assert!(first_box < reasoning_at, "stdout={stdout:?}");
    assert!(
        reasoning_at < plan_at,
        "reasoning text inside the reasoning box"
    );
    assert!(
        reasoning_at < answer_box,
        "answer box comes after the reasoning box"
    );
    assert!(answer_box < answer_at);
    assert!(
        !stdout.contains("<think>"),
        "raw tag leaked, stdout={stdout:?}"
    );
    assert!(!stdout.contains('\x1b'));
}
