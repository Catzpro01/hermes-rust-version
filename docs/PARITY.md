# Hermes-RS Parity Report

## Compatible ✅

| Feature | Status | Notes |
|---|---|---|
| Hermes home resolution | ✅ | `HERMES_HOME` > explicit CLI path > `~/.hermes` |
| `config.yaml` parsing | ✅ | Read-only, compatible schema |
| SQLite `state.db` | ✅ | Hermes schema, WAL mode, concurrent writes |
| Session create/resume/list | ✅ | UUID v7 (Python uses UUID v4) |
| OpenAI-compatible provider | ✅ | `/v1/chat/completions`, Bearer auth, redaction |
| Streaming response | ✅ | SSE chunks rendered as they arrive |
| Cancellation | ✅ | Ctrl-C cancels active HTTP stream; partial turn is discarded |
| Config-declared providers | ✅ | `providers:` in `config.yaml`, selected by `--provider` / `model.provider` |
| Per-provider credential env | ✅ | `key_env` names the env var a provider uses (see Spec 005 STRIDE) |
| Mid-session provider switch | ✅ | `/provider <name>` preserves session; failures roll back |
| Wire-mode routing | ✅ | `api_mode: chat_completions | completions` selects endpoint/payload |

## Spec 005 — provider routing parity

Rust routing now matches the config-declared, per-provider model the installed
Python Hermes uses for user-defined `providers:` entries:

- **Selection.** Both resolve the active provider from config (CLI override
  wins over `model.provider`, falling back to a default). Rust falls back to
  the built-in offline `fake` when nothing is configured.
- **Credential.** Python providers read their key from per-provider env vars;
  Rust reads `key_env` with an explicit fallback chain
  (`key_env` → `model.api_key` → error). A pinned-but-empty `key_env` errors
  rather than silently borrowing another key, preventing cross-provider
  credential leakage.
- **Mid-session switch.** Python persists a session across provider/model
  changes; Rust `/provider` swaps the backing provider on `ConversationRunner`
  without touching `self.turns`, and only at a turn boundary. A provider whose
  construction fails is rolled back, leaving the active one untouched.
- **Wire mode.** `api_mode` selects between the chat-completions and the legacy
  completions endpoints; streaming is normalized to the same provider-neutral
  event sequence so a switch is transparent to callers.

## Differences ⚠️

## Differences ⚠️

| Feature | Python | Rust | Impact |
|---|---|---|---|
| Session ID format | UUID v4 | UUID v7 | Low — schema-compatible and time-sortable |
| FTS index | Enabled | Not yet | Low — search is not implemented |
| Provider catalog | Many built-ins + plugins | Config-declared + built-in `fake` | Medium — Rust has no dynamic plugin loading |
| Tool execution | Python sandbox | Native shell integration | High — different security model |
| TUI rendering | Rich/curses | Plain stdout | Low — cosmetic |

## Known Gaps 🚧

- Function calling / tool use (planned Spec 002)
- FTS5 search on message content
- Dynamic plugin/provider loading
- Conversation branching and edit

## Testing

```bash
cargo test --test cli_e2e
cargo test --test smoke
./scripts/smoke-test.sh
```

## Verification notes

The checked-in Hermes-compatible fixture reports `sessions.id` as `TEXT`, with UUID values stored as 36-character strings. The live Python Hermes home on the validation VM did not contain `state.db`, so no claim is made about an existing live Python session database.

The config audit covers the currently supported Hermes-RS schema: `model.default`, `model.provider`, `model.base_url`, and `model.api_key`. Unknown fields remain ignored by serde for forward compatibility.

## Operational SIGINT verification

The Unix integration test `sigint_stream` synchronizes on the first SSE chunk before sending `SIGINT`. It verifies exit code 130 and zero persisted assistant messages after cancellation.

## Spec 002 tool status

| Capability | Status | Notes |
|---|---|---|
| XML tool-call parsing | ✅ | Streaming tag-aware parser |
| Agentic loop | ✅ | Cancellable, maximum 10 iterations |
| SQLite tool-call records | ✅ | Tool name, arguments, result, status |
| Read-only filesystem tools | ✅ | cwd jail and output limits |
| Confirmed writes | ✅ | Atomic write and default-deny confirmation |
| Readonly shell policy | ✅ | Blocklist, timeout, cancellation |
| Security documentation | ✅ | See `docs/SECURITY.md` |
