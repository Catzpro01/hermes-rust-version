use assert_cmd::Command;
use rusqlite::Connection;
use std::path::Path;
use tempfile::TempDir;

const ID: &str = "550e8400-e29b-41d4-a716-446655440000";

fn hermes_cmd() -> Command {
    Command::cargo_bin("hermes-rs").unwrap()
}

fn inject_malicious_session(home: &Path) {
    let db = home.join("state.db");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch("CREATE TABLE sessions (id TEXT PRIMARY KEY, source TEXT NOT NULL, started_at REAL NOT NULL); CREATE TABLE messages (id INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL, role TEXT NOT NULL, content TEXT, timestamp REAL NOT NULL); CREATE TABLE tool_calls (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, turn_index INTEGER NOT NULL, tool_name TEXT NOT NULL, arguments TEXT NOT NULL, result TEXT, status TEXT NOT NULL, created_at REAL NOT NULL);").unwrap();
    conn.execute(
        "INSERT INTO sessions VALUES (?1, 'fixture', 1700000000.0)",
        [ID],
    )
    .unwrap();
    let messages = [
        ("user", "hello"),
        ("assistant", "before\x1b[31mRED\x1b[0mafter"),
        ("assistant", "normal\x1b]0;PWNED TITLE\x07text"),
        ("assistant", "start\x1bP1$r1q\x1b\\end"),
        ("assistant", "before\x1b[truncated"),
        ("assistant", r"code: \x1b[31m should stay"),
    ];
    for (i, (role, content)) in messages.iter().enumerate() {
        conn.execute(
            "INSERT INTO messages (session_id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![ID, role, content, 1700000000.0 + i as f64],
        )
        .unwrap();
    }
    conn.execute("INSERT INTO tool_calls VALUES ('call-ansi-1', ?1, 1, 'shell_readonly', ?2, ?3, 'success', 1700000001.0)", rusqlite::params![ID, "cmd: echo\x1b[32mGREEN\x1b[0m", "output:\x1b]8;;http://evil.com\x07click\x1b]8;;\x07"]).unwrap();
}

fn run(home: &Path, input: String) -> String {
    let output = hermes_cmd()
        .env("HERMES_HOME", home)
        .args(["--provider", "fake", "--resume"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn e2e_inspection_renderers_strip_ansi_without_mutating_state() {
    let home = TempDir::new().unwrap();
    inject_malicious_session(home.path());
    let before = canonical_snapshot(&home.path().join("state.db"));
    let stdout = run(
        home.path(),
        format!("/sessions\n/inspect {ID}\n/messages {ID}\n/tool-calls {ID}\n/exit\n"),
    );
    assert_eq!(
        before,
        canonical_snapshot(&home.path().join("state.db")),
        "inspection modified canonical state.db"
    );
    assert!(
        !stdout.contains('\x1b'),
        "stdout contains actual ESC byte: {stdout:?}"
    );
    assert!(
        stdout.contains(r"\x1b[31m"),
        "literal escaped source was removed"
    );
    assert!(stdout.contains("before") && stdout.contains("RED") && stdout.contains("after"));
    assert!(stdout.contains("shell_readonly") && stdout.contains("success"));
}

#[test]
fn e2e_messages_strip_actual_esc_and_preserve_literal() {
    let home = TempDir::new().unwrap();
    inject_malicious_session(home.path());
    let stdout = run(home.path(), format!("/messages {ID}\n/exit\n"));
    assert!(!stdout.contains('\x1b'));
    assert!(stdout.contains(r"\x1b[31m"));
}

#[test]
fn e2e_tool_calls_strip_actual_esc() {
    let home = TempDir::new().unwrap();
    inject_malicious_session(home.path());
    let stdout = run(home.path(), format!("/tool-calls {ID}\n/exit\n"));
    assert!(!stdout.contains('\x1b'));
    assert!(stdout.contains("shell_readonly"));
}

#[test]
fn e2e_sessions_strip_preview_esc() {
    let home = TempDir::new().unwrap();
    inject_malicious_session(home.path());
    let stdout = run(home.path(), "/sessions\n/exit\n".to_string());
    assert!(!stdout.contains('\x1b'));
}

fn canonical_snapshot(path: &Path) -> Vec<String> {
    let conn = Connection::open(path).unwrap();
    let mut snapshot = Vec::new();
    for sql in [
        "SELECT id, source, started_at FROM sessions ORDER BY id",
        "SELECT id, session_id, role, content, timestamp FROM messages ORDER BY id",
        "SELECT id, session_id, turn_index, tool_name, arguments, result, status, created_at FROM tool_calls ORDER BY id",
    ] {
        let mut stmt = conn.prepare(sql).unwrap();
        let rows = stmt.query_map([], |row| {
            let mut values = Vec::new();
            for index in 0..row.as_ref().column_count() {
                values.push(row.get::<_, String>(index).unwrap_or_else(|_| format!("<non-string:{index}>")));
            }
            Ok(values.join("\u{1f}"))
        }).unwrap();
        snapshot.extend(rows.map(|row| row.unwrap()));
    }
    snapshot
}
