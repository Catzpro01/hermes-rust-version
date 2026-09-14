//! Spec 017 T01/T05 — `hermes setup` wizard E2E.
//!
//! Non-TTY: clear error, exit 1, no writes. PTY: the real wizard is driven
//! key-by-key (prompts render on stderr via inquire/crossterm; notices on
//! the pty stdout). Reader threads pump both into one buffer — see
//! `.scratch/hermes-rs-total-parity/issues/01-wizard-skeleton.md` for why
//! polling the pty master with `nix::poll` was abandoned.

mod consts {
    pub const MODE_QUESTION: &str = "How would you like to set up Hermes?";
    pub const CANCELED_MESSAGE: &str = "Setup cancelled.";
    pub const FIRST_TIME: &str = "No existing configuration found — running first-time setup.";
    pub const PROVIDER_QUESTION: &str = "Select provider:";
    pub const TERMINAL_QUESTION: &str = "Select terminal backend:";
    pub const TERMINAL_NOT_WIRED: &str =
        "   Docker only for now; Modal, SSH, Daytona, and Singularity are not wired yet.";
    pub const SETUP_COMPLETE: &str = "Setup complete! You're ready to go.";
    pub const BACKUP_NOTICE: &str = "Previous config backed up to: ";
    pub const NOUS_SIGNUP: &str = "Sign up: https://portal.nousresearch.com/manage-subscription";
    pub const NOUS_NOT_AVAILABLE: &str =
        "  Nous Portal OAuth is not available in Hermes-RS yet — pick a provider below.";
}

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn setup_rejects_non_tty_with_clear_error_and_writes_nothing() {
    let home = TempDir::new().unwrap();
    Command::cargo_bin("hermes-rs")
        .unwrap()
        .env("HERMES_HOME", home.path())
        .args(["setup"])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("interactive terminal"))
        .stdout(predicate::str::contains(consts::SETUP_COMPLETE).not());
    assert!(!home.path().join("state.db").exists());
    assert!(!home.path().join("config.yaml").exists());
    assert!(!home.path().join(".env").exists());
}

#[test]
fn setup_unknown_section_is_a_clear_error() {
    let home = TempDir::new().unwrap();
    Command::cargo_bin("hermes-rs")
        .unwrap()
        .env("HERMES_HOME", home.path())
        .args(["setup", "bogus"])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("unknown setup section 'bogus'"));
}

