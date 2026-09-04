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
    let conn = rusqlite::Connection::open(home.path().join("state.db")).unwrap();
    conn.execute_batch("CREATE TABLE sessions(id TEXT PRIMARY KEY, source TEXT NOT NULL, started_at REAL NOT NULL); CREATE TABLE messages(id INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL, role TEXT NOT NULL, content TEXT, timestamp REAL NOT NULL); CREATE TABLE tool_calls(id TEXT PRIMARY KEY, session_id TEXT NOT NULL, turn_index INTEGER NOT NULL, tool_name TEXT NOT NULL, arguments TEXT NOT NULL, result TEXT, status TEXT NOT NULL, created_at REAL NOT NULL); INSERT INTO sessions VALUES ('550e8400-e29b-41d4-a716-446655440000', 'fixture', 1700000000.0); INSERT INTO messages(session_id, role, content, timestamp) VALUES ('550e8400-e29b-41d4-a716-446655440000', 'assistant', 'searchable safe result', 1700000001.0);").unwrap();
    drop(conn);
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
    assert!(
        second.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&second.stderr)
    );
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        stdout.contains("Search results for: searchable"),
        "stdout={stdout:?}"
    );
    assert!(!stdout.contains('\x1b'));
    assert!(!stdout.contains("tool completed"));
}
