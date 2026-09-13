# T09 — Session picker (`hermes sessions browse`, `/sessions`) — Spec 017 §F

Status: DONE — menunggu review Matt (+ CI hijau sebagai gate).

## Scope (spec §F verbatim)

Port `main.py::_session_browse_picker` (L1326-1650): curses browser +
live search filter. String yang di-port verbatim (dipin di unit test):

- Hint: `  Browse sessions — ↑↓ navigate  Enter select  Type to filter  Esc quit`
  + varian filter: `  Browse sessions — filter: {search_text}█`
- Baris: `{name:<nw}  {status:<5}  {msgs:>5}  {last_active:<10}  {source:<5} {sid}`
- Header: `  {'Title / Preview':<nw}  {'Stat':<5}  {'Msgs':>5}  {'Active':<10}  {'Src':<5} {ID}`
- Footer: `  {cursor+1}/{shown} sessions` (+ ` (filtered from {total})`) + `   d delete`
- `No sessions found.` / `  No sessions match the filter.` / `Terminal too small`
- Delete: `d` (filter harus kosong) → `  Delete session '{label}'? [y/N]`
  → `Deleted.` / `Delete failed.`
- Fallback non-curses: `\n  Browse sessions  (enter number to resume, q to cancel)\n`
- **[KOREKSI] Tanpa opsi `n. New session`** — new session = jalankan
  `hermes` tanpa argumen; resume = `hermes -c` / `--resume-id`.

## Yang dikerjakan

- Baru: `crates/hermes-cli/src/session_picker.rs` — konstanta verbatim +
  fungsi format murni (`column_header`, `format_row`, `footer`,
  `hint_filter`, `delete_prompt`), `collect_rows` (sanitasi di boundary,
  nama = first user message single-line), `apply_filter` (substring
  case-insensitive atas nama/sid/source), `browse` (loop crossterm:
  alternate screen + raw mode, ↑↓/Enter/Esc/filter/`d`, Ctrl+C → 130),
  `browse_numbered` (fallback generik `BufRead`/`Write`).
- `SessionStore::delete_session` (core): hapus `tool_calls` +
  `messages` + `sessions` (children dulu — FK tanpa cascade); `false`
  bila id tak dikenal.
- `Commands::Sessions { action: Option<SessionsAction> }` +
  `SessionsAction::Browse`. Bare `sessions` = list lama (zero
  regression); `browse` = picker (TTY) / numbered fallback (piped,
  scriptable: `echo 1 | hermes sessions browse`). Seleksi mencetak
  `Selected session {id}` + hint `--resume-id` (subcommand tidak
  pernah masuk REPL — invariant Spec 014).
- REPL `/sessions` (TTY) → picker, seleksi di-resume in place
  (`Resumed {id}`); piped → list lama (byte-stable, E2E Spec 014 utuh).
- Startup REPL baru: `--resume-id <id>` (validasi + `session not
  found`), lalu `--resume`/piped → latest-or-create, lalu TTY bare →
  **selalu sesi baru** (spec §F; `select_session` lama dengan
  `n. New session` dihapus dari `session_menu.rs`).
- Bugfix samping (satu baris + komentar, di scope sesi): `store.list()`
  = newest-first, jadi `.last()` me-resume sesi TERLAMA — diganti
  `.first()` di `repl.rs` (piped/`--resume`) dan `tui/worker.rs`.
- Flag baru `--resume-id <ID>` (long-only; `-r` tetap bool `--resume`).

## Adaptasi yang didokumentasikan (di header modul)

- `sid` = 8 char pertama UUID (ID Python pendek; UUID penuh 36 char
  tak muat di picker 80 kolom).
- `status` = `done` bila ada pesan else `empty` (`intr`/`err` tak
  terlacak — kolom dipertahankan untuk paritas).
- `name` = preview pesan user pertama (belum ada session title).
- Baris cursor = reverse video (palet `_status_attr` tak ada di §F).
- `q` = char filter di mode browser (hint hanya mencantumkan `Esc
  quit`); `q to cancel` hanya di fallback numbered.
- `Active` = umur relatif (`just now`, `5m ago`, … — ≤10 sel);
  format tanggal Python tak terekam di §F.
- Ambang `Terminal too small` = 60×8 (ambang Python tak terekam).

## Tes

- Unit picker (11): bentuk verbatim hint/header/row/footer/delete,
  truncasi nama, `name_width`, `relative_active`, filter, `frame_lines`,
  `collect_rows` (sanitasi ANSI + newline-folding, sesi kosong),
  fallback numbered (pilih/batal/garbage/empty-store).
- Core: `delete_session_removes_session_messages_and_tool_calls`.
- Piped E2E (`subcommands_e2e.rs`, +4): fallback pilih/batal,
  empty-store, `sessions --help`, `--resume-id` (+ unknown/malformed).
- PTY E2E (`session_picker_e2e.rs`, 7): frame verbatim + Enter,
  ↓ baris kedua, filter live + footer count, `d`+`y` (ground truth
  rusqlite: sibling utuh), Esc batal, empty store, `/sessions` REPL
  resume in place.

## Keputusan untuk review Matt

1. Startup TTY bare → sesi baru (menghapus startup picker lama).
   Alternatif: startup picker tetap ada tanpa opsi new (pengguna
   terjebak bila ingin sesi baru selain via `/new`).
2. TUI (`--tui`) tetap resume-latest (hanya bugfix last→first);
   redesign startup TUI + picker TUI = tiket lanjutan.
3. Resume display §H (`↻ Resumed session …` + recap) BELUM di-port
   (`Resumed {id}` dipertahankan) — usulan tiket lanjutan.
4. `sessions list/stats/prune/export/rename/delete` (tips.py) BELUM
   di-port — hanya `browse`; klaim sesuai implementasi.
