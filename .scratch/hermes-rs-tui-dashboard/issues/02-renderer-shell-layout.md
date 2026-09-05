# 012-02: Renderer shell — Ratatui loop + multi-panel layout + crossterm events

**Status:** breakdown.

**What to build:** Siklus renderer Ratatui: masuk raw mode (crossterm), gambarkan
layout multi-panel tiap frame, terima event keyboard (termasuk `/` command &
quit), dan keluar rapi (restore terminal) pada Ctrl-C/q.

## Desain

- `tui/mod.rs` — `run_tui(...)`; `tui/app.rs` — `App` state machine & tick;
  `tui/layout.rs` — fungsi layout murni (unit-testable) membagi rect jadi panel
  (header/status, transcript, tool log, input).
- Renderer thread/loop memakai `ratatui::Terminal` + crossterm event poll.
- Setiap `TuiEvent` dari channel diperbarui ke state `App`; tiap frame di-redraw.
- Keluar: `q`, Ctrl-C → restore terminal (disable raw mode, show cursor),
  exit code konsisten (130 utk SIGINT bila dipicu di worker).
- Worker agentic berjalan di `tokio` task terpisah; renderer tidak memblokir I/O
  agentic.

## Kriteria

- [ ] Layout murni unit-testable: komposisi panel benar pada ukuran terminal
      tipis/lebar (tidak panic, proporsional).
- [ ] Raw mode masuk/keluar bersih (test keluar memastikan terminal direstore).
- [ ] Input keyboard (`/goal`, dsb. minimal; `q`/Ctrl-C quit) diproses & dikirim
      ke worker.
- [ ] Siklus tick tidak busy-loop tak terkendali.
- [ ] Unit tests layout + clippy bersih. (Integrasi visual penuh di tiket 05.)
