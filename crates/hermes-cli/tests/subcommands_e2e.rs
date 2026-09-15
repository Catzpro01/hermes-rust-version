//! Spec 014: shell subcommands — end-to-end.
//!
//! Guarantees: a subcommand runs to completion and exits 0 without entering
//! the REPL (no prompt, no state touch); a bare invocation still enters the
//! REPL exactly as before (zero regression); global flags parse on both
//! sides of the subcommand. `model` (T02) lists configured providers and
//! models with the active marker; `sessions` and `inspect` (T03) reuse the
//! REPL's own `session_menu` renderers so shell and REPL output is
//! byte-identical; `messages`, `tool-calls` (T04) and `search` (T05) reuse
//! the same renderers and are proven against the live REPL too; `info` and
//! `mcp` (T06) render from config only; `version`/`--version` (T07) is
//! static and never touches the home.

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
        .stdout(predicate::str::contains("models: claude-sonnet-4-5"))
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
fn mcp_restart_unknown_server_is_a_clear_error() {
    // Same contract as the REPL's `/mcp restart <unknown>`: error, non-zero.
    hermes_cmd()
        .env("HERMES_HOME", TempDir::new().unwrap().path())
        .args(["mcp", "restart", "srv-1"])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("no MCP server named 'srv-1'"));
}

const MCP_CONFIG: &str = "mcp_servers:\n  files:\n    command: npx\n    args: [\"-y\", \"srv\"]\n    env:\n      TOKEN: sk-proj-mcp-secret-fixture-0001\n  auto:\n    command: srv\n    confirm: false\n";

#[test]
fn mcp_list_renders_configured_servers_without_spawning_or_leaking() {
    let home = TempDir::new().unwrap();
    std::fs::write(home.path().join("config.yaml"), MCP_CONFIG).unwrap();
    for args in [vec!["mcp"], vec!["mcp", "list"]] {
        let out = hermes_cmd()
            .env("HERMES_HOME", home.path())
            .args(&args)
            .write_stdin("")
            .assert()
            .success()
            .stdout(predicate::str::contains("\u{1b}").not());
        let stdout = String::from_utf8_lossy(&out.get_output().stdout);
        assert_eq!(
            stdout,
            "MCP servers:\n  auto         configured ? tool(s) (auto mode)\n  files        configured ? tool(s) (confirm mode)\n",
            "{args:?}"
        );
        assert!(
            !stdout.contains("sk-proj-mcp-secret"),
            "env leaked: {stdout}"
        );
    }
    // The shell never creates the store or spawns anything that writes home.
    assert!(!home.path().join("state.db").exists());
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["mcp", "restart", "files"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "mcp[files]: restart is only available inside the REPL (`/mcp restart files`)",
        ));
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
        .stdout(predicate::str::contains(
            "provider: fake | estimated context: ~0 tokens | limit: none | window: 0/0 turns sent | pinned: 0 | compression: off\n",
        ))
        .stdout(predicate::str::contains(format!(
            "Hermes Home: {}\n",
            home.path().display()
        )))
        .stdout(predicate::str::contains("Active provider: fake (built-in)\n"))
        .stdout(predicate::str::contains("No config.yaml found\n"))
        // Spec 007b: sandbox is on by default (no config needed).
        .stdout(predicate::str::contains("sandbox: on | cwd="))
        .stdout(predicate::str::contains("Sessions: 0\n"))
        .stdout(predicate::str::contains("❯ ").not())
        .stdout(predicate::str::contains("\u{1b}").not());
    // Read-only: `info` never creates the store.
    assert!(!home.path().join("state.db").exists());
}

