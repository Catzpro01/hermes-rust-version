# 014-01: Subcommand Parser Foundation
**Status:** DONE — menunggu review Matt (commit ini).

## Cakupan (Ticket 01)
Refactor `Args` di `crates/hermes-cli/src/main.rs` menjadi fondasi clap
`Subcommand` — tanpa mengubah perilaku default:

- **`enum Commands`** (clap `Subcommand`): `Model`, `Sessions`,
  `Inspect { id }`, `Messages { id }`, `ToolCalls { id }`,
  `Search { query }`, `Info`, `Mcp { action: Option<McpAction> }`.
- **`enum McpAction`** (nested subcommand): `List`, `Restart { name }` →
  shell: `hermes mcp`, `hermes mcp list`, `hermes mcp restart <name>`.
- Field baru di `Args`: `#[command(subcommand)] command: Option<Commands>`.
- **Zero regression:** `command == None` → alur REPL/TUI persis seperti
  HEAD (dispatch terjadi **sebelum** gate TUI, resolve home, config, dan
  provider — subcommand tidak pernah menyentuh state/TTY).
- **Flag global** (`global = true`): `--hermes-home`, `--provider`,
  `--api-url`, `--tui` — ter-parse sebelum **dan** sesudah subcommand.
- **Placeholder T01:** `coming soon: {name} (Spec 014)` ke stdout + exit 0;
  `{name}` shell-verbatim (termasuk kebab-case `tool-calls`). Output statis
  (tanpa data sesi/provider) → kontrak sanitasi CLI-boundary terpenuhi
  trivial; tidak ada byte kanonik yang ditulis.

## Bukti
- **Unit test** (`main.rs`, 5 test): parse semua 8 variant + 2 action MCP;
  flag global di dua sisi subcommand; pin nama + pesan placeholder
  (10 case, termasuk `tool-calls`).
- **E2E** (`tests/subcommands_e2e.rs`, 7 test):
  - `hermes-rs model` → `coming soon: model (Spec 014)`, exit 0, **tanpa
    prompt `❯ `**, `state.db` tidak dibuat;
  - `hermes-rs --provider fake` (tanpa subcommand) → tetap masuk REPL
    (`echo: hello` + `❯ `) — zero regression;
  - `model --provider fake` (flag setelah subcommand) → parse;
  - `mcp restart srv-1`, `tool-calls abc`, `search deploy` → placeholder;
  - `inspect` tanpa id → error clap (exit ≠ 0, stderr menyebut `id`).
- `cargo test --workspace` → TEST_RC=0, **412 passed / 0 failed** —
  log VM `/tmp/t014_test.log`
- `cargo clippy --workspace --all-targets -- -D warnings` → CLIPPY_RC=0 —
  log VM `/tmp/t014_clippy.log`

## STRIDE
- Tidak ada surface eksekusi/network/input baru: placeholder murni statis.
- Subcommand riil (T02–T07) hanya membaca `state.db`/registry yang sudah
  ada; output akan melewati jalur sanitasi + redaksi yang sama dengan REPL
  (boundary CLI stdout). `--tui` + subcommand → subcommand menang (tanpa
  raw-mode, aman di pipa).
