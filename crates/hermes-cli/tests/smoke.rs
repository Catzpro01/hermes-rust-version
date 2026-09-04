use assert_cmd::Command;
use predicates::prelude::*;
use std::{fs, path::Path};
use tempfile::TempDir;

fn hermes_cmd() -> Command {
    Command::cargo_bin("hermes-rs").unwrap()
}
fn fixture() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hermes_state.db")
        .leak()
}

#[test]
fn smoke_fake_provider_full_cycle() {
    let home = TempDir::new().unwrap();
    let db = home.path().join("state.db");
    fs::copy(fixture(), &db).unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake"])
        .write_stdin("hello\n/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("echo: hello"))
        .stdout(predicate::str::contains("hermes>"));
    assert!(fs::metadata(db).unwrap().len() > 0);
}

#[test]
fn smoke_session_persistence_across_runs() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake"])
        .write_stdin("first message\n/exit\n")
        .assert()
        .success();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake", "--resume"])
        .write_stdin("/sessions\n/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("first message"));
}

#[test]
fn smoke_ctrl_d_exits_zero() {
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["--provider", "fake"])
        .write_stdin("")
        .assert()
        .success();
}

#[test]
fn smoke_invalid_config_error() {
    let home = TempDir::new().unwrap();
    fs::write(home.path().join("config.yaml"), "invalid: [yaml: broken").unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid config"));
}

#[test]
fn smoke_missing_home_error() {
    hermes_cmd()
        .env("HERMES_HOME", "/nonexistent/path")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn smoke_python_hermes_untouched() {
    let python_home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|p| p.join(".hermes"));
    let Some(python_home) = python_home else {
        return;
    };
    if !python_home.exists() {
        return;
    }
    let state = python_home.join("state.db");
    let before = fs::metadata(&state).and_then(|m| m.modified()).ok();
    let isolated = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", isolated.path())
        .args(["--provider", "fake"])
        .write_stdin("test\n/exit\n")
        .assert()
        .success();
    let after = fs::metadata(&state).and_then(|m| m.modified()).ok();
    assert_eq!(before, after, "Python Hermes state.db was modified");
}