#[test]
fn info_first_line_matches_live_repl_info_on_a_fresh_session() {
    // Config with a per-provider context limit + compression, so every field
    // of the `/info` line is exercised. `fake` is the built-in provider the
    // REPL can actually start offline; the limit is read from `model`.
    let home = TempDir::new().unwrap();
    std::fs::write(
        home.path().join("config.yaml"),
        "model:\n  context_length: 4096\ncompression:\n  enabled: true\n  target_max_tokens: 2048\nproviders:\n  anthropic:\n    models:\n      claude-sonnet-4-5: {}\n",
    )
    .unwrap();
    let repl = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake"])
        .write_stdin("/info\n/exit\n")
        .output()
        .unwrap();
    assert!(
        repl.status.success(),
        "repl failed: stderr={} stdout={}",
        String::from_utf8_lossy(&repl.stderr),
        String::from_utf8_lossy(&repl.stdout)
    );
    let repl_stdout = String::from_utf8_lossy(&repl.stdout);
    let repl_line = repl_stdout
        .lines()
        .map(strip_prompt)
        .find(|l| l.starts_with("provider: "))
        .expect("REPL /info line")
        .to_owned();
    let out = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["info", "--provider", "fake"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("Providers configured: 1\n"))
        .stdout(predicate::str::contains("Sessions: 1\n")); // the REPL run above created one
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    assert_eq!(
        stdout.lines().next().unwrap(),
        repl_line,
        "shell `info` line 1 == REPL `/info`"
    );
    assert!(repl_line.contains("limit: 4096"), "{repl_line}");
    assert!(
        repl_line.contains("compression: on (target ~2048 tokens)"),
        "{repl_line}"
    );
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
        .stderr(predicate::str::contains(
            "invalid session id 'abc' (expected a UUID)",
        ));
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
        .stderr(predicate::str::contains(
            "invalid session id 'not-a-uuid' (expected a UUID)",
        ));
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
        .stderr(predicate::str::contains(format!(
            "session not found: {missing}"
        )));
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["messages", missing])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(format!(
            "session not found: {missing}"
        )));
}

#[test]
fn search_without_store_says_no_results_and_creates_nothing() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["search", "deploy"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::is_match("^No search results.\n$").unwrap());
    assert!(!home.path().join("state.db").exists());
}

#[test]
fn search_hits_match_live_repl_and_redact_credentials() {
    let home = TempDir::new().unwrap();
    let c = seed_state_db(home.path());
    c.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES (?1,'assistant','parity deploy with API_KEY=super-secret-fixture-xyz',1700000002.0)",
        [SEED_ID_A],
    )
    .unwrap();
    drop(c);
    let db = home.path().join("state.db");
    // Live REPL reference on the same store.
    let repl = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake"])
        .write_stdin("/search parity\n/exit\n")
        .output()
        .unwrap();
    assert!(repl.status.success());
    let repl_stdout = String::from_utf8_lossy(&repl.stdout);
    let repl_lines: Vec<String> = repl_stdout
        .lines()
        .map(strip_prompt)
        .skip_while(|l| !l.starts_with("Search results for: "))
        .take_while(|l| {
            l.starts_with("Search results for: ")
                || (l.starts_with('[') && l[1..].starts_with(|c: char| c.is_ascii_digit()))
                || l.starts_with("  ")
        })
        .map(str::to_owned)
        .collect();
    assert!(repl_lines.len() >= 3, "REPL search output: {repl_stdout}");
    // Canonical snapshot after the REPL run (which appended its own session).
    let before = canonical_rows(&db);
    let out = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["search", "parity"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Search results for: parity\n"))
        .stdout(predicate::str::contains("***REDACTED***"))
        .stdout(predicate::str::contains("super-secret-fixture-xyz").not())
        .stdout(predicate::str::contains("❯ ").not())
        .stdout(predicate::str::contains("\u{1b}").not());
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    assert_eq!(
        stdout,
        format!("{}\n", repl_lines.join("\n")),
        "shell and REPL search must render identically"
    );
    // Read-only over canonical tables (the FTS index is derived state only).
    assert_eq!(before, canonical_rows(&db));
}

