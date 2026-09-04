use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn fake_cli_runs_prompt_and_exits_gracefully() {
    let home = tempdir().unwrap();
    std::fs::write(
        home.path().join("config.yaml"),
        "model:\n  provider: auto\n",
    )
    .unwrap();
    let mut command = Command::cargo_bin("hermes-rs").unwrap();
    command.args([
        "--provider",
        "fake",
        "--hermes-home",
        home.path().to_str().unwrap(),
    ]);
    command
        .write_stdin("hello\n/exit\n")
        .assert()
        .success()
        .stdout(predicates::str::contains("echo: hello"));
}

#[test]
fn fake_cli_runs_tool_call_then_followup() {
    let home = tempfile::tempdir().unwrap();
    let mut command = assert_cmd::Command::cargo_bin("hermes-rs").unwrap();
    command.args([
        "--provider",
        "fake",
        "--hermes-home",
        home.path().to_str().unwrap(),
    ]);
    command
        .write_stdin("tool\n/exit\n")
        .assert()
        .success()
        .stdout(predicates::str::contains("tool completed"));
}
