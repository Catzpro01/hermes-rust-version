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

#[test]
fn inspection_commands_show_read_only_session_details() {
    let home = tempfile::tempdir().unwrap();
    let first = assert_cmd::Command::cargo_bin("hermes-rs")
        .unwrap()
        .args([
            "--provider",
            "fake",
            "--hermes-home",
            home.path().to_str().unwrap(),
        ])
        .write_stdin("hello inspection\n/exit\n")
        .output()
        .unwrap();
    assert!(first.status.success());
    let stdout = String::from_utf8_lossy(&first.stdout);
    let id = stdout
        .split_whitespace()
        .find(|part| part.len() == 36 && part.chars().filter(|c| *c == '-').count() == 4)
        .unwrap()
        .to_owned();
    let input = format!("/sessions\n/inspect {id}\n/messages {id}\n/tool-calls {id}\n/exit\n");
    let second = assert_cmd::Command::cargo_bin("hermes-rs")
        .unwrap()
        .args([
            "--provider",
            "fake",
            "--hermes-home",
            home.path().to_str().unwrap(),
        ])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(second.status.success());
    let output = String::from_utf8_lossy(&second.stdout);
    assert!(output.contains("started="));
    assert!(output.contains("Turns: 2"));
    assert!(output.contains("user: hello inspection"));
}

#[test]
fn search_cli_is_sanitized_and_never_executes_results() {
    let home = tempfile::tempdir().unwrap();
    let first = assert_cmd::Command::cargo_bin("hermes-rs")
        .unwrap()
        .args([
            "--provider",
            "fake",
            "--hermes-home",
            home.path().to_str().unwrap(),
        ])
        .write_stdin("hello searchable\n/exit\n")
        .output()
        .unwrap();
    assert!(first.status.success());
    let second = assert_cmd::Command::cargo_bin("hermes-rs")
        .unwrap()
        .args([
            "--provider",
            "fake",
            "--hermes-home",
            home.path().to_str().unwrap(),
            "--resume",
        ])
        .write_stdin("/search searchable\n/search rm -rf /\n/exit\n")
        .output()
        .unwrap();
    assert!(second.status.success());
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(stdout.contains("Search results for: searchable"));
    assert!(!stdout.contains('\x1b'));
    assert!(!stdout.contains("tool completed"));
}
