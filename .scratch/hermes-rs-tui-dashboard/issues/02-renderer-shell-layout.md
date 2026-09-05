# 012-02: Renderer shell — Ratatui loop + multi-panel layout + crossterm events
**Status:** DONE (commit pending, review Matt).

## Cakupan final (Ticket 02)
- `tui/mod.rs` — `run_tui()` nyata (bukan stub): spawn worker di tokio task,
  renderer jalan di `spawn_blocking` (tidak memblokir worker), `RawGuard`
  (Drop guard) menjamin restore terminal di SEMUA jalur keluar (q, Ctrl-C,
  error, unwind/panic). `q` → exit 0; Ctrl-C → exit 130 (via err "interrupted").
- `tui/event.rs` — enum `TuiEvent` (dipindah dari mod.rs Ticket 01).
- `tui/channel.rs` — `EventQueue` bounded (cap 256) + **drop-oldest** (Warning B,
  producer tidak pernah block); `TuiCommand` renderer→worker (unbounded, tak
  lossy). Unit test drop-oldest & bound.
- `tui/layout.rs` — fungsi `split` PURE & manual-saturating (total, tak panic di
  ukuran mana pun), membagi header/transcript/tool_log/input. Unit test ukuran
  tipis/lebar/tiny/offset: panel selalu di dalam area, tanpa overlap.
- `tui/app.rs` — `App` state display (status/session/provider/token/iteration,
  transcript & tool_log bounded), `apply(TuiEvent)`, `handle_key` (q, Ctrl-C,
  Enter→Submit, Backspace, Esc), `render` memakai ratatui. Render headless
  via `TestBackend` di ukuran besar/sedang/tipis/tiny.
- `tui/worker.rs` — worker DEMO (berlabel; data agentic nyata Tickets 03/04)
  menstream event mewakili satu turn agar shell + bounded-queue + keyboard
  teruji end-to-end di terminal hidup.
- Keyboard → `TuiCommand` → worker channel (kriteria #6); worker di tokio task
  terpisah (#9); bounded channel (#8). Terminal-restore invariant via Drop guard
  (catatan penting Matt).

## Kriteria (hasil verifikasi)
- [x] Layout murni unit-testable: panel benar di ukuran tipis/lebar (tak panic).
- [x] Raw mode masuk/keluar bersih via `RawGuard` (Drop, termasuk panic/unwind).
- [x] Input keyboard (`q`/Ctrl-C quit; Enter submit → worker) diproses & dikirim.
- [x] Siklus tick terbatas (`poll` 50ms), bukan busy-loop tak terkendali.
- [x] Unit tests layout/channel/app + clippy bersih. (Integrasi visual penuh tiket 05.)

## Bukti verifikasi (VM)
`cargo test --workspace` = **300/300 green** (sebelumnya 285 + 15 test baru tui).
`cargo clippy --workspace --all-targets -- -D warnings` bersih.
`cargo check` sukses (ratatui 0.29 + crossterm 0.28; API terverifikasi via source).
