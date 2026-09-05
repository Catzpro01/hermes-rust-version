//! Spec 017 T02 — Banner v0.21.0: PTY E2E proof (real TTY, real winsize).
//!
//! The unit tests pin the composed buffer byte-for-byte against the Python
//! v0.21.0 reference; these integration tests prove the same thing through
//! the full startup path (real pty, real `terminal_width()`, real ANSI
//! stream): border `#CD7F32`, title `#FFD700` bold, and the 6-line logo
//! appearing only at terminal width >= 95.

#![cfg(unix)]

use std::fs::File;
use std::io::{Read, Write as _};
use std::process::{Command as StdCommand, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use nix::pty::{openpty, OpenptyResult, Winsize};
use tempfile::TempDir;

const TIMEOUT: Duration = Duration::from_secs(15);

const TITLE: &str = "Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301";
/// SGR 1 (bold) + truecolor `#FFD700` — the version title.
const SGR_TITLE_GOLD_BOLD: &str = "\x1b[1;38;2;255;215;0m";
/// Truecolor `#CD7F32` — the single-line panel border.
const SGR_BORDER_BRONZE: &str = "\x1b[38;2;205;127;50m";
/// First logo row marker (6-line `HERMES_AGENT_LOGO`).
const LOGO_MARKER: &str = "██╗  ██╗";
/// Goodbye line on clean exit.
const GOODBYE: &str = "Goodbye! ⚕";

/// Drives the real REPL over a pty with a configurable column count.
struct PtyRepl {
    child: std::process::Child,
    master: Arc<Mutex<File>>,
    output: Arc<Mutex<Vec<u8>>>,
    _home: TempDir,
}

fn spawn_reader(
    mut stream: impl Read + Send + 'static,
    output: Arc<Mutex<Vec<u8>>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => output.lock().unwrap().extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    })
}

impl PtyRepl {
    fn spawn(cols: u16) -> Self {
        let ws = Winsize {
            ws_row: 30,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let OpenptyResult { master, slave } = openpty(&ws, None).expect("openpty");
        let home = TempDir::new().unwrap();
        // Offline fake provider config (same shape as the piped E2E tests).
        std::fs::write(
            home.path().join("config.yaml"),
            "model:\n  provider: auto\n",
        )
        .unwrap();
        let slave_file = File::from(slave);
        let stdout_file = slave_file.try_clone().expect("dup slave for stdout");
        let mut child = StdCommand::new(env!("CARGO_BIN_EXE_hermes-rs"))
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .env("HERMES_HOME", home.path())
            .args(["--provider", "fake"])
            .stdin(slave_file)
            .stdout(stdout_file)
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn hermes-rs");
        let stderr = child.stderr.take().expect("stderr piped");
        let master_file = File::from(master);
        let write_master = Arc::new(Mutex::new(
            master_file.try_clone().expect("dup master for writes"),
        ));
        let output = Arc::new(Mutex::new(Vec::new()));
        spawn_reader(master_file, output.clone());
        spawn_reader(stderr, output.clone());
        PtyRepl {
            child,
            master: write_master,
            output,
            _home: home,
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
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(format!(
            "timeout waiting for {needle:?}; output: {}",
            self.snapshot()
        ))
    }

    fn snapshot(&self) -> String {
        String::from_utf8_lossy(&self.output.lock().unwrap()).into_owned()
    }

    fn send(&mut self, bytes: &[u8]) {
        self.master
            .lock()
            .unwrap()
            .write_all(bytes)
            .expect("write to pty master");
    }
}

#[test]
fn banner_wide_tty_shows_logo_gold_title_bronze_border() {
    let mut r = PtyRepl::spawn(100);

    let out = r.wait_for(TITLE).expect("title must render on a 100-col TTY");
    assert!(
        out.contains(LOGO_MARKER),
        "6-line logo must appear at width 100 (>= 95)"
    );
    assert!(
        out.contains(SGR_TITLE_GOLD_BOLD),
        "title must be bold #FFD700 (truecolor SGR)"
    );
    assert!(
        out.contains(SGR_BORDER_BRONZE),
        "panel border must be #CD7F32 (truecolor SGR)"
    );
    // The banner is the v0.21.0 info grid, not the old welcome copy.
    assert!(out.contains("Available Tools"));
    assert!(out.contains("Available Skills"));
    assert!(out.contains("No skills installed"));

    r.send(b"/exit\r");
    let out = r
        .wait_for(GOODBYE)
        .expect("clean exit after /exit on TTY");
    assert!(!out.contains('\0'), "no NUL bytes on the pty stream");
    let status = r.child.wait().expect("child exits");
    assert!(status.success(), "exit: {status}");
}

#[test]
fn banner_narrow_tty_hides_logo() {
    let mut r = PtyRepl::spawn(80);

    let out = r.wait_for(TITLE).expect("title must render on an 80-col TTY");
    assert!(
        !out.contains(LOGO_MARKER),
        "logo must be hidden below width 95 (80-col TTY)"
    );
    assert!(
        out.contains(SGR_TITLE_GOLD_BOLD),
        "title must be bold #FFD700 (truecolor SGR)"
    );
    assert!(
        out.contains(SGR_BORDER_BRONZE),
        "panel border must be #CD7F32 (truecolor SGR)"
    );

    r.send(b"/exit\r");
    r.wait_for(GOODBYE).expect("clean exit after /exit on TTY");
    let status = r.child.wait().expect("child exits");
    assert!(status.success(), "exit: {status}");
}
