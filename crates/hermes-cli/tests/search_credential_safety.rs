use assert_cmd::Command;
use rusqlite::Connection;
use std::path::Path;
use tempfile::TempDir;

const ID: &str = "550e8400-e29b-41d4-a716-446655440000";
fn canonical(path: &Path) -> Vec<String> {
    let c = Connection::open(path).unwrap();
    let mut v = Vec::new();
    for sql in [
        "SELECT id,source,started_at FROM sessions ORDER BY id",
        "SELECT id,session_id,role,content,timestamp FROM messages ORDER BY id",
    ] {
        let mut q = c.prepare(sql).unwrap();
        v.extend(
            q.query_map([], |r| {
                Ok(format!(
                    "{:?}|{:?}|{:?}|{:?}|{:?}",
                    r.get::<_, String>(0),
                    r.get::<_, Option<String>>(1),
                    r.get::<_, Option<String>>(2),
                    r.get::<_, Option<String>>(3),
                    r.get::<_, Option<f64>>(4)
                ))
            })
            .unwrap()
            .map(|r| r.unwrap()),
        );
    }
    v
}
#[test]
fn search_does_not_leak_credentials() {
    let home = TempDir::new().unwrap();
    let db = home.path().join("state.db");
    let c = Connection::open(&db).unwrap();
    c.execute_batch("CREATE TABLE sessions(id TEXT PRIMARY KEY,source TEXT NOT NULL,started_at REAL NOT NULL); CREATE TABLE messages(id INTEGER PRIMARY KEY AUTOINCREMENT,session_id TEXT NOT NULL,role TEXT NOT NULL,content TEXT,timestamp REAL NOT NULL); CREATE TABLE tool_calls(id TEXT PRIMARY KEY,session_id TEXT NOT NULL,turn_index INTEGER NOT NULL,tool_name TEXT NOT NULL,arguments TEXT NOT NULL,result TEXT,status TEXT NOT NULL,created_at REAL NOT NULL);").unwrap();
    c.execute(
        "INSERT INTO sessions VALUES (?1,'fixture',1700000000.0)",
        [ID],
    )
    .unwrap();
    c.execute("INSERT INTO messages(session_id,role,content,timestamp) VALUES (?1,'assistant','deploy with API_KEY=super-secret-fixture-xyz',1700000001.0)",[ID]).unwrap();
    let mut c2 = c;
    hermes_core::search::migration::run_migrations(&mut c2).unwrap();
    hermes_core::search::rebuild_index(&c2).unwrap();
    drop(c2);
    let before = canonical(&db);
    let out = Command::cargo_bin("hermes-rs")
        .unwrap()
        .args([
            "--provider",
            "fake",
            "--hermes-home",
            home.path().to_str().unwrap(),
            "--resume",
        ])
        .write_stdin("/search deploy\n/exit\n")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("super-secret-fixture-xyz"),
        "credential leaked: {stdout}"
    );
    assert!(stdout.contains("***REDACTED***"));
    assert!(!stdout.contains('\x1b'));
    assert_eq!(before, canonical(&db));
}
