//! Spec 013 Ticket 03 — Banner, Prompt, & Strings: piped E2E proof.
//!
//! These drive the real binary with piped (non-TTY) stdin, so they lock the
//! render-boundary invariants: brand strings land on stdout, piped output
//! stays byte-stable and ANSI-free, and the TTY-only banner never leaks.

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
fn help_lists_commands_with_hermes_header_and_separator() {
    let (out, _home) = run("/help\n/exit\n");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // verbatim kawaii header (spec §9) + the ─×40 separator
    assert!(
        stdout.contains("(^_^)? Available Commands"),
        "stdout={stdout:?}"
    );
    assert!(
        stdout.contains(&"─".repeat(40)),
        "40-dash separator missing"
    );
    // a sample of the command list
    assert!(stdout.contains("/provider"));
    assert!(stdout.contains("/reflect"));
    assert!(stdout.contains("/tool-calls"));
    // piped stdout stays ANSI-free
    assert!(!stdout.contains('\x1b'));
}

#[test]
fn clean_exit_prints_goodbye() {
    let (out, _home) = run("/exit\n");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Goodbye! ⚕"), "stdout={stdout:?}");
    assert!(!stdout.contains('\x1b'));
}

#[test]
fn piped_response_is_framed_with_response_label() {
    let (out, _home) = run("hello\n/exit\n");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // the §5.1 box with the verbatim ` ⚕ Hermes ` label
    assert!(
        stdout.contains("╭─ ⚕ Hermes "),
        "response frame header missing, stdout={stdout:?}"
    );
    assert!(stdout.contains('╯'), "response frame footer missing");
    // the model answer still lands inside the frame
    assert!(stdout.contains("echo: hello"));
    assert!(!stdout.contains('\x1b'));
}

#[test]
fn prompt_symbol_is_hermes_brand_and_banner_stays_untty() {
    let (out, _home) = run("hello\n/exit\n");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // the brand prompt replaced the legacy `hermes> `
    assert!(
        stdout.contains("❯ "),
        "brand prompt missing, stdout={stdout:?}"
    );
    assert!(!stdout.contains("hermes> "));
    // the welcome banner is TTY-only: no logo art on piped stdout
    assert!(
        !stdout.contains("██╗"),
        "banner must not print on piped runs"
    );
    assert!(!stdout.contains('\x1b'));
}