#[test]
fn messages_and_tool_calls_match_live_repl_and_write_nothing() {
    let home = TempDir::new().unwrap();
    let c = seed_state_db(home.path());
    // An ANSI-carrying assistant turn proves sanitization at the boundary.
    c.execute(
        "INSERT INTO messages(session_id,role,content,timestamp) VALUES (?1,'assistant','reply \u{1b}[31mred\u{1b}[0m done',1700000001.7)",
        [SEED_ID_B],
    )
    .unwrap();
    drop(c);
    let db = home.path().join("state.db");
    let before = canonical_rows(&db);

    let repl = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake"])
        .write_stdin(format!(
            "/messages {SEED_ID_B}\n/tool-calls {SEED_ID_B}\n/exit\n"
        ))
        .output()
        .unwrap();
    assert!(repl.status.success());
    let repl_stdout = String::from_utf8_lossy(&repl.stdout);
    // `[N] role: content` lines only (the banner's `[context …]` line also
    // starts with `[`, so require a digit after the bracket).
    let repl_msgs: Vec<String> = repl_stdout
        .lines()
        .map(strip_prompt)
        .filter(|l| l.starts_with('[') && l[1..].starts_with(|c: char| c.is_ascii_digit()))
        .map(str::to_owned)
        .collect();
    let repl_calls: Vec<String> = repl_stdout
        .lines()
        .map(strip_prompt)
        .filter(|l| l.starts_with("tc-b-1 "))
        .map(str::to_owned)
        .collect();

    let out = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["messages", SEED_ID_B])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("❯ ").not())
        .stdout(predicate::str::contains("\u{1b}").not());
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    assert_eq!(
        stdout,
        "[1] user: parity check B\n[2] assistant: reply red done\n"
    );
    assert_eq!(
        stdout,
        format!("{}\n", repl_msgs.join("\n")),
        "messages: shell == REPL"
    );

    let out = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["tool-calls", SEED_ID_B])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("\u{1b}").not());
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    assert_eq!(stdout, "tc-b-1 [success] fixture_tool args={} result=ok\n");
    assert_eq!(
        stdout,
        format!("{}\n", repl_calls.join("\n")),
        "tool-calls: shell == REPL"
    );

    // A session with no tool calls prints nothing and exits 0.
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["tool-calls", SEED_ID_A])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
    // The piped REPL resumes an existing session and may touch its own rows;
    // the seeded rows must survive both runs untouched.
    let after = canonical_rows(&db);
    for row in &before {
        assert!(after.contains(row), "seeded row lost/modified: {row}");
    }
}

// ---------------------------------------------------------------------------
// T07: `--version` / `hermes version` / `--help`
// ---------------------------------------------------------------------------

const VERSION_LABEL: &str = "Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301";

#[test]
fn version_flag_and_subcommand_render_the_banner_label() {
    let home = TempDir::new().unwrap();
    let mut outputs = Vec::new();
    for args in [
        vec!["--version"],
        vec!["-V"],
        vec!["version"],
        vec!["info", "--version"],
    ] {
        let out = hermes_cmd()
            .env("HERMES_HOME", home.path())
            .args(&args)
            .write_stdin("")
            .assert()
            .success()
            .stdout(predicate::str::starts_with(format!("{VERSION_LABEL}\n")))
            .stdout(predicate::str::contains("Install directory: "))
            .stdout(predicate::str::contains("Install method: cargo\n"))
            .stdout(predicate::str::contains("Crate version: 0.21.0\n"))
            .stdout(predicate::str::contains("❯ ").not())
            .stdout(predicate::str::contains("\u{1b}").not());
        outputs.push(String::from_utf8_lossy(&out.get_output().stdout).into_owned());
    }
    assert!(
        outputs.windows(2).all(|w| w[0] == w[1]),
        "all forms identical: {outputs:?}"
    );
    assert!(!home.path().join("state.db").exists());
}

#[test]
fn version_works_without_a_hermes_home_or_with_a_broken_config() {
    // Missing home: still exit 0 (Python `--version` never needs a profile).
    hermes_cmd()
        .env("HERMES_HOME", "/nonexistent/hermes-home-for-tests")
        .args(["--version"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(VERSION_LABEL));
    // Broken config.yaml: `version` must not even parse it.
    let home = TempDir::new().unwrap();
    std::fs::write(home.path().join("config.yaml"), "model: [unclosed").unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["version"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(VERSION_LABEL));
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["info"])
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid config"));
}

