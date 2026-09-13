# 014-06: `hermes info` + `hermes mcp [list|restart <name>]`
**Status:** DONE — menunggu review Matt.

## Cakupan (Ticket 06)

### `hermes info`
Render lewat `subcommands::render_info` (pure, unit-testable).
- **Baris 1** memakai **bentuk persis** baris `/info` REPL untuk sesi baru:
  `provider: <p> | estimated context: ~0 tokens | limit: <n|none> | window:
  0/0 turns sent | pinned: 0 | compression: <off|on (target ~N tokens)|on
  (no target)>`. `limit`/`compression` dihitung dengan
  `repl::resolve_context` yang sama (precedence provider → model →
  compression). Shell tidak punya turn in-flight, jadi token/window/pinned
  = 0 (fakta, bukan placeholder).
- **Baris berikutnya (shell-only):** `Hermes Home:`, `Active provider:`
  (`fake (built-in)` bila tanpa config), `Providers configured:`, `Model
  provider:` (bila diset), `MCP servers configured:`, `Sessions:` (0 bila
  `state.db` tidak ada — tidak dibuat).
- Provider aktif: `--provider` > `model.provider` (≠ `auto`) > `fake`
  (`active_provider`, sama dengan T02).

### `hermes mcp` / `mcp list` / `mcp restart <name>`
Render lewat `subcommands::render_mcp` dari `config.yaml` saja.
- **Tidak menambah surface eksekusi:** shell **tidak pernah men-spawn**
  child MCP (invariant ROADMAP "new execution surface butuh STRIDE"; spawn
  tetap eksklusif REPL Spec 011). Karena itu kolom status membaca
  `configured` dan jumlah tool `?`, dengan **layout baris REPL yang sama**
  `  {:<12} {:<10} {} tool(s) ({} mode)`; mode = `confirm`/`auto` dari
  `confirm` (default true, Spec 011b). Nama diurutkan.
- Tanpa server → teks REPL yang sama: `no MCP servers connected (add
  `mcp_servers:` to config.yaml)`.
- `restart <name>`: nama dikenal → pointer jelas ke `/mcp restart <name>`
  di REPL (exit 0); nama tak dikenal → `error: no MCP server named
  '<name>'`, exit 1 (pesan REPL yang sama).
- **Redaksi:** nilai `env` (rahasia) tidak pernah dirender — hanya nama
  server dan mode.

## Bukti
- **Unit** (`subcommands.rs`): `info_first_line_matches_repl_shape`,
  `info_without_config_reports_builtin_fake`,
  `mcp_list_uses_repl_row_layout_sorted`,
  `mcp_without_servers_matches_repl_message`,
  `mcp_restart_points_to_repl_and_rejects_unknown_name`.
- **E2E** (`tests/subcommands_e2e.rs`):
  `info_subcommand_shows_home_and_provider` (tanpa config, tidak membuat
  store), `info_first_line_matches_live_repl_info_on_a_fresh_session`
  (config dengan `context_length` + compression aktif; baris 1 ==
  `/info` REPL hidup),
  `mcp_list_renders_configured_servers_without_spawning_or_leaking`
  (2 server, `env` rahasia tidak bocor, `mcp` == `mcp list`, restart
  pointer), `mcp_restart_unknown_server_is_a_clear_error`,
  `mcp_list_with_no_config`.
