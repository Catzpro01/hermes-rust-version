# 013-02: Color Palette & Theme System
**Status:** DONE (commit `7608316`).

## Cakupan
- `crates/hermes-cli/src/tui/theme.rs`: `HermesTheme` (29 field `Color::Rgb`
  bernama sesuai kunci skin Python), konstruktor `dark_canonical()`,
  `hex_to_rgb()`, 30+ style helper (`banner_*`, `response_*`, `status_bar_*`,
  clarify/sudo/approval modal styles), `ColorDepth` + `detect_color_depth()`
  (COLORTERM/TERM), `truecolor_to_256()` (cube + grayscale ramp).
- Scope **helpers only** — panel mulai mengonsumsi dari Ticket 03 (module
  diberi `#![allow(dead_code)]` dengan alasan eksplisit, preseden `worker.rs`).
