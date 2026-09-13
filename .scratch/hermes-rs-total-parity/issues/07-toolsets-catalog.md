# T07 — Toolsets catalog (26) / `hermes tools` (Spec 017)

Status: DONE — MERGED (PR #2, `319d719`)

- Katalog `CONFIGURABLE_TOOLSETS` (26) + `_DEFAULT_OFF_TOOLSETS` (8)
  verbatim di `wizard/catalog.rs`.
- Subcommand baru `hermes tools`: TTY → checklist multiselect wizard
  (section `Tools`, pre-check dari `tools.enabled_toolsets` atau katalog −
  default-off), tulis atomik + backup; piped → listing plain
  `  [x] key  label — tools` (ANSI-free, tanpa tulis).
- Schema: `HermesConfig.tools: Option<ToolsConfig{enabled_toolsets}>`
  (absent → default; registry tool Rust **belum** membacanya — parity
  config/display saja; wiring ke registry = tiket lanjutan).
- Tes: unit `tools_listing_follows_defaults_then_config`; E2E piped
  `tools_piped_lists_catalog_without_writing`, PTY
  `tools_tty_checklist_writes_enabled_toolsets`.
