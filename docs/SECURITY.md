# Hermes-RS Tool Security Model

## Scope

Spec 002 tools execute with the permissions of the Hermes-RS process. They are not a substitute for an OS sandbox or container. Never run the CLI with untrusted prompts in a privileged account.

## Policy tiers

| Tool | Policy | Boundary |
|---|---|---|
| `read_file` | Always safe | Canonical path must remain inside cwd; 100 KB limit |
| `list_dir` | Always safe | Canonical path must remain inside cwd; 500 entry limit |
| `write_file` | Allowlist + confirmation | cwd-only, atomic temp-file rename, explicit `y`/`yes` |
| `shell_readonly` | Blocklist + confirmation | blocks destructive commands, pipes, redirects; 30 second timeout |

## Confirmation

Write and shell operations require an injected confirmation callback. The CLI defaults to deny: only `y` or `yes` authorizes an operation. EOF, malformed input, or callback failure denies execution.

## Path safety

Read and write tools reject absolute paths outside the root, lexical `..` traversal, invalid components, and symlink escapes for existing targets. New writes are staged inside the canonical root and renamed atomically.

## Cancellation and limits

Every tool receives a `CancellationToken`. Cancellation prevents new work and removes pending temporary writes. Shell commands have a bounded timeout. Output limits prevent accidental unbounded file/directory responses.

## Shell blocklist

Readonly shell blocks `rm`, `sudo`, `chmod`, `chown`, `curl`, `wget`, `dd`, `mkfs`, pipes, and redirects. This is defense-in-depth, not a complete command safety proof. `unsafe_mode` is an explicit API escape hatch and must never be enabled for untrusted input.

## Threat model

The model may generate malicious, surprising, or destructive tool arguments. The registry and each tool must independently enforce policy; prompt instructions are not authorization. Tool results are untrusted data and must not be interpreted as permission to broaden policy.

## Review requirements

Any new tool must define its root, input validation, output limits, timeout/cancellation behavior, confirmation policy, and tests for traversal, denial, and failure before registration in the CLI.

## Provider credentials (Spec 005 — per-provider `key_env` routing)

Provider API keys are resolved by `ProviderRegistry` when a provider is built.
This is a new credential surface, so its behavior is pinned below and governed
by the fallback chain in `crates/hermes-core/src/provider/registry.rs`
(`resolve_api_key`).

### Fallback chain (per configured provider)

1. `key_env` declared and its environment variable holds a non-empty value →
   use it.
2. `key_env` declared but the variable is unset/empty → **error** naming the
   variable. It must NOT fall back to `model.api_key`.
3. `key_env` absent → fall back to the global `model.api_key`, else error.

The legacy single-provider path (`model_level_fallback`, used only when no
`providers:` section selects the active provider) keeps its historical
`OPENAI_API_KEY` → `HERMES_API_KEY` → `model.api_key` order.

### STRIDE

- **Spoofing.** A `key_env` pin could be pointed at another provider's variable.
  The resolution only ever sends the key named by the active provider's own
  pin (or the explicit global `model.api_key`); it never guesses from
  well-known variable names for a configured provider. Because a pinned but
  empty `key_env` errors instead of falling back, one provider's key can never
  be silently forwarded to another provider's endpoint (no cross-provider
  credential leakage).
- **Tampering / Repudiation.** Credentials are wrapped in `SecretString` from
  resolution through the HTTP bearer header; they never transit as plain
  `String` outside the provider. Config is read-only at runtime.
- **Information disclosure.** This is the most common leak vector. Error and
  log messages name the missing *variable* (`environment variable 'X' is not
  set or empty`), never its value. The registry and `HttpProvider` redact
  credentials on every error/output path; the tests
  (`missing_key_env_names_the_variable_not_a_value`,
  `pinned_but_empty_key_env_does_not_fall_back_to_model_key`) assert no value
  ever appears in a message.
- **Elevation of privilege / DoS.** No new execution or network surface is
  introduced by key resolution; a failed resolution only prevents that one
  provider from being constructed and does not affect an already-active
  provider (rollback semantics of `/provider`).

## MCP servers (Spec 011 — Model Context Protocol client)

