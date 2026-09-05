# 012-01: TUI event model + dependency + CLI flag opt-in

**Status:** breakdown.

**What to build:** Fondasi arsitektur TUI: model event yang menghubungkan worker
agentic dengan renderer, penambahan dep `ratatui`/`crossterm`, dan flag `--tui`
di `main.rs` yang mengaktifkan jalur TUI tanpa mengubah jalur readline default.

## Desain

- `TuiEvent` (enum di layer CLI, mis. `crates/hermes-cli/src/tui/event.rs`):
  `StatusChanged`, `TokenTick { estimate, limit }`, `Chunk { text }`,
  `ToolStarted { name, arguments }`, `ToolDone { name, status }`, `Iteration { n }`,
  `Done`, `Error { message }`, `SessionInfo`. Semua payload membawa teks yang
  SUDAH di-redaksi/di-sanitasi di sumber, bukan di renderer.
- Dep ditambah hanya di `crates/hermes-cli/Cargo.toml`: `ratatui`, `crossterm`.
  `hermes-core` tetap bebas UI.
- `main.rs`: arg `--tui` → pilih antara `repl::run_repl` (default) dan
  `tui::run_tui(...)`. Zero regression: tanpa `--tui` perilaku identik.

## Kriteria

- [ ] `TuiEvent` mencakup semua data dashboard yang dibutuhkan (token, tool,
      status provider/goal/plan, session, error), payload ter-redaksi.
- [ ] `--tui` di-parse; tanpa flag default REPL readline (regresi nol).
- [ ] Dep hanya di cli; build hermes-core tidak berubah.
- [ ] Sanitasi boundary dijelaskan: event membawa teks sudah bersih.
- [ ] Unit test: enum + rendering redaksi helper (bila ada) murni; clippy bersih.
