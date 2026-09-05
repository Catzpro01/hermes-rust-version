//! Spec 014: shell subcommands — end-to-end.
//!
//! Guarantees: a subcommand runs to completion and exits 0 without entering
//! the REPL (no prompt, no state touch); a bare invocation still enters the
//! REPL exactly as before (zero regression); global flags parse on both
//! sides of the subcommand. `model` (T02) lists configured providers and
//! models with the active marker; the remaining subcommands stay T01
//! placeholders until T03-T07 land.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn hermes_cmd() -> Command {
    Command::cargo_bin("hermes-rs").unwrap()
}

const MODEL_CONFIG: &str = "model:\n  provider: anthropic\nproviders:\n  anthropic:\n    models:\n      claude-sonnet-4-5: {}\n  openai:\n    name: OpenAI\n    models:\n      gpt-4o: {}\n";

fn home_with_model_config() -> TempDir {
    let home = TempDir::new().unwrap();
    std::fs::write(home.path().join("config.yaml"), MODEL_CONFIG).unwrap();
    home
}

#[test]
fn model_lists_configured_providers_with_active_marker() {
    let home = home_with_model_config();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["model"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("Providers:"))
        .stdout(predicate::str::contains("* anthropic (active)"))
        .stdout(predicate::str::contains(
            "models: claude-sonnet-4-5",
        ))
        .stdout(predicate::str::contains("openai (OpenAI)"))
        .stdout(predicate::str::contains("models: gpt-4o"))
        .stdout(predicate::str::contains("❯ ").not()) // no REPL prompt
        .stdout(predicate::str::contains("\u{1b}").not()); // piped -> ANSI-free
    // model is read-only: it must not create or touch the canonical store.
    assert!(!home.path().join("state.db").exists());
}

#[test]
fn model_provider_flag_filters_to_one_provider() {
    let home = home_with_model_config();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["model", "--provider", "openai"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("* openai (OpenAI) (active)"))
        .stdout(predicate::str::contains("models: gpt-4o"))
        .stdout(predicate::str::contains("anthropic").not())
        .stdout(predicate::str::contains("\u{1b}").not());
}

#[test]
fn model_without_config_shows_builtin_fake() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["model"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("Providers:"))
        .stdout(predicate::str::contains("* fake (active, built-in)"))
        .stdout(predicate::str::contains("❯ ").not());
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
        .stdout(predicate::str::contains("fake (active, built-in)"));
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