Hermes-RS can spawn child-process MCP servers (from `mcp_servers:` in
`config.yaml`) and register their `tools/list` tools into the `ToolRegistry`.
This is a **new execution surface**: each configured server launches an arbitrary
child command and its tools run that server's code.

- **Config is trusted input.** `command`/`args`/`env` come from the user's own
  `config.yaml`, the same trust level as provider config. Adding an MCP server
  is equivalent to granting a new capability; a user must know what a server can
  do before enabling it.
- **Disabled by default.** An empty `mcp_servers:` map spawns nothing and adds
  no tools, so configs written before Spec 011 behave unchanged.
- **Secret env redaction.** MCP `env` values may hold tokens. They are redacted
  (`***REDACTED***`) on every `Debug`/display path while the key name stays
  visible; values are never logged. The child gets exactly the `env` the user
  set — Hermes does not forward its own credentials to a server.
- **Confirmation gate (secure-by-default).** MCP tools default to requiring the
  Spec 002 confirmation callback; a server must set `confirm: false` explicitly
  to run its tools without per-call approval. This stops a configured server
  from executing silently. A decline surfaces `ToolError::Denied`, never
  retried.
- **Environment expansion.** `env` values may contain `${VAR}` / `$VAR`
  placeholders expanded from the process environment at spawn. An unset
  variable is an error naming the variable (never its value); expanded values
  are never logged.
- **Message-size limit.** Inbound JSON-RPC messages over 10 MB are rejected,
  guarding against an oversized server response (DoS).
- **REPL visibility.** `/mcp` lists each server and its status; `/mcp restart
  <name>` replaces a server's child process and re-discovers its tools.
- **Transport scope.** Hermes talks to the server only over that child's
  stdin/stdout (newline-delimited JSON-RPC). Hermes opens no network socket to
  the server; any network a server performs is the server's own configured
  behavior.
- **Limits.** Each MCP tool call has a bounded per-call timeout; a hanging server
  cannot wedge the agent forever. Servers failing to start/discover are reported
  and skipped without taking down the rest of the session. Child processes are
  killed on drop (`kill_on_drop`) when the session ends.

## TUI dashboard (Spec 012 — sanitization boundary & terminal safety)

The opt-in Ratatui dashboard (`--tui`) is **display-only**: it renders state
already produced by the shared engine and accepts input; it performs no tool
execution, network, or filesystem work of its own. Its security posture:

- **Sanitization at the source, not the renderer.** The agentic loop exposes
  *raw* data through a UI-free `AgentEvent` observer in `hermes-core`
  (`events.rs`). The CLI boundary (`worker::agent_event_to_tui`) scrubs every
  text field — ANSI/control sequences are stripped and credentials are redacted
  (`***REDACTED***`) — *before* it becomes a display `TuiEvent`. The renderer
  only ever sees clean text, so there is no second path that could forget to
  sanitize. A headless E2E injects a credential and an ANSI escape into a core
  event and asserts neither reaches the `TestBackend` buffer.
- **No new attack surface.** No I/O, network, or execution is added. All tool
  work still flows through the existing `chat_agentic` + `ToolRegistry`, and
  the TUI's interactive confirmation sink **denies by default** (`DenyConfirmation`),
  so a destructive tool cannot run without an explicit approve flow (matching
  Spec 002 / Spec 011b secure-by-default).
- **Terminal-state integrity.** Raw mode + alternate screen are entered under a
  `RawGuard` `Drop` guard that restores the terminal on *every* exit path —
  normal quit (`q`), Ctrl-C (exit 130 via the `interrupted` mapping), error, and
  unwind/panic. The terminal is never left in raw mode with the cursor hidden.
- **Non-`Send` session store handled without `unsafe`.** `SessionStore` wraps a
  `rusqlite` connection (not `Send`), so the TUI worker runs inline (as the
  REPL does) and is `select!`-ed against the blocking renderer, keeping the
  renderer responsive without shared-mutable access to the connection.
- **Input is a prompt, never a command.** Keystrokes go into a `Vec<char>`
  input buffer (multi-byte safe) and are forwarded to the engine as a prompt
  line — never executed directly. Cursor/backspace/history operate on the local
  buffer only.
