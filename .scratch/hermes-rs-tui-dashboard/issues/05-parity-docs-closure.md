# 012-05: Parity, docs & closure — Spec 012 (tiket terakhir Phase 5)
**Status:** DONE (commit pending, review Matt final).

## Ringkasan
Closure Spec 012 + seluruh pengerjaan TUI (T01–T05). Tambahan kuat di tiket ini:
**E2E headless** (`TestBackend`) menggantikan uji manual TTY (sandbox tak punya TTY),
dan penguatan boundary: streaming text (Chunk/Done/Error/Notice) kini **di-redaksi
juga** (`worker::scrub` = sanitize ANSI + redact creds), bukan hanya arg tool.

## Yang diselesaikan
- **`tui/e2e.rs`** (modul test): E2E headless via `TestBackend` —
  1) full-session sim (token meter `500/1000`, goal/plan/reflection, tool log
  `read_file`, transcript final answer, input placeholder) → semua panel assert;
  2) streaming chunk tampil live sebelum Done (buffer streaming, 0 msg final);
  3) **sanitasi end-to-end** — inject credential + ANSI di `AgentEvent` core →
     render → assert secret & `\x1b` TIDAK ada di buffer terminal;
  4) **Ctrl-C** → `QuitInterrupt` + `should_quit` (main map "interrupted" → 130;
     restore via RawGuard Drop guard).
- Boundary diperkuat: `agent_event_to_tui` scrubs Chunk/Done/Error/Notice
  (sanitize+redact) selain ToolStarted; `sanitized_chunk` kini helper allow-dead
  (diganti `scrub`).
- **Docs**: `PARITY.md` (bagian Spec 012 TUI — Python Hermes tak punya dashboard;
  row differences TUI), `ROADMAP.md` (baris 012 → Done; closure section; bukti
  316 green), `SECURITY.md` (boundary sanitasi TUI, RawGuard terminal-restore,
  deny-by-default confirmation, SessionStore non-Send tanpa unsafe).

## Kriteria (hasil verifikasi)
- [x] E2E headless TestBackend: session penuh → assert panel (token, tool, transcript, input).
- [x] E2E sanitasi: inject credential + ANSI → assert tidak tampil.
- [x] E2E SIGINT: Ctrl-C → QuitInterrupt / exit-130 mapping (headless; RawGuard restore).
- [x] --tui aktif → TUI; tanpa flag → REPL identik (regresi nol; smoke tetap hijau).
- [x] docs/PARITY.md Spec 012 (TUI; Python Hermes tak punya).
- [x] docs/ROADMAP.md Spec 012 → Done; catatan jujur Phase 5 (011+012 Done; Spec 010/007 Not started).
- [x] docs/SECURITY.md TUI sanitization boundary.
- [x] smoke_python_hermes_untouched tetap lulus (0 failure).
- [x] 316/316 tests green, clippy bersih.
- [x] Known limitations didokumentasikan (multiline paste, tool-log scroll bottom, Cancelled/PlanUpdated ditunda).

## Bukti verifikasi (VM)
`cargo test --workspace` = **316/316 green**, 0 failure.
`cargo clippy --workspace --all-targets -- -D warnings` bersih.

## Catatan transparan utk review final
Phase 5 sebenarnya = Spec 010 (Not started), 011 (Done), 012 (Done). ROADMAP saya
perbarui jujur: tidak menulis "100% DONE" buta karena Spec 010 plugin/WASM masih
Not started (juga Spec 007 sandbox). Mohon Matt konfirmasi apakah Spec 010 di-descope
dari milestone agar label 100% akurat.
