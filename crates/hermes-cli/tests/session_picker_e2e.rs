//! Spec 017 T09 — session browse picker: PTY E2E proof (real TTY).
//!
//! Drives `hermes sessions browse` and the REPL's `/sessions` over a real
//! pty (reader threads pump master + stderr into one buffer — the pattern
//! proven by `wizard_e2e.rs`): the verbatim §F frame (hint, column header,
//! rows, footer) renders, ↑↓/Enter/filter/`d`+`y`/Esc behave, and the shell
//! prints the selection while the REPL resumes it in place.

#![cfg(unix)]

use std::fs::File;
use std::io::{Read, Write as _};
use std::path::Path;
use std::process::{Child, Command as StdCommand, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use nix::pty::{openpty, OpenptyResult, Winsize};
use tempfile::TempDir;

const TIMEOUT: Duration = Duration::from_secs(20);
const SEED_A: &str = "550e8400-e29b-41d4-a716-446655440000";
const SEED_B: &str = "660f8400-e29b-41d4-a716-446655440001";
const HINT: &str = "  Browse sessions — ↑↓ navigate  Enter select  Type to filter  Esc quit";
const GOODBYE: &str = "Goodbye! ⚕";

/// Seed a canonical `state.db`: A (older, "deploy the thing") + B (newer,
/// "second topic"). The picker lists newest first, so row 1 is B.
fn seed_state_db(home: &Path) {
    let c = rusqlite::Connection::open(home.join("state.db")).unwrap();
    c.execute_batch(
        "CREATE TABLE sessions(id TEXT PRIMARY KEY,source TEXT NOT NULL,started_at REAL NOT NULL); CREATE TABLE messages(id INTEGER PRIMARY KEY AUTOINCREMENT,session_id TEXT NOT NULL,role TEXT NOT NULL,content TEXT,timestamp REAL NOT NULL); CREATE TABLE tool_calls(id TEXT PRIMARY KEY,session_id TEXT NOT NULL,turn_index INTEGER NOT NULL,tool_name TEXT NOT NULL,arguments TEXT NOT NULL,result TEXT,status TEXT NOT NULL CHECK(status IN ('success','error','denied','timeout','cancelled')),created_at REAL NOT NULL);",
    )
    .unwrap();
    c.execute("INSERT INTO sessions VALUES (?1,'cli',1700000000.0)", [SEED_A])
        .unwrap();
    c.execute("INSERT INTO sessions VALUES (?1,'cli',1700000001.0)", [SEED_B])
        .unwrap();
    c.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES (?1,'user','deploy the thing',1700000000.5)",
        [SEED_A],
    )
    .unwrap();
    c.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES (?1,'user','second topic',1700000001.5)",
        [SEED_B],
    )
    .unwrap();
}

struct PtyPicker {
    child: Child,
    master: Arc<Mutex<File>>,
    output: Arc<Mutex<Vec<u8>>>,
    home: TempDir,
}

fn spawn_reader(mut src: impl Read + Send + 'static, sink: Arc<Mutex<Vec<u8>>>) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match src.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => sink.lock().unwrap().extend_from_slice(&buf[..n]),
            }
        }
    });
}

