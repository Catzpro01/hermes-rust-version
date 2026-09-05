# 012-01: TUI event model + dependency + CLI flag opt-in
**Status:** DONE (commit pending, review Matt).
**What to build:** Fondasi arsitektur TUI: model event yang menghubungkan worker
agentic dengan renderer, penambahan dep `ratatui`/`crossterm`, dan flag `--tui`
di `main.rs` yang mengaktifkan jalur TUI tanpa mengubah jalur readline default.

## Cakupan final (Ticket 01)
- `crates/hermes-cli/src/tui/mod.rs`: enum `TuiEvent` + constructor
  `sanitized_chunk`/`tool_started` yang memakai `sanitize_untrusted_output`
  (output.rs) dan `redact_credentials` (hermes-core::search::redact) — payload
  DIBERSIHKAN di sumber sebelum masuk channel, renderer tidak menyaring lagi.
  Helper `redact`; `run_tui()` stub untuk Ticket 02+.
- Dep hanya di `crates/hermes-cli/Cargo.toml`: `ratatui = "0.29"`,
  `crossterm = "0.28"`. `hermes-core` tetap bebas UI.
- `main.rs`: arg `--tui` (clap). Gate stdin interaktif: jika `--tui` dan stdin
  bukan TTY (piped) → error `--tui requires an interactive terminal`
  (Warning A Matt). Tanpa `--tui` jalur readline identik (regresi nol).
- E2E baru `cli_e2e.rs`: `--tui` + stdin pipa → failure berisi pesan TTY.
  Unit test tui: stripping ANSI/C0, redaksi + truncate, konstruksi semua varian.

## Kriteria (hasil verifikasi)
- [x] `TuiEvent` mencakup semua data dashboard yang dibutuhkan, payload ter-redaksi.
- [x] `--tui` di-parse; tanpa flag default REPL readline (regresi nol).
- [x] Dep hanya di cli; build hermes-core tidak berubah.
- [x] Sanitasi boundary dijelaskan: event membawa teks sudah bersih.
- [x] Unit test murni; clippy bersih.
- [x] Gate TTY stdin interaktif → error jelas utk piped stdin (Warning A).
- [x] Desain bounded channel drop-oldest utk worker→renderer (Ticket 02+; Warning B).

## Bukti verifikasi (VM)
`cargo test --workspace` = **285/285 green** (281 baseline + 4 baru).
`cargo clippy --workspace --all-targets -- -D warnings` bersih.
`cargo check` fetch network ratatui/crossterm sukses.
