# 012-04: Tool log + streaming transcript + input editor + redaksi
**Status:** DONE (commit pending, review Matt).

## Cakupan final (Ticket 04)
- **Streaming transcript**: `App` mengumpulkan `TuiEvent::Chunk` ke buffer
  `streaming` (live), tool round (ToolStarted) membuang teks transien scaffolding,
  dan `Done` mem-finalize SATU pesan assistant otoritatif (hindari duplikasi
  fragment). `agent_event_to_tui` kini memetakan `AgentEvent::Done` → `TuiEvent::Done`
  (bukan Chunk), sesuai semantik.
- **Scroll transcript** (PgUp/PgDn): offset terpisah; 0 = ikuti bottom (default),
  PgUp naik, PgDn turun (clamp ke bottom). Ditambah live-tail streaming.
- **Tool log**: ToolStarted `▶ name args(ringkas+redaksi 120)` & ToolDone `✓ name
  (status)`; bottom-anchored tail.
- **Replay tool-log dari state.db**: `run_loop` memanggil `store.list_tool_calls`
  saat resume session → notif jumlah + baris ToolDone historis sebelum Notice live.
- **Input editor single-line**: buffer `Vec<char>` + cursor (multi-byte aman) —
  insert/Backspace/Left/Right/Home/End, Enter submit, Esc clear.
- **Input history** (↑/↓, cap 20, dedupe consecutive): history_back/forward.
- **Sanitasi semua panel + test end-to-end**: `injected_credential_never_reaches_the_panel`
  — secret di AgentEvent core → map (redaksi) → apply → render TestBackend → secret
  TIDAK ada di buffer terminal.
- Tool log: `AgentEvent::ToolStarted`/`ToolDone` → panel (lewat mapping + App).

## Kriteria (hasil verifikasi)
- [x] Transcript: streaming chunk live dari `AgentEvent::Chunk`.
- [x] Scroll transcript (PgUp/PgDn).
- [x] Tool log: name/status/args ringkas + redaksi.
- [x] Replay tool-log dari state.db saat session resume.
- [x] Input bar single-line + Enter.
- [x] Input history arrow ↑/↓ (>=10; kita cap 20).
- [x] Sanitasi di semua panel (test inject credential → tidak tampil).
- [x] ToolStarted/ToolDone → tool log.
- [x] 312/312 green (+6), clippy bersih.

## Catatan transparan (tidak blocker)
- Tool log scroll bottom-anchored (transcript scroll penuh PgUp/PgDn; acceptance
  #2 transcript). Multiline paste & AgentEvent::Cancelled/PlanUpdated = nice-to-have
  lintas tiket.
- Uji manual `--tui` (live streaming, scroll, history di terminal nyata) butuh TTY;
  sandbox headless: semua unit/rendering hijau. E2E TestBackend penuh di Ticket 05.

## Bukti verifikasi (VM)
`cargo test --workspace` = **312/312 green** (306 + 6).
`cargo clippy --workspace --all-targets -- -D warnings` bersih.
