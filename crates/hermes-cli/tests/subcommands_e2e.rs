//! Spec 014: shell subcommands — end-to-end.
//!
//! Guarantees: a subcommand runs to completion and exits 0 without entering
//! the REPL (no prompt, no state touch); a bare invocation still enters the
//! REPL exactly as before (zero regression); global flags parse on both
//! sides of the subcommand. `model` (T02) lists configured providers and
//! models with the active marker; `sessions` and `inspect` (T03) reuse the
//! REPL's own `session_menu` renderers so shell and REPL output is
//! byte-identical; the remaining subcommands stay T01 placeholders until
//! T04-T07 land.

use assert_cmd::Command;
use predicates::prelude::*;
use rusqlite::Connection;
use std::path::Path;
use tempfile::TempDir;

fn hermes_cmd() -> Command {
    Command::cargo_bin("hermes-rs").unwrap()
}

const MODEL_CONFIG: &str = "model:\n  provider: anthropic\nproviders:\n  anthropic:\n    models:\n      claude-sonnet-4-5: {}\n  openai:\n    name: OpenAI\n    models:\n      gpt-4o: {}\n";

fn home_with_model_config() -> TempDir {
    let home = TempDir::new().unwrap();
    std::fs::write(home.path().join("config.yaml"), MODEL_CONFIG).unwrap();
    home
}

#[test]
fn model_lists_configured_providers_with_active_marker() {
    let home = home_with_model_config();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["model"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("Providers:"))
        .stdout(predicate::str::contains("* anthropic (active)"))
        .stdout(predicate::str::contains(
            "models: claude-sonnet-4-5",
        ))
        .stdout(predicate::str::contains("openai (OpenAI)"))
        .stdout(predicate::str::contains("models: gpt-4o"))
        .stdout(predicate::str::contains("❯ ").not()) // no REPL prompt
        .stdout(predicate::str::contains("\u{1b}").not()); // piped -> ANSI-free
    // model is read-only: it must not create or touch the canonical store.
    assert!(!home.path().join("state.db").exists());
}

#[test]
fn model_provider_flag_filters_to_one_provider() {
    let home = home_with_model_config();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["model", "--provider", "openai"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("* openai (OpenAI) (active)"))
        .stdout(predicate::str::contains("models: gpt-4o"))
        .stdout(predicate::str::contains("anthropic").not())
        .stdout(predicate::str::contains("\u{1b}").not());
}

#[test]
fn model_without_config_shows_builtin_fake() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["model"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("Providers:"))
        .stdout(predicate::str::contains("* fake (active, built-in)"))
        .stdout(predicate::str::contains("❯ ").not());
}

#[test]
fn bare_invocation_still_enters_repl_zero_regression() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake"])
        .write_stdin("hello\n/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("echo: hello"))
        .stdout(predicate::str::contains("❯ "));
}

#[test]
fn global_flags_parse_after_subcommand() {
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["model", "--provider", "fake"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("fake (active, built-in)"));
}

#[test]
fn mcp_subcommand_placeholder_with_action() {
    // After T06, mcp is real - with no config, shows no servers message
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["mcp", "restart", "srv-1"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("no MCP servers connected"));
}

#[test]
fn mcp_list_with_no_config() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["mcp", "list"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("no MCP servers connected"));
}

#[test]
fn info_subcommand_shows_home_and_provider() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["info"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hermes Home:"))
        .stdout(predicate::str::contains("Active provider:"));
}


#[test]
fn tool_calls_subcommand_parses_kebab_case() {
    // After T04, tool-calls is real - kebab-case still works but validates UUID
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["tool-calls", "abc"])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("invalid session id 'abc' (expected a UUID)"));
}

#[test]
fn messages_subcommand_parses_and_validates() {
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["messages", "not-a-uuid"])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("invalid session id 'not-a-uuid' (expected a UUID)"));
}

#[test]
fn tool_calls_and_messages_unknown_id_is_clear_error() {
    let missing = "99999999-9999-9999-9999-999999999999";
    let home = TempDir::new().unwrap();
    drop(seed_state_db(home.path()));
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["tool-calls", missing])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(format!("session not found: {missing}")));
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["messages", missing])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(format!("session not found: {missing}")));
}

#[test]
fn search_subcommand_placeholder() {
    // After T05, search is real - with no store, prints "No search results."
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["search", "deploy"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("No search results."));
}

#[test]
fn search_with_store_and_redaction() {
    // With store, search should work and redact credentials
    let home = TempDir::new().unwrap();
    drop(seed_state_db(home.path()));
    // Search for existing content should return results (or no results, but not placeholder)
    let out = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["search", "parity"])
        .write_stdin("")
        .assert()
        .success();
    // Should not contain placeholder
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    assert!(!stdout.contains("coming soon: search"), "search should be implemented, not placeholder: {stdout}");
}


#[test]
fn inspect_requires_an_id() {
    // clap rejects the missing positional with exit code 2 (usage error).
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["inspect"])
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("id"));
}

