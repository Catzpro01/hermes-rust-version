//! Spec 017 (T01): wizard skeleton — end-to-end.
//!
//! The interactive path is driven over a real pseudo-terminal: the child's
//! stdin+stdout are a pty (inquire uses raw mode on stdin; its crossterm
//! backend renders prompts to stderr). Two background reader threads pump
//! the pty master and the stderr pipe into one shared output buffer; the
//! main test thread polls that buffer every 50 ms and sends key bytes to
//! the master. The non-TTY path is the deterministic invariant-8 check
//! (clear error, exit 1). The wizard writes nothing, so a fresh
//! `HERMES_HOME` must stay empty after every run.

/// Mirrors `wizard::` constants (a binary crate can't be imported from
/// integration tests). Keep in sync with `src/wizard/mod.rs`.
mod wizard_e2e_consts {
    pub const IMPORT_QUESTION: &str = "Would you like to see what can be imported?";
    pub const MODE_QUESTION: &str = "How would you like to set up Hermes?";
    pub const SECTIONS_QUESTION: &str = "Select sections to configure:";
    pub const COMPLETE_MARKER: &str = "Setup (skeleton) complete:";
    pub const CANCELED_MESSAGE: &str = "Setup cancelled.";
}

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn wizard_rejects_non_tty_with_clear_error() {
    let home = TempDir::new().unwrap();
    Command::cargo_bin("hermes-rs")
        .unwrap()
        .env("HERMES_HOME", home.path())
        .args(["--setup-skeleton"])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("interactive terminal"))
        .stdout(predicate::str::contains(wizard_e2e_consts::COMPLETE_MARKER).not());
    assert!(!home.path().join("state.db").exists());
}

#[cfg(unix)]
mod pty {
    //! PTY-driven wizard flow ("mock stdin").

    use super::wizard_e2e_consts;
    use super::*;
    use std::fs::File;
    use std::io::{Read, Write as _};
    use std::process::{Command as StdCommand, Stdio};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use nix::pty::{openpty, OpenptyResult, Winsize};

    const TIMEOUT: Duration = Duration::from_secs(15);

    /// Drives the wizard over a pty. All rendered output (prompts on the
    /// stderr pipe, the final summary on pty stdout) is accumulated by
    /// reader threads into `output`; the main thread never blocks on an
    /// fd directly, which keeps this test robust on slow CI machines.
    struct PtyWizard {
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

    impl PtyWizard {
        fn spawn() -> Self {
            let ws = Winsize {
                ws_row: 24,
                ws_col: 80,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let OpenptyResult { master, slave } = openpty(&ws, None).expect("openpty");
            let home = TempDir::new().unwrap();
            let slave_file = File::from(slave);
            let stdout_file = slave_file.try_clone().expect("dup slave for stdout");
            let mut child = StdCommand::new(env!("CARGO_BIN_EXE_hermes-rs"))
                .env("TERM", "xterm-256color")
                .env("HERMES_HOME", home.path())
                .args(["--setup-skeleton"])
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
            PtyWizard {
                child,
                master: write_master,
                output,
                _home: home,
            }
        }

        /// Wait until `needle` appears in the combined pty+stderr output,
        /// or time out / the child exits first.
        fn wait_for(&mut self, needle: &str) -> Result<String, String> {
            let start = Instant::now();
            while start.elapsed() < TIMEOUT {
                let buf = self.snapshot();
                if buf.contains(needle) {
                    return Ok(buf);
                }
                if self.child_exited() {
                    break; // child is gone; the needle will never arrive
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            let buf = self.snapshot();
            Err(format!(
                "timeout waiting for {needle:?}; output so far: {buf}"
            ))
        }

        fn snapshot(&self) -> String {
            String::from_utf8_lossy(&self.output.lock().unwrap()).into_owned()
        }

        fn child_exited(&mut self) -> bool {
            self.child.try_wait().expect("try_wait").is_some()
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
    fn wizard_skeleton_happy_path() {
        let mut w = PtyWizard::spawn();

        // Step 1 — import (default No) → ENTER.
        w.wait_for(wizard_e2e_consts::IMPORT_QUESTION).unwrap();
        w.send(b"\r");
        // Step 2 — mode (default first = Quick Setup) → ENTER.
        w.wait_for(wizard_e2e_consts::MODE_QUESTION).unwrap();
        w.send(b"\r");
        // Step 3 — sections → SPACE (toggle first) + ENTER (confirm).
        w.wait_for(wizard_e2e_consts::SECTIONS_QUESTION).unwrap();
        w.send(b" \r");
        // Summary (skeleton-only marker, on the pty stdout).
        let out = w.wait_for(wizard_e2e_consts::COMPLETE_MARKER).unwrap();
        assert!(out.contains("import: no"), "{out}");
        assert!(out.contains("Quick Setup (Nous Portal)"), "{out}");
        assert!(out.contains("Model & Provider"), "{out}");

        let status = w.child.wait().expect("child exits");
        assert!(status.success(), "exit: {status}");
        // The wizard must not touch the canonical store.
        assert!(!w._home.path().join("state.db").exists());
    }

    #[test]
    fn wizard_skeleton_esc_cancels_and_rolls_back() {
        let mut w = PtyWizard::spawn();

        w.wait_for(wizard_e2e_consts::IMPORT_QUESTION).unwrap();
        w.send(b"\r");
        w.wait_for(wizard_e2e_consts::MODE_QUESTION).unwrap();
        // ESC cancels the wizard (Python: "Escape cancels an active setup
        // wizard") — not an error, no completion output.
        w.send(b"\x1b");
        let out = w.wait_for(wizard_e2e_consts::CANCELED_MESSAGE).unwrap();
        assert!(!out.contains(wizard_e2e_consts::COMPLETE_MARKER));

        let status = w.child.wait().expect("child exits");
        assert!(status.success(), "exit: {status}");
        assert!(!w._home.path().join("state.db").exists());
    }
}
