use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;
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

#[test]
fn tui_flag_rejects_piped_non_interactive_stdin() {
    // Spec 012 Ticket 01: --tui on a non-TTY (piped) stdin must error clearly
    // rather than entering crossterm raw mode and hanging/crashing.
    let home = tempdir().unwrap();
    std::fs::write(
        home.path().join("config.yaml"),
        "model:\n  provider: auto\n",
    )
    .unwrap();
    let mut command = Command::cargo_bin("hermes-rs").unwrap();
    command
        .args([
            "--tui",
            "--provider",
            "fake",
            "--hermes-home",
            home.path().to_str().unwrap(),
        ])
        .write_stdin("hello\n")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "--tui requires an interactive terminal",
        ));
}

/// The REPL driven piped with the offline `fake` provider, which echoes back
/// every prompt it receives. That echo is what makes "the model never saw this"
/// assertable: a command handled by the REPL produces no echo.
fn fake_repl(home: &std::path::Path) -> Command {
    let mut command = Command::cargo_bin("hermes-rs").unwrap();
    command.args([
        "--provider",
        "fake",
        "--hermes-home",
        home.to_str().unwrap(),
    ]);
    command
}

/// A catalogued command this build has no handler for must say so instead of
/// being forwarded to the model (`/compress` is Python's, autocomplete offers
/// it, and before this it silently became prose).
#[test]
fn unported_slash_command_is_reported_and_never_reaches_the_model() {
    let home = tempdir().unwrap();
    let notice = contains("/compress is not implemented");
    fake_repl(home.path())
        .write_stdin("/compress\n/exit\n")
        .assert()
        .success()
        .stdout(contains("echo: /compress").not())
        .stderr(notice)
        .stderr(contains("Python:"));
}

/// Slash-prefixed prose and paths are not commands: they must still reach the
/// model exactly as before, or the notice would eat ordinary input.
#[test]
fn slash_prefixed_prose_and_paths_still_reach_the_model() {
    let home = tempdir().unwrap();
    let script = "/definitely-not-a-command\n/home/user/x is where I live\n/exit\n";
    fake_repl(home.path())
        .write_stdin(script)
        .assert()
        .success()
        .stdout(contains("echo: /definitely-not-a-command"))
        .stdout(contains("echo: /home/user/x is where I live"));
}

/// `/help` used to advertise four commands the REPL never dispatched
/// (`/history`, `/model`, `/reset`, `/quit`), so they became prose. Three are
/// real now; `/model` is no longer advertised and reports itself as unported.
#[test]
fn advertised_aliases_dispatch_and_model_does_not() {
    let home = tempdir().unwrap();
    let script = "hello\n/history\n/reset\n/model gpt-x\n/quit\n";
    fake_repl(home.path())
        .write_stdin(script)
        .assert()
        .success()
        .stdout(contains("[1] user: hello"))
        .stdout(contains("New session"))
        .stdout(contains("echo: /history").not())
        .stdout(contains("echo: /reset").not())
        .stderr(contains("/model is not implemented"));
}

/// `/help unported` is the honest inventory of the catalog gap: what Python
/// has, what completion offers, and what this build cannot run yet.
#[test]
fn help_unported_lists_the_catalog_gap() {
    let home = tempdir().unwrap();
    let heading = contains("commands this build does not implement");
    fake_repl(home.path())
        .write_stdin("/help unported\n/exit\n")
        .assert()
        .success()
        .stdout(heading)
        .stdout(contains("/compress"))
        .stdout(contains("[gateway-only]"))
        .stdout(contains("echo: /help unported").not());
}
