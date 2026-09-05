//! Spec 014 (T01): subcommand parser foundation — end-to-end.
//!
//! Guarantees: a subcommand runs to completion and exits 0 without entering
//! the REPL (no prompt, no state touch); a bare invocation still enters the
//! REPL exactly as before (zero regression); global flags parse on both
//! sides of the subcommand.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn hermes_cmd() -> Command {
    Command::cargo_bin("hermes-rs").unwrap()
}

#[test]
fn model_subcommand_prints_placeholder_and_exits_without_repl() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["model"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("coming soon: model (Spec 014)"))
        .stdout(predicate::str::contains("❯ ").not()); // no REPL prompt
                                                       // The placeholder must not create or touch the canonical store.
    assert!(!home.path().join("state.db").exists());
}

#[test]
fn bare_invocation_still_enters_repl_zero_regression() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake"])
        .write_stdin("hello\n/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("echo: hello"))
        .stdout(predicate::str::contains("❯ "));
}

#[test]
fn global_flags_parse_after_subcommand() {
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["model", "--provider", "fake"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("coming soon: model (Spec 014)"));
}

#[test]
fn mcp_subcommand_placeholder_with_action() {
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["mcp", "restart", "srv-1"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("coming soon: mcp (Spec 014)"));
}

#[test]
fn tool_calls_subcommand_parses_kebab_case() {
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["tool-calls", "abc"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "coming soon: tool-calls (Spec 014)",
        ));
}

#[test]
fn search_subcommand_placeholder() {
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["search", "deploy"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("coming soon: search (Spec 014)"));
}

#[test]
fn inspect_requires_an_id() {
    // clap rejects the missing positional with exit code 2 (usage error).
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["inspect"])
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("id"));
}