#[test]
fn help_lists_every_subcommand_and_global_flags() {
    let out = hermes_cmd()
        .args(["--help"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("\u{1b}").not());
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    for cmd in [
        "setup",
        "version",
        "model",
        "tools",
        "sessions",
        "inspect",
        "messages",
        "tool-calls",
        "search",
        "info",
        "mcp",
        "help",
    ] {
        assert!(
            stdout.lines().any(|l| l.trim_start().starts_with(cmd)),
            "help must list `{cmd}`: {stdout}"
        );
    }
    for flag in [
        "--hermes-home",
        "--provider",
        "--api-url",
        "--tui",
        "--version",
        "--help",
    ] {
        assert!(stdout.contains(flag), "help must list `{flag}`: {stdout}");
    }
    assert!(
        !stdout.contains("--setup-skeleton"),
        "hidden flag must stay hidden"
    );
    // Subcommand help is also available.
    hermes_cmd()
        .args(["mcp", "--help"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("restart"))
        .stdout(predicate::str::contains("list"));
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
        .stderr(predicate::str::contains(format!(
            "session not found: {missing}"
        )));
    // No store at all: same clear error, still no file created.
    let empty = TempDir::new().unwrap();
    let out = hermes_cmd()
        .env("HERMES_HOME", empty.path())
        .args(["inspect", missing])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(format!(
            "session not found: {missing}"
        )));
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
        .args(["sessions", "--hermes-home", home.path().to_str().unwrap()])
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

// ---------------------------------------------------------------------------
// Spec 007: tool execution sandbox — config surface
// ---------------------------------------------------------------------------

#[test]
fn sandbox_config_is_reported_by_info_and_repl_sandbox_command() {
    let home = TempDir::new().unwrap();
    std::fs::write(
        home.path().join("config.yaml"),
        "sandbox:\n  enabled: true\n  cpu_seconds: 5\n  max_output_kb: 8\n  env_allowlist: [CARGO_HOME]\n",
    )
    .unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["info"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("sandbox: on | cwd="))
        .stdout(predicate::str::contains("CARGO_HOME"))
        .stdout(predicate::str::contains("output<=8KiB"))
        .stdout(predicate::str::contains("cpu=5s"))
        .stdout(predicate::str::contains("network=inherit"));
    // The REPL shows the same summary line via `/sandbox`.
    let repl = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake"])
        .write_stdin("/sandbox\n/exit\n")
        .output()
        .unwrap();
    assert!(repl.status.success());
    let stdout = String::from_utf8_lossy(&repl.stdout);
    assert!(
        stdout
            .lines()
            .map(strip_prompt)
            .any(|l| l.starts_with("sandbox: on | cwd=") && l.contains("cpu=5s")),
        "{stdout}"
    );
}

#[test]
fn invalid_sandbox_config_fails_at_load_time() {
    let home = TempDir::new().unwrap();
    std::fs::write(
        home.path().join("config.yaml"),
        "sandbox:\n  enabled: true\n  network: dney\n",
    )
    .unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["info"])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "invalid sandbox config field `network`",
        ));
    // `version` is still fine (never loads config).
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["version"])
        .write_stdin("")
        .assert()
        .success();
}

