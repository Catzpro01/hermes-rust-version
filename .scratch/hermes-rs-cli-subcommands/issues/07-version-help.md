# 014-07: `--version` / `hermes version` + `--help` parity
**Status:** DONE — menunggu review Matt.

## Cakupan (Ticket 07)
- **Label versi.** Python `--version` mencetak `Hermes Agent v0.21.0
  (2026.8.31) · upstream 63279301` lalu fakta instalasi
  (`docs/HERMES_UI_SPEC.md` §4), **bukan** `hermes v…`. Sebelumnya clap
  merender `hermes-rs 0.21.0-rs`. Sekarang `disable_version_flag = true`
  dan flag `-V/--version` (global, posisi bebas) + subcommand `hermes
  version` keduanya memanggil `subcommands::render_version`:
  ```
  Hermes-RS v0.21.0 (2026.8.31) · upstream 63279301
  Install directory: <dir binary>
  Install method: cargo
  Crate version: 0.1.0
  Hermes home: <home>            (hanya bila home resolve)
  ```
  Baris 1 = `VERSION_LABEL` yang sama dengan judul panel banner (satu
  konstanta, satu sumber). Brand `Hermes-RS` mengikuti keputusan T02 Spec
  013 (intentional). Fakta Python yang tidak ada di build cargo (`Python:`,
  `OpenAI SDK:`, `Update available:`) tidak dipalsukan.
- **Static & aman:** `version` di-dispatch **sebelum** `load_home_config`
  — bekerja tanpa home, dan dengan `config.yaml` rusak; tidak membuat
  direktori/file, tidak ada provider/network.
- **`--help`:** memuat semua subcommand (`setup, version, model, sessions,
  inspect, messages, tool-calls, search, info, mcp, help`) dan flag global
  (`--hermes-home`, `--provider`, `--api-url`, `--tui`, `--version`,
  `--help`); `--setup-skeleton` tetap tersembunyi; `hermes mcp --help`
  memuat `list`/`restart`. Struktur argparse Python (~90 subcommand)
  **tidak** direplikasi — hanya subset yang benar-benar diimplementasi
  (kebijakan PARITY: tidak mengklaim fitur yang belum ada).

## Bukti
- **Unit:** `main.rs::version_flag_and_subcommand_parse`,
  `subcommands.rs::version_renders_label_and_never_touches_home`.
- **E2E:** `version_flag_and_subcommand_render_the_banner_label`
  (`--version`, `-V`, `version`, `info --version` → output identik, tanpa
  ESC, tanpa store), `version_works_without_a_hermes_home_or_with_a_broken_config`
  (home tidak ada → exit 0; config rusak → `version` exit 0 sementara
  `info` gagal `Invalid config`), `help_lists_every_subcommand_and_global_flags`.
