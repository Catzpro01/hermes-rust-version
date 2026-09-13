# 014-04: `hermes messages <id>` + `hermes tool-calls <id>`
**Status:** DONE — menunggu review Matt.

## Cakupan (Ticket 04)
Dua subcommand transkrip di `crates/hermes-cli/src/subcommands.rs`, memakai
**fungsi render REPL yang sama** (`session_menu::show_messages` /
`session_menu::show_tool_calls`) sehingga shell dan REPL identik byte-level.

- **`hermes messages <id>`** → satu baris per turn:
  `[N] <role>: <content>` (role = `user` / `assistant` / nama tool; konten
  & role disanitasi — ANSI/C0 dibuang di boundary stdout).
- **`hermes tool-calls <id>`** → satu baris per tool call:
  `<id> [<status>] <tool> args=<json> result=<text>` (semua field
  disanitasi). Sesi tanpa tool call → stdout kosong, exit 0.
- **Kebab-case:** clap merender `ToolCalls` sebagai `tool-calls`; `name()`
  mengembalikan `"tool-calls"` (pinned unit test).
- **Error + exit non-zero:** id bukan UUID → `error: invalid session id
  '<raw>' (expected a UUID)`, exit 1; id valid tapi tidak ada (atau store
  tidak ada) → `error: session not found: <uuid>`, exit 1 — kontrak yang
  sama dengan T03 `inspect`.
- **Read-only:** `open_existing_store` (T03) — tidak pernah membuat
  `state.db`, tidak menulis baris kanonik.

## Bukti
- **E2E** `tests/subcommands_e2e.rs`:
  - `messages_and_tool_calls_match_live_repl_and_write_nothing` — seed
    `state.db` (termasuk turn assistant yang membawa ANSI `\x1b[31m`),
    jalankan REPL `/messages` + `/tool-calls` pada store yang sama, lalu
    subcommand: stdout **byte-exact** (`[1] user: parity check B` /
    `[2] assistant: reply red done` — ANSI dibuang; `tc-b-1 [success]
    fixture_tool args={} result=ok`), identik dengan baris REPL, tidak ada
    prompt `❯ `, tidak ada ESC, baris seed tetap utuh.
  - `tool_calls_subcommand_parses_kebab_case`,
    `messages_subcommand_parses_and_validates`,
    `tool_calls_and_messages_unknown_id_is_clear_error`.

## Blocking edges
Bergantung T03 (`open_existing_store`, `parse_session_id`). Tidak
memblokir tiket lain.