/// Spec 007b — `--no-sandbox` (global flag) and `sandbox.enabled: false`
/// both switch the boundary shell tools to the inherit policy; the REPL's
/// `/sandbox` agrees with `hermes info` in every combination.
#[test]
fn no_sandbox_flag_and_enabled_false_yield_inherit() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["info", "--no-sandbox"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("sandbox: off (inherit)\n"));
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--no-sandbox", "info"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("sandbox: off (inherit)\n"));
    let repl = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake", "--no-sandbox"])
        .write_stdin("/sandbox\n/exit\n")
        .output()
        .unwrap();
    assert!(repl.status.success());
    let stdout = String::from_utf8_lossy(&repl.stdout);
    assert!(
        stdout
            .lines()
            .map(strip_prompt)
            .any(|l| l == "sandbox: off (inherit)"),
        "{stdout}"
    );

    std::fs::write(
        home.path().join("config.yaml"),
        "sandbox:\n  enabled: false\n",
    )
    .unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["info"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("sandbox: off (inherit)\n"));
    // A `sandbox:` section without `enabled` is still on.
    std::fs::write(
        home.path().join("config.yaml"),
        "sandbox:\n  cpu_seconds: 2\n",
    )
    .unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["info"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("sandbox: on | cwd="))
        .stdout(predicate::str::contains("cpu=2s"));
}

// Spec 017 T09: `hermes sessions browse` — piped stdin takes the numbered
// non-curses fallback (spec §F L1639): verbatim header, numbered rows
// (newest first), `q`/EOF cancels, a number selects.
#[test]
fn sessions_browse_fallback_selects_cancels_and_writes_nothing() {
    let home = TempDir::new().unwrap();
    let c = seed_state_db(home.path());
    let before = canonical_rows(&home.path().join("state.db"));

    let out = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["sessions", "browse"])
        .write_stdin("1\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\u{1b}").not()) // piped -> ANSI-free
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(
        stdout.contains("  Browse sessions  (enter number to resume, q to cancel)"),
        "verbatim fallback header: {stdout}"
    );
    assert!(
        stdout.contains("Title / Preview"),
        "column header: {stdout}"
    );
    assert!(
        stdout.contains(&format!("Selected session {SEED_ID_B}")),
        "1 = newest first: {stdout}"
    );
    assert!(
        stdout.contains(&format!("hermes-rs --resume-id {SEED_ID_B}")),
        "resume hint: {stdout}"
    );

    // `q` and EOF cancel with exit 0 and no selection.
    for stdin in ["q\n", ""] {
        let out = hermes_cmd()
            .env("HERMES_HOME", home.path())
            .args(["sessions", "browse"])
            .write_stdin(stdin)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let stdout = String::from_utf8_lossy(&out);
        assert!(
            !stdout.contains("Selected session"),
            "cancelled: {stdout:?}"
        );
    }
    drop(c);
    assert_eq!(
        before,
        canonical_rows(&home.path().join("state.db")),
        "browse must not write state"
    );
}

#[test]
fn sessions_browse_without_store_says_no_sessions_found_and_creates_nothing() {
    let home = TempDir::new().unwrap();
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["sessions", "browse"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("No sessions found.\n"))
        .stdout(predicate::str::contains("\u{1b}").not());
    assert!(
        !home.path().join("state.db").exists(),
        "browse must never create the canonical store"
    );
}

#[test]
fn sessions_help_lists_browse() {
    hermes_cmd()
        .args(["sessions", "--help"])
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("browse"));
}

// Spec 017 T09: `--resume-id <id>` reopens one specific session (the shell
// picker's selection stays actionable without entering the REPL from a
// subcommand). Unknown/malformed ids are clear errors with exit 1.
#[test]
fn resume_id_opens_the_named_session() {
    let home = TempDir::new().unwrap();
    let c = seed_state_db(home.path());
    drop(c);
    let out = hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake", "--resume-id", SEED_ID_A])
        .write_stdin("/exit\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(
        stdout.contains(&format!("Hermes-RS session {SEED_ID_A} (provider fake)")),
        "piped header names the resumed session: {stdout}"
    );

    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args([
            "--provider",
            "fake",
            "--resume-id",
            "00000000-0000-4000-8000-000000000000",
        ])
        .write_stdin("/exit\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("session not found"));
    hermes_cmd()
        .env("HERMES_HOME", home.path())
        .args(["--provider", "fake", "--resume-id", "not-a-uuid"])
        .write_stdin("/exit\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid session id"));
}
