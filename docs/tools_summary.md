# hermes-core: tools module

Module path: `crates/hermes-core/src/tools/`

Tool-calling primitives for Spec 002: explicit, policy-gated execution of model-issued tool calls. All tools are async (`Tool` trait via `async-trait`), cancellable through a `tokio_util::sync::CancellationToken`, and error-typed via `ToolError` (`thiserror`).

## Files

| File | Lines | Purpose |
|------|------:|---------|
| `mod.rs` | 202 | Core types, `Tool` trait, `ToolRegistry`, re-exports, `ToolExecutionStatus` |
| `readonly.rs` | 183 | `read_file` and `list_dir` tools with root jail + size caps |
| `shell.rs` | ~190 | `shell` and `shell_readonly` tools with confirmation + blocklist; both spawn via `sandbox::run_shell` |
| `sandbox.rs` | ~480 | Spec 007 `SandboxPolicy` (env allowlist, cwd, output cap, `ulimit` rlimits, `unshare --net`), single spawn site |
| `write.rs` | 194 | `write_file` tool with confirmation, root jail, atomic write |

## Core types (`mod.rs`)

- **`ToolCall`** — model-issued call: `id: Option<String>`, `name: String`, `arguments: String`.
- **`ToolResponse`** — result: `id`, `name`, `content`, `success: bool`.
- **`ToolExecutionStatus`** — `success`, `error`, `denied`, `timeout`, `cancelled` (persisted in `tool_calls` table).
- **`Tool`** trait — `async fn execute(&self, call: &ToolCall, cancel: CancellationToken) -> Result<ToolResponse, ToolError>`
- **`ToolRegistry`** — central registry, `register()`, `get()`, `list()`.

## Tools detail

### `read_file`

- **Description:** "Read a UTF-8 file under the tool root, limited to 100 KB."
- **Limit:** `MAX_FILE_BYTES = 100_000` (100 KB) in `readonly.rs:8`
- **Behavior:** Reads file, truncates at 100 KB, appends "\n[truncated at 100 KB]" if truncated. Returns `ToolResponse` with content.
- **Safety:** Root jail - path must be under tool root, no `..` escape. UTF-8 validated.
- **Cancellation:** `tokio::select! { _ = cancel.cancelled() => Err(Cancelled), result = fs::read() }`

### `list_dir`

- **Description:** "List entries under the tool root, limited to 500 entries."
- **Limit:** 500 entries in `readonly.rs:104`
- **Behavior:** Lists directory entries, if >500, truncates and appends "[truncated at 500 entries]".
- **Safety:** Root jail, same as read_file.

### `shell` and `shell_readonly`

- **Description:** Execute shell command with confirmation and blocklist.
- **Safety:** Dangerous patterns checked via `approval.rs`, confirmation gate.
- **Timeout:** Configurable, default via `Tool` impl.
- **Cancellation:** `tokio::select!` against cancel token, timeout via `tokio::time::timeout`.

### Sandbox (Spec 007, `sandbox.rs`)

- **`SandboxPolicy`** — `inherit()` (default, == Spec 002) or `strict(root)` / `from_config(cfg, root)`.
- **Env:** `env_clear` + `DEFAULT_ENV_ALLOWLIST` (`PATH HOME LANG LC_ALL LC_CTYPE TERM TZ USER SHELL TMPDIR`) + config names; sets `HERMES_SANDBOX=1`.
- **Output:** `DEFAULT_MAX_OUTPUT_BYTES = 64 KiB`, marker `[output truncated by sandbox]`.
- **Limits:** `ulimit -t/-f/-u/-v` via `sh -c '… && exec sh -c "$1"' hermes-sandbox <cmd>` (command is `$1`, never interpolated).
- **Network:** `NetworkPolicy::Deny` → `unshare --user --map-root-user --net --`; fails closed when unavailable.
- **Order:** blocklist → confirmation → sandbox; timeout + cancellation still apply.

### `write_file`

- **Description:** Write file with confirmation, root jail, atomic write.
- **Safety:** Confirmation required, root jail, atomic write via temp file + rename.
- **Cancellation:** Checks `cancel.is_cancelled()` before write, and after write before rename; removes tmp file if cancelled - ensures `cancelled turn never persisted partially` invariant.
- **Behavior:** `fs::write(tmp, content)` → `fs::rename(tmp, target)` atomic.

## Provider module architecture (cross-ref)

For provider-related limits (retry, timeout, cooldown), see `provider_architecture.md`:

| Constant | Value | Location | Purpose |
|----------|-------|----------|---------|
| `MAX_FILE_BYTES` | 100_000 (100 KB) | `tools/readonly.rs:8` | read_file limit |
| `list_dir limit` | 500 entries | `tools/readonly.rs:104` | list_dir truncation |
| `RETRYABLE_HTTP_STATUS` | [429, 500, 502, 503, 504] | `provider/mod.rs:16` | Retryable HTTP statuses (pinned by test) |
| `RetryPolicy.max_attempts` | 3 | `provider/http.rs:34` | Bounded retry attempts |
| `RetryPolicy.base_delay` | 200ms | `provider/http.rs:35` | Exponential backoff base |
| `RetryPolicy.max_delay` | 2000ms (2s) | `provider/http.rs:36` | Backoff cap |
| `HTTP_CLIENT_TIMEOUT` | 30s | `provider/http.rs:378` | Per-request timeout (DoS vector) |
| `DEFAULT_COOLDOWN` | 60s | `provider/health.rs:18` | Per-provider cooldown (circuit breaker) |

## Safety and invariants

- **Root jail:** All file tools enforce path under tool root.
- **Size caps:** 100 KB read, 500 entries list prevent DoS.
- **Cancellation:** All tools honor `CancellationToken`, return `ToolError::Cancelled`, never partial persist.
- **Atomic write:** write_file uses tmp + rename, removes tmp on cancel.
- **No credential leak:** ToolRegistry never logs arguments, credential redaction at output boundary (Spec 003).

## Tests

- `crates/hermes-core/tests/` - agentic_loop, conversation, provider_fallback
- `crates/hermes-cli/tests/search_credential_safety.rs` - proves redaction
- Property tests for tool parsing.

---
*Audit 2026-09-06 - verified against source: readonly.rs, mod.rs, provider/mod.rs, provider/http.rs, provider/health.rs*