// ---------------------------------------------------------------------------
// T03: `hermes sessions` + `hermes inspect <id>`
//
// Fixtures seed `state.db` directly with rusqlite (same technique as
// `search_credential_safety.rs`). Parity with the REPL is asserted against
// the *live* REPL output on the same store, not a copy of the format.
// ---------------------------------------------------------------------------

const SEED_ID_A: &str = "550e8400-e29b-41d4-a716-446655440000";
const SEED_ID_B: &str = "660f8400-e29b-41d4-a716-446655440001";

/// Seed a canonical `state.db` with two fixture sessions (one user message
/// each, distinct `started_at`) and one tool call on session B.
fn seed_state_db(home: &Path) -> Connection {
    let db = home.join("state.db");
    let c = Connection::open(&db).unwrap();
    c.execute_batch(
        "CREATE TABLE sessions(id TEXT PRIMARY KEY,source TEXT NOT NULL,started_at REAL NOT NULL); CREATE TABLE messages(id INTEGER PRIMARY KEY AUTOINCREMENT,session_id TEXT NOT NULL,role TEXT NOT NULL,content TEXT,timestamp REAL NOT NULL); CREATE TABLE tool_calls(id TEXT PRIMARY KEY,session_id TEXT NOT NULL,turn_index INTEGER NOT NULL,tool_name TEXT NOT NULL,arguments TEXT NOT NULL,result TEXT,status TEXT NOT NULL CHECK(status IN ('success','error','denied','timeout','cancelled')),created_at REAL NOT NULL);",
    )
    .unwrap();
    c.execute(
        "INSERT INTO sessions VALUES (?1,'fixture',1700000000.0)",
        [SEED_ID_A],
    )
    .unwrap();
    c.execute(
        "INSERT INTO sessions VALUES (?1,'fixture',1700000001.0)",
        [SEED_ID_B],
    )
    .unwrap();
    c.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES (?1,'user','parity check A',1700000000.5)",
        [SEED_ID_A],
    )
    .unwrap();
    c.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES (?1,'user','parity check B',1700000001.5)",
        [SEED_ID_B],
    )
    .unwrap();
    c.execute(
        "INSERT INTO tool_calls(id,session_id,turn_index,tool_name,arguments,result,status,created_at) VALUES ('tc-b-1',?1,0,'fixture_tool','{}','ok','success',1700000001.6)",
        [SEED_ID_B],
    )
    .unwrap();
    c
}

/// Canonical state rows (sessions, messages, tool_calls) — a before/after
/// snapshot proving a subcommand wrote no state.
fn canonical_rows(db: &Path) -> Vec<String> {
    let c = Connection::open(db).unwrap();
    let mut v = Vec::new();
    {
        let mut q = c
            .prepare("SELECT id,source,started_at FROM sessions ORDER BY id")
            .unwrap();
        v.extend(
            q.query_map([], |r| {
                Ok(format!(
                    "{:?}|{:?}|{:?}",
                    r.get::<_, String>(0),
                    r.get::<_, String>(1),
                    r.get::<_, f64>(2)
                ))
            })
            .unwrap()
            .map(|r| r.unwrap()),
        );
    }
    {
        let mut q = c
            .prepare("SELECT id,session_id,role,content,timestamp FROM messages ORDER BY id")
            .unwrap();
        v.extend(
            q.query_map([], |r| {
                Ok(format!(
                    "{:?}|{:?}|{:?}|{:?}|{:?}",
                    r.get::<_, i64>(0),
                    r.get::<_, String>(1),
                    r.get::<_, String>(2),
                    r.get::<_, Option<String>>(3),
                    r.get::<_, f64>(4)
                ))
            })
            .unwrap()
            .map(|r| r.unwrap()),
        );
    }
    {
        let mut q = c
            .prepare(
                "SELECT id,session_id,turn_index,tool_name,arguments,result,status,created_at FROM tool_calls ORDER BY id",
            )
            .unwrap();
        v.extend(
            q.query_map([], |r| {
                Ok(format!(
                    "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
                    r.get::<_, String>(0),
                    r.get::<_, String>(1),
                    r.get::<_, i64>(2),
                    r.get::<_, String>(3),
                    r.get::<_, String>(4),
                    r.get::<_, Option<String>>(5),
                    r.get::<_, String>(6),
                    r.get::<_, f64>(7)
                ))
            })
            .unwrap()
            .map(|r| r.unwrap()),
        );
    }
    v
}

/// The REPL prints the `❯ ` prompt without a trailing newline, so the first
/// output line of a command is glued onto the prompt line. Strip it before
/// comparing, so we compare the command's own output, not the prompt.
fn strip_prompt(line: &str) -> &str {
    line.strip_prefix("❯ ").unwrap_or(line)
}

/// The session-list lines as `session_menu::list_sessions` prints them
/// (one `  started=` per session).
fn session_list_lines(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(strip_prompt)
        .filter(|l| l.contains("  started="))
        .map(str::to_owned)
        .collect()
}