#[test]
fn setup_skeleton_flag_is_gone() {
    Command::cargo_bin("hermes-rs")
        .unwrap()
        .args(["--setup-skeleton"])
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

/// Piped `hermes tools` is a plain listing (T07) — no prompt, no writes.
#[test]
fn tools_piped_lists_catalog_without_writing() {
    let home = TempDir::new().unwrap();
    Command::cargo_bin("hermes-rs")
        .unwrap()
        .env("HERMES_HOME", home.path())
        .args(["tools"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Toolsets:\n"))
        .stdout(predicate::str::contains("[x] web  "))
        .stdout(predicate::str::contains("[ ] spotify  "))
        .stdout(predicate::str::contains('\x1b').not());
    assert!(!home.path().join("config.yaml").exists());
}

#[cfg(unix)]
mod pty {
    use super::consts;
    use std::fs::File;
    use std::io::{Read, Write as _};
    use std::process::{Child, Command as StdCommand, Stdio};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use nix::pty::{openpty, OpenptyResult, Winsize};
    use tempfile::TempDir;

    const TIMEOUT: Duration = Duration::from_secs(20);

    struct PtyWizard {
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

    impl PtyWizard {
        fn spawn(args: &[&str], existing_config: Option<&str>) -> Self {
            let ws = Winsize {
                ws_row: 40,
                ws_col: 120,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let OpenptyResult { master, slave } = openpty(&ws, None).expect("openpty");
            let home = TempDir::new().unwrap();
            if let Some(cfg) = existing_config {
                std::fs::write(home.path().join("config.yaml"), cfg).unwrap();
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
                    // Drain a little more, then give up.
                    std::thread::sleep(Duration::from_millis(100));
                    let buf = self.snapshot();
                    if buf.contains(needle) {
                        return Ok(buf);
                    }
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(format!(
                "timeout waiting for {needle:?}; output so far: {}",
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

    /// `hermes setup terminal` on an empty home: first-time notice, the
    /// verbatim "not wired yet" line, Local (default) → ENTER → config
    /// written atomically with `terminal.backend: local`, no backup (no
    /// previous file), exit 0.
    #[test]
    fn setup_terminal_section_writes_config_atomically() {
        let mut w = PtyWizard::spawn(&["setup", "terminal"], None);
        w.wait_for(consts::FIRST_TIME).unwrap();
        w.wait_for(consts::TERMINAL_NOT_WIRED).unwrap();
        w.wait_for(consts::TERMINAL_QUESTION).unwrap();
        w.send(b"\r");
        let out = w.wait_for(consts::SETUP_COMPLETE).unwrap();
        assert!(
            !out.contains(consts::MODE_QUESTION),
            "section mode skips the mode question"
        );
        assert!(
            !out.contains(consts::BACKUP_NOTICE),
            "no backup without a previous file"
        );
        let status = w.child.wait().expect("child exits");
        assert!(status.success(), "exit: {status}");

        let cfg = std::fs::read_to_string(w.home.path().join("config.yaml")).unwrap();
        assert!(
            cfg.contains("terminal:") && cfg.contains("backend: local"),
            "{cfg}"
        );
        // Atomic write: no temp leftovers; wizard never creates the store.
        let names: Vec<String> = std::fs::read_dir(w.home.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().all(|n| !n.contains(".tmp-")), "{names:?}");
        assert!(!w.home.path().join("state.db").exists());
    }

    /// ESC at the mode question → `Setup cancelled.`, exit 0, and an
    /// existing config is left byte-identical (no backup either).
    #[test]
    fn setup_esc_cancels_without_touching_config() {
        const CFG: &str = "model:\n  provider: auto\ncustom: keep\n";
        let mut w = PtyWizard::spawn(&["setup"], Some(CFG));
        w.wait_for(consts::MODE_QUESTION).unwrap();
        w.send(b"\x1b");
        let out = w.wait_for(consts::CANCELED_MESSAGE).unwrap();
        assert!(!out.contains(consts::SETUP_COMPLETE));
        assert!(
            !out.contains(consts::FIRST_TIME),
            "existing config → no first-time notice"
        );
        let status = w.child.wait().expect("child exits");
        assert!(status.success(), "exit: {status}");
        assert_eq!(
            std::fs::read_to_string(w.home.path().join("config.yaml")).unwrap(),
            CFG
        );
        let names: Vec<String> = std::fs::read_dir(w.home.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["config.yaml".to_owned()], "{names:?}");
    }

    /// `hermes setup model` with an existing config: provider picker (39
    /// verbatim entries), ENTER on the first (Nous Portal), keep the
    /// prefilled URL/key/model prompts → backup created, unknown keys
    /// preserved, `model.provider` updated.
    #[test]
    fn setup_model_section_backs_up_and_merges() {
        const CFG: &str = "custom: keep\nmodel:\n  provider: auto\n  name: my-model\n";
        let mut w = PtyWizard::spawn(&["setup", "model"], Some(CFG));
        let out = w.wait_for(consts::PROVIDER_QUESTION).unwrap();
        assert!(
            out.contains("Choose how to connect to your main chat model."),
            "{out}"
        );
        w.send(b"\r"); // provider: first entry (nous)
        w.wait_for("API base URL").unwrap();
        w.send(b"\r");
        w.wait_for("Environment variable holding the API key")
            .unwrap();
        w.send(b"\r");
        w.wait_for("Model name").unwrap();
        w.send(b"\r");
        let out = w.wait_for(consts::SETUP_COMPLETE).unwrap();
        assert!(out.contains(consts::BACKUP_NOTICE), "{out}");
        let status = w.child.wait().expect("child exits");
        assert!(status.success(), "exit: {status}");

        let cfg = std::fs::read_to_string(w.home.path().join("config.yaml")).unwrap();
        assert!(cfg.contains("custom: keep"), "{cfg}");
        assert!(cfg.contains("provider: nous"), "{cfg}");
        assert!(cfg.contains("name: my-model"), "{cfg}");
        assert!(cfg.contains("key_env: NOUS_API_KEY"), "{cfg}");
        let backups: Vec<String> = std::fs::read_dir(w.home.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("config.yaml.bak."))
            .collect();
        assert_eq!(backups.len(), 1, "{backups:?}");
        assert_eq!(
            std::fs::read_to_string(w.home.path().join(&backups[0])).unwrap(),
            CFG
        );
    }

    /// Quick Setup (default mode, ENTER) prints the verbatim Nous Portal
    /// notice followed by the Rust-only "not available" line, then falls
    /// through to the regular provider picker. ESC there → cancelled,
    /// nothing written.
    #[test]
    fn quick_setup_states_nous_oauth_is_unavailable() {
        let mut w = PtyWizard::spawn(&["setup"], None);
        w.wait_for(consts::MODE_QUESTION).unwrap();
        w.send(b"\r"); // Quick Setup (first option)
        let out = w.wait_for(consts::PROVIDER_QUESTION).unwrap();
        let signup = out.find(consts::NOUS_SIGNUP).expect("portal notice");
        let notice = out
            .find(consts::NOUS_NOT_AVAILABLE)
            .expect("rust-only notice");
        assert!(
            signup < notice,
            "notice follows the verbatim portal block: {out}"
        );
        w.send(b"\x1b");
        w.wait_for(consts::CANCELED_MESSAGE).unwrap();
        let status = w.child.wait().expect("child exits");
        assert!(status.success(), "exit: {status}");
        assert!(!w.home.path().join("config.yaml").exists());
    }

    /// T07: `hermes tools` on a TTY is the wizard's Tools checklist. Toggle
    /// the first row (web, pre-checked → off) and confirm → config gets
    /// `tools.enabled_toolsets` without `web`, with the other defaults.
    #[test]
    fn tools_tty_checklist_writes_enabled_toolsets() {
        let mut w = PtyWizard::spawn(&["tools"], None);
        w.wait_for("Select toolsets to enable:").unwrap();
        w.send(b" \r");
        w.wait_for(consts::SETUP_COMPLETE).unwrap();
        let status = w.child.wait().expect("child exits");
        assert!(status.success(), "exit: {status}");
        let cfg = std::fs::read_to_string(w.home.path().join("config.yaml")).unwrap();
        let parsed: hermes_core::config::HermesConfig = serde_yaml::from_str(&cfg).unwrap();
        let on = parsed.tools.expect("tools section").enabled_toolsets;
        assert!(!on.contains(&"web".to_owned()), "{on:?}");
        assert!(
            on.contains(&"file".to_owned()) && on.contains(&"terminal".to_owned()),
            "{on:?}"
        );
        assert!(!on.contains(&"spotify".to_owned()), "{on:?}");
    }

    /// T06: `hermes model` on a TTY (no --provider) is the wizard's Model
    /// section — same code path as `hermes setup model` (spec §C.3).
    #[test]
    fn model_tty_is_the_provider_picker() {
        let mut w = PtyWizard::spawn(&["model"], None);
        let out = w.wait_for(consts::PROVIDER_QUESTION).unwrap();
        assert!(
            out.contains("Choose how to connect to your main chat model."),
            "{out}"
        );
        assert!(
            out.contains("Nous Portal (Everything your agent needs"),
            "{out}"
        );
        w.send(b"\x1b");
        w.wait_for(consts::CANCELED_MESSAGE).unwrap();
        let status = w.child.wait().expect("child exits");
        assert!(status.success(), "exit: {status}");
        assert!(!w.home.path().join("config.yaml").exists());
    }
}
