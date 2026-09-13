# hermes-rs: CLI subcommands

Module path: `crates/hermes-cli/src/subcommands.rs` (Spec 014)

Shell subcommands reuse the same data sources and output functions as the REPL (`session_menu`, `search`, MCP handles) so the shell and the REPL render identically. Output is colored only on a TTY: piped stdout stays ANSI-free, same invariant as banner and status bar.

Dispatch happens after `load_config` but before provider resolution and session creation (review Matt, T01): `hermes model` needs config.yaml only; session subcommands open `state.db` read-only. No subcommand ever enters REPL/TUI or creates a session.

## Implementation status (Spec 014) — all tickets landed

| Subcommand | File | Status | Notes |
|------------|------|--------|-------|
| `model` | `subcommands.rs:render_model()` | **Done** (T02) | Lists configured providers and models, active marker `*`, `--provider <name>` filter, unknown provider error. Colors: gold provider names, dim brown secondary. |
| `sessions` | `subcommands.rs` + `session_menu.rs:list_sessions()` | **Done** (T03) | Identical rendering to REPL `/sessions`. Missing store → "No sessions." Read-only, never creates store. |
| `inspect <id>` | `subcommands.rs` + `session_menu.rs:inspect_session()` | **Done** (T03) | Identical to REPL `/inspect`. Malformed UUID → clear error, non-zero exit. Missing store or unknown id → error. |
| `messages <id>` | `subcommands.rs` + `session_menu.rs:show_messages()` | **Done** (T04) | `[N] role: content`, identical to REPL `/messages`; sanitized at the stdout boundary. Same id/error contract as `inspect`. |
| `tool-calls <id>` | `subcommands.rs` + `session_menu.rs:show_tool_calls()` | **Done** (T04) | `<id> [status] <tool> args=… result=…`, identical to REPL `/tool-calls`; empty output for a session without calls. |
| `search <query>` | `subcommands.rs` + `session_menu.rs:search_sessions()` | **Done** (T05) | FTS5 + credential redaction (Spec 004), identical to REPL `/search`. Missing store → "No search results." without creating it. |
| `info` | `subcommands.rs:render_info()` | **Done** (T06) | Line 1 = the REPL `/info` line for a fresh session (`resolve_context` shared); then home, active provider, provider/MCP counts, session count. |
| `mcp [list\|restart <name>]` | `subcommands.rs:render_mcp()` | **Done** (T06) | REPL `/mcp` row layout from `config.yaml` only; status `configured` (the shell never spawns MCP children); `restart` points to the REPL, unknown name → error. `env` secrets never rendered. |
| `version` / `-V` / `--version` | `subcommands.rs:render_version()` | **Done** (T07) | `Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301` (banner `VERSION_LABEL`) + install facts. Dispatched before config load: works without a home or with a broken config. |
| `setup` | `subcommands.rs` | Spec 017 (wizard) | Interactive wizard skeleton; not part of Spec 014 scope. |

Global flags: `--hermes-home`, `--provider`, `--api-url`, `--tui`, `--version`, `--help` work before or after the subcommand (position-independent).

## Source mapping

- **CLI definition:** `src/cli.rs` (old, deprecated after Spec 014) vs `src/main.rs:Commands` (new Spec 014 enum)
  - New enum in `main.rs`: `Model`, `Sessions`, `Inspect { id }`, `Messages { id }`, `ToolCalls { id }`, `Search { query }`, `Info`, `Mcp { .. }`
  - Old `cli.rs` had 22+ commands (hermes, chat, serve, cloud, gateway, config, model, auth, tools, mcp, skills, plugins, sessions, memory, cron, acp, setup, doctor, status, logs, profile, backup, import, update, completion, uninstall) - now being migrated to Spec 014 board.

- **Dispatch:** `subcommands.rs:run()`:
  ```rust
  if matches!(cmd, Commands::Version) { return render_version(..) } // before config load
  let (home, config) = load_home_config(..)?;
  match cmd {
    Model => render_model(..),
    Sessions => list_sessions(&store),
    Inspect { id } => inspect_session(&store, id),
    Messages { id } => show_messages(&store, id),
    ToolCalls { id } => show_tool_calls(&store, id),
    Search { query } => search_sessions(&store, query),
    Info => render_info(..),
    Mcp { action } => render_mcp(..),
    Setup => wizard prompts,
  }
  ```
  The T01 `placeholder()` helper was removed in T08: every variant is real.

## Invariants (docs/ROADMAP.md)

- **state.db canonical:** Subcommands never create store, only open existing read-only (`open_existing_store()` checks `path.exists()`).
- **SIGINT exit 130:** REPL path, not subcommands (subcommands are non-interactive).
- **Credential redacted:** `render_model()` never prints API keys, only provider names and model names. `load_home_config()` loads config but redaction in Debug.
- **Python Hermes untouched:** `smoke_python_hermes_untouched` test.
- **Sanitasi hanya di CLI stdout boundary:** session/search renderers call `sanitize_untrusted_output` + `redact_credentials` at print time; canonical SQLite is never rewritten. Colors only on a TTY (`model`); everything else is plain in both REPL and shell.
- **No new execution surface:** `mcp` reads `config.yaml` only — the shell never spawns an MCP child (that stays REPL-only under Spec 011's STRIDE model).
- **Zero regression REPL/TUI:** `bare_invocation_still_enters_repl_zero_regression` test.

## Tests

- `crates/hermes-cli/src/main.rs` (unit): parser pins — every subcommand parses, `mcp` actions, global flags before/after the subcommand, `--version`/`-V`/`version`.
- `crates/hermes-cli/src/subcommands.rs` (unit): `render_model` (plain + SGR), `active_provider` precedence, `name()` pinned for all 12 variants, `render_version`, `render_info` (with/without config), `render_mcp` (sorted rows, empty, restart).
- `crates/hermes-cli/tests/subcommands_e2e.rs` (26 E2E against the real binary): every subcommand exits 0 without the REPL prompt and ANSI-free when piped; `sessions`, `inspect`, `messages`, `tool-calls`, `search` and `info` (line 1) are compared against the **live REPL output on the same store**; credentials never leak (`API_KEY=…` in messages, `sk-proj-…` in MCP `env`); read-only subcommands never create `state.db` and canonical rows are unchanged; a bare invocation still enters the REPL; `--version` works without a home / with a broken config; `--help` lists every subcommand and global flag.

## Limits and constants (cross-ref)

See `tools_summary.md` and `provider_architecture.md` for:
- `MAX_FILE_BYTES = 100_000` (100 KB)
- `list_dir = 500 entries`
- `RETRYABLE = [429, 500, 502, 503, 504]`
- `RetryPolicy: 3 attempts, 200ms base, 2000ms max`
- `HTTP_CLIENT_TIMEOUT = 30s`
- `DEFAULT_COOLDOWN = 60s`

---
*Audit 2026-09-06 - verified against source: subcommands.rs, main.rs, ROADMAP.md, provider/mod.rs. Updated 2026-09-13 for Spec 014 closure (T04–T08).*
