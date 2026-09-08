# hermes-rs: CLI subcommands

Module path: `crates/hermes-cli/src/subcommands.rs` (Spec 014)

Shell subcommands reuse the same data sources and output functions as the REPL (`session_menu`, `search`, MCP handles) so the shell and the REPL render identically. Output is colored only on a TTY: piped stdout stays ANSI-free, same invariant as banner and status bar.

Dispatch happens after `load_config` but before provider resolution and session creation (review Matt, T01): `hermes model` needs config.yaml only; session subcommands open `state.db` read-only. No subcommand ever enters REPL/TUI or creates a session.

## Implementation status (Spec 014)

| Subcommand | File | Status | Notes |
|------------|------|--------|-------|
| `model` | `subcommands.rs:render_model()` | **Done** (T02) | Lists configured providers and models, active marker `*`, `--provider <name>` filter, unknown provider error. Colors: gold provider names, dim brown secondary. |
| `sessions` | `subcommands.rs` + `session_menu.rs:list_sessions()` | **Done** (T03) | Identical rendering to REPL `/sessions`. Missing store → "No sessions." Read-only, never creates store. |
| `inspect <id>` | `subcommands.rs` + `session_menu.rs:inspect_session()` | **Done** (T03) | Identical to REPL `/inspect`. Malformed UUID → clear error, non-zero exit. Missing store or unknown id → error. |
| `messages <id>` | `subcommands.rs:placeholder()` | **Placeholder** (T04) | Returns `coming soon: messages (Spec 014)` - static, no state/provider/network, sanitization trivially satisfied. |
| `tool-calls <id>` | `subcommands.rs:placeholder()` | **Placeholder** (T04) | `coming soon: tool-calls (Spec 014)` |
| `search <query>` | `subcommands.rs:placeholder()` | **Placeholder** (T05) | `coming soon: search (Spec 014)` - FTS5 + redaction impl pending, but core search already works in REPL via `session_menu.rs:search_sessions()` |
| `info` | `subcommands.rs:placeholder()` | **Placeholder** (T05) | `coming soon: info (Spec 014)` |
| `mcp` | `subcommands.rs:placeholder()` | **Placeholder** (T06) | `coming soon: mcp (Spec 014)` |

Global flags: `--hermes-home`, `--provider`, `--api-url`, `--version`, `--help` work after subcommand (parity).

## Source mapping

- **CLI definition:** `src/cli.rs` (old, deprecated after Spec 014) vs `src/main.rs:Commands` (new Spec 014 enum)
  - New enum in `main.rs`: `Model`, `Sessions`, `Inspect { id }`, `Messages { id }`, `ToolCalls { id }`, `Search { query }`, `Info`, `Mcp { .. }`
  - Old `cli.rs` had 22+ commands (hermes, chat, serve, cloud, gateway, config, model, auth, tools, mcp, skills, plugins, sessions, memory, cron, acp, setup, doctor, status, logs, profile, backup, import, update, completion, uninstall) - now being migrated to Spec 014 board.

- **Dispatch:** `subcommands.rs:run()`:
  ```rust
  match cmd {
    Model => render_model(),
    Sessions => list_sessions(),
    Inspect { id } => inspect_session(),
    other => println!("{}", placeholder(other)),
  }
  ```

- **Placeholder impl:**
  ```rust
  pub fn placeholder(cmd: &Commands) -> String {
    format!("coming soon: {} (Spec 014)", name(cmd))
  }
  ```

## Invariants (docs/ROADMAP.md)

- **state.db canonical:** Subcommands never create store, only open existing read-only (`open_existing_store()` checks `path.exists()`).
- **SIGINT exit 130:** REPL path, not subcommands (subcommands are non-interactive).
- **Credential redacted:** `render_model()` never prints API keys, only provider names and model names. `load_home_config()` loads config but redaction in Debug.
- **Python Hermes untouched:** `smoke_python_hermes_untouched` test.
- **Sanitasi hanya di CLI stdout boundary:** Placeholder static output, no ANSI unless TTY, same as banner/status_bar.
- **Zero regression REPL/TUI:** `bare_invocation_still_enters_repl_zero_regression` test.

## Tests

- `tests/subcommands_e2e.rs`:
  - `global_flags_parse_after_subcommand`
  - `inspect_requires_an_id`
  - `mcp_subcommand_placeholder_with_action`
  - `model_lists_configured_providers_with_active_marker`
  - `model_provider_flag_filters_to_one_provider`
  - `model_without_config_shows_builtin_fake`
  - `search_subcommand_placeholder`
  - `tool_calls_subcommand_parses_kebab_case`
  - `bare_invocation_still_enters_repl_zero_regression`

## Limits and constants (cross-ref)

See `tools_summary.md` and `provider_architecture.md` for:
- `MAX_FILE_BYTES = 100_000` (100 KB)
- `list_dir = 500 entries`
- `RETRYABLE = [429, 500, 502, 503, 504]`
- `RetryPolicy: 3 attempts, 200ms base, 2000ms max`
- `HTTP_CLIENT_TIMEOUT = 30s`
- `DEFAULT_COOLDOWN = 60s`

---
*Audit 2026-09-06 - verified against source: subcommands.rs, main.rs, ROADMAP.md, provider/mod.rs*