impl PtyPicker {
    fn spawn(args: &[&str], seed: bool) -> Self {
        let ws = Winsize {
            ws_row: 30,
            ws_col: 100,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let OpenptyResult { master, slave } = openpty(&ws, None).expect("openpty");
        let home = TempDir::new().unwrap();
        std::fs::write(home.path().join("config.yaml"), "model:\n  provider: auto\n").unwrap();
        if seed {
            seed_state_db(home.path());
        }
        let slave_file = File::from(slave);
        let stdout_file = slave_file.try_clone().expect("dup slave for stdout");
        let mut child = StdCommand::new(env!("CARGO_BIN_EXE_hermes-rs"))
            .env("TERM", "xterm-256color")
            .env("HERMES_HOME", home.path())
            .args(args)
            .stdin(slave_file)
            .stdout(stdout_file)
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn hermes-rs");
        let stderr = child.stderr.take().expect("stderr piped");
        let master_file = File::from(master);
        let write_master = Arc::new(Mutex::new(master_file.try_clone().expect("dup master")));
        let output = Arc::new(Mutex::new(Vec::new()));
        spawn_reader(master_file, output.clone());
        spawn_reader(stderr, output.clone());
        PtyPicker {
            child,
            master: write_master,
            output,
            home,
        }
    }

    fn wait_for(&mut self, needle: &str) -> Result<String, String> {
        let start = Instant::now();
        while start.elapsed() < TIMEOUT {
            let buf = self.snapshot();
            if buf.contains(needle) {
                return Ok(buf);
            }
            if self.child.try_wait().expect("try_wait").is_some() {
                std::thread::sleep(Duration::from_millis(100));
                let buf = self.snapshot();
                if buf.contains(needle) {
                    return Ok(buf);
                }
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(format!("timeout waiting for {needle:?}; output so far: {}", self.snapshot()))
    }

    fn snapshot(&self) -> String {
        String::from_utf8_lossy(&self.output.lock().unwrap()).into_owned()
    }

    fn send(&mut self, bytes: &[u8]) {
        self.master.lock().unwrap().write_all(bytes).expect("write to pty master");
    }
}

#[test]
fn browse_renders_verbatim_frame_and_selects_first_row_with_enter() {
    let mut p = PtyPicker::spawn(&["sessions", "browse"], true);
    let out = p.wait_for(HINT).expect("verbatim hint line");
    assert!(out.contains("Title / Preview"), "column header: {out}");
    assert!(out.contains("second topic"), "newest session first: {out}");
    assert!(out.contains("deploy the thing"), "older session listed: {out}");
    assert!(out.contains("1/2 sessions   d delete"), "footer: {out}");
    // Stat/Msgs columns: one done session with one message each row.
    assert!(out.contains("done"), "status column: {out}");

    p.send(b"\r");
    let out = p.wait_for(&format!("Selected session {SEED_B}")).expect("select newest");
    assert!(
        out.contains(&format!("hermes-rs --resume-id {SEED_B}")),
        "resume hint: {out}"
    );
    let status = p.child.wait().expect("child exits");
    assert!(status.success(), "exit: {status}");
}

#[test]
fn browse_arrow_down_selects_the_second_row() {
    let mut p = PtyPicker::spawn(&["sessions", "browse"], true);
    p.wait_for(HINT).expect("hint");
    p.send(b"\x1b[B");
    std::thread::sleep(Duration::from_millis(500));
    p.send(b"\r");
    p.wait_for(&format!("Selected session {SEED_A}")).expect("select older");
    let status = p.child.wait().expect("child exits");
    assert!(status.success(), "exit: {status}");
}

#[test]
fn browse_live_filter_narrows_rows_and_footer_counts() {
    let mut p = PtyPicker::spawn(&["sessions", "browse"], true);
    p.wait_for(HINT).expect("hint");
    p.send(b"second");
    p.wait_for("  Browse sessions — filter: second█").expect("filter hint with block cursor");
    p.wait_for("1/1 sessions (filtered from 2)").expect("filtered footer counts");
    p.send(b"\r");
    p.wait_for(&format!("Selected session {SEED_B}")).expect("select filtered row");
    let status = p.child.wait().expect("child exits");
    assert!(status.success(), "exit: {status}");
}

#[test]
fn browse_delete_removes_the_session_from_state_db() {
    let mut p = PtyPicker::spawn(&["sessions", "browse"], true);
    p.wait_for(HINT).expect("hint");
    // Cursor starts on B ("second topic"): `d` asks, `y` confirms.
    p.send(b"d");
    p.wait_for("  Delete session 'second topic'? [y/N]").expect("explicit [y/N] prompt");
    p.send(b"y");
    std::thread::sleep(Duration::from_millis(500));
    p.send(b"\x1b");
    let status = p.child.wait().expect("child exits");
    assert!(status.success(), "exit: {status}");
    let out = p.snapshot();
    assert!(out.contains("Deleted."), "delete notice: {out}");
    assert!(!out.contains("Selected session"), "cancelled, no selection: {out}");
    // Ground truth: B's rows are gone, A's are untouched.
    let c = rusqlite::Connection::open(p.home.path().join("state.db")).unwrap();
    let sessions: Vec<String> = c
        .prepare("SELECT id FROM sessions ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(sessions, vec![SEED_A.to_owned()]);
    let msgs: i64 = c
        .query_row("SELECT COUNT(*) FROM messages WHERE session_id=?1", [SEED_B], |r| r.get(0))
        .unwrap();
    assert_eq!(msgs, 0);
}

#[test]
fn browse_escape_cancels_without_selecting() {
    let mut p = PtyPicker::spawn(&["sessions", "browse"], true);
    p.wait_for(HINT).expect("hint");
    p.send(b"\x1b");
    let status = p.child.wait().expect("child exits");
    assert!(status.success(), "exit: {status}");
    let out = p.snapshot();
    assert!(!out.contains("Selected session"), "no selection: {out}");
    assert!(!out.contains("Deleted."), "no delete: {out}");
}

#[test]
fn browse_without_a_store_reports_no_sessions() {
    let mut p = PtyPicker::spawn(&["sessions", "browse"], false);
    p.wait_for("No sessions found.").expect("empty-store message");
    let status = p.child.wait().expect("child exits");
    assert!(status.success(), "exit: {status}");
    assert!(!p.home.path().join("state.db").exists(), "browse must not create the store");
}

#[test]
fn repl_sessions_opens_the_picker_and_resumes_in_place() {
    let mut p = PtyPicker::spawn(&["--provider", "fake"], true);
    // Interactive startup starts a FRESH session (spec §F: no picker at
    // launch), so the picker shows [new-empty, B, A]: one Down lands on B.
    p.wait_for("❯ ").expect("repl prompt");
    p.send(b"/sessions\r");
    p.wait_for(HINT).expect("picker opens from /sessions");
    p.send(b"\x1b[B");
    std::thread::sleep(Duration::from_millis(500));
    p.send(b"\r");
    p.wait_for(&format!("Resumed {SEED_B}")).expect("resume the picked session");
    p.send(b"/exit\r");
    p.wait_for(GOODBYE).expect("clean exit");
    let status = p.child.wait().expect("child exits");
    assert!(status.success(), "exit: {status}");
}
