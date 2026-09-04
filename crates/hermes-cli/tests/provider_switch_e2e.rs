//! End-to-end coverage for the mid-session `/provider` command, driven through
//! the real binary with a config that declares a second (not-yet-built) provider.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

const UNSET_ENV: &str = "HERMES_PROVIDER_SWITCH_UNSET_ZZ9";

fn home_with_config() -> TempDir {
    let home = TempDir::new().unwrap();
    fs::write(
        home.path().join("config.yaml"),
        format!(
            "providers:\n  p:\n    api: http://localhost:9/\n    key_env: {UNSET_ENV}\n    models:\n      m: {{}}\n"
        ),
    )
    .unwrap();
    home
}

fn spawn(home: &TempDir) -> std::process::Output {
    Command::cargo_bin("hermes-rs")
        .unwrap()
        .env_remove(UNSET_ENV)
        .args(["--provider", "fake", "--hermes-home", home.path().to_str().unwrap()])
        .write_stdin("/provider\n/provider p\n/provider nope\nhello\n/exit\n")
        .output()
        .unwrap()
}

#[test]
fn provider_command_lists_marks_and_rolls_back_on_failure() {
    let home = home_with_config();
    let out = spawn(&home);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // Listing marks the active provider and shows the configured one.
    assert!(stdout.contains("fake (active)"), "stdout={stdout:?}");
    assert!(stdout.contains("\n  p\n"), "configured provider listed: {stdout:?}");

    // A provider that cannot be constructed must roll back, keeping `fake`
    // active instead of leaving a half-finished switch.
    assert!(
        stderr.contains("keeping provider fake"),
        "rollback message expected in stderr={stderr:?}"
    );

    // An unknown provider errors (naming the available ones) and does not
    // disturb the running session.
    assert!(
        stderr.contains("'nope' is not configured") && stderr.contains("available: fake, p"),
        "unknown-name error expected: {stderr:?}"
    );

    // The session is still alive and serviced by the (unchanged) fake provider.
    assert!(stdout.contains("echo: hello"), "stdout={stdout:?}");
}

#[test]
fn provider_switch_to_an_available_provider_keeps_session_running() {
    let home = home_with_config();
    let out = Command::cargo_bin("hermes-rs")
        .unwrap()
        .env_remove(UNSET_ENV)
        .args(["--provider", "fake", "--hermes-home", home.path().to_str().unwrap()])
        .write_stdin("/provider fake\nhello after switch\n/exit\n")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Switched provider to fake"),
        "stdout={stdout:?}"
    );
    // Re-selecting the active provider must not reset the conversation: the
    // following turn is still answered.
    assert!(
        stdout.contains("echo: hello after switch"),
        "stdout={stdout:?}"
    );
}