/// The metadata lines as `session_menu::inspect_session` prints them.
fn inspect_meta_lines(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(strip_prompt)
        .filter(|l| {
            l.starts_with("Session: ")
                || l.starts_with("Source: ")
                || l.starts_with("Started: ")
                || l.starts_with("Turns: ")
                || l.starts_with("Tool calls: ")
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn sessions_lists_all_sessions_with_repl_parity() {
    let home = TempDir::new().unwrap();
    let c = seed_state_db(home.path());
    let before = canonical_rows(&home.path().join("state.db"));

    // Live REPL reference: `/sessions` on the very same store.
    let repl = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake"])
        .write_stdin("/sessions\n/exit\n")
        .output()
        .unwrap();
    assert!(repl.status.success(), "repl must succeed");
    drop(c);

    // The subcommand: byte-exact, and identical to the REPL's list lines.
    let out = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["sessions"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("❯ ").not()) // no REPL prompt
        .stdout(predicate::str::contains("\u{1b}").not()); // piped -> ANSI-free
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    let expected = format!(
        "{SEED_ID_B}  started=1700000001.000  parity check B\n\
         {SEED_ID_A}  started=1700000000.000  parity check A\n"
    );
    assert_eq!(stdout, expected, "byte-exact, started_at DESC order");
    let repl_lines = session_list_lines(&String::from_utf8_lossy(&repl.stdout));
    assert_eq!(
        stdout,
        format!("{}\n", repl_lines.join("\n")),
        "shell and REPL must render identically"
    );
    // Read-only: no state row was created or modified by either run.
    assert_eq!(
        before,
        canonical_rows(&home.path().join("state.db")),
        "subcommand must not write state"
    );
}

#[test]
fn sessions_without_store_says_no_sessions_and_creates_nothing() {
    let home = TempDir::new().unwrap();
    let out = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["sessions"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("❯ ").not())
        .stdout(predicate::str::contains("\u{1b}").not());
    assert_eq!(
        String::from_utf8_lossy(&out.get_output().stdout),
        "No sessions.\n",
        "same text the REPL prints for an empty store"
    );
    assert!(
        !home.path().join("state.db").exists(),
        "subcommand must never create the canonical store"
    );
}

#[test]
fn inspect_shows_session_metadata_with_repl_parity() {
    let home = TempDir::new().unwrap();
    let c = seed_state_db(home.path());
    let before = canonical_rows(&home.path().join("state.db"));

    // Live REPL reference: `/inspect <id>` on the very same store.
    let repl = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake"])
        .write_stdin(format!("/inspect {SEED_ID_B}\n/exit\n"))
        .output()
        .unwrap();
    assert!(repl.status.success(), "repl must succeed");
    drop(c);

    let out = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["inspect", SEED_ID_B])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("❯ ").not())
        .stdout(predicate::str::contains("\u{1b}").not());
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    let expected = format!(
        "Session: {SEED_ID_B}\n\
         Source: fixture\n\
         Started: 1700000001.000\n\
         Turns: 1\n\
         Tool calls: 1\n"
    );
    assert_eq!(stdout, expected, "byte-exact metadata");
    let repl_meta = inspect_meta_lines(&String::from_utf8_lossy(&repl.stdout));
    assert_eq!(
        stdout,
        format!("{}\n", repl_meta.join("\n")),
        "shell and REPL must render identically"
    );
    assert_eq!(
        before,
        canonical_rows(&home.path().join("state.db")),
        "subcommand must not write state"
    );
}

#[test]
fn inspect_unknown_id_is_a_clear_error() {
    let missing = "99999999-9999-9999-9999-999999999999";
    // Store exists, session does not.
    let home = TempDir::new().unwrap();
    drop(seed_state_db(home.path()));
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["inspect", missing])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(format!("session not found: {missing}")));
    // No store at all: same clear error, still no file created.
    let empty = TempDir::new().unwrap();
    let out = hermes_cmd()
        .env("HERMES_HOME", empty.path())
        .args(["inspect", missing])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(format!("session not found: {missing}")));
    let _ = out;
    assert!(!empty.path().join("state.db").exists());
}

#[test]
fn inspect_malformed_id_is_a_clear_error() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["inspect", "abc-123"])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "invalid session id 'abc-123' (expected a UUID)",
        ));
}

#[test]
fn sessions_accepts_hermes_home_flag_after_subcommand() {
    let home = TempDir::new().unwrap();
    drop(seed_state_db(home.path()));
    // No HERMES_HOME env: the global flag (after the subcommand) resolves
    // the home instead — position-independent flags (Spec 014 principle).
    hermes_cmd()
        .args([
            "sessions",
            "--hermes-home",
            home.path().to_str().unwrap(),
        ])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "{SEED_ID_B}  started=1700000001.000  parity check B"
        )))
        .stdout(predicate::str::contains(format!(
            "{SEED_ID_A}  started=1700000000.000  parity check A"
        )));
}
