# 014-02: `hermes model`
**Status:** DONE — menunggu review Matt.

## Cakupan (Ticket 02)
Implementasi subcommand `hermes model`: daftar provider terkonfigurasi +
model-nya, dengan penanda provider aktif. Modul baru
`crates/hermes-cli/src/subcommands.rs` (dispatch + helper bersama untuk
T03-T07).

- **Dispatch:** `main.rs` → `subcommands::run(cmd, args)` **sebelum** provider
  resolution & session creation; `run()` memuat home/config lewat
  `subcommands::load_home_config` (semantik identik dengan path REPL:
  `config.yaml` hilang = `None`, slice offline `fake` tetap usable).
- **Output** (pinned, ANSI-free saat piped):
  ```
  Providers:
    * anthropic (active)
      models: claude-opus-4-1, claude-sonnet-4-5
      openai (OpenAI)
      models: gpt-4o
  ```
  - Nama provider diurutkan; model diurutkan & digabung koma;
    model kosong → `models: (not configured)`.
  - Tanpa config → `* fake (active, built-in)`.
  - `--provider <name>` memfilter satu provider; nama tak dikenal →
    error jelas `unknown provider 'x' (configured: a, b)` + exit 1.
  - Precedence aktif: flag `--provider` > `model.provider` (≠ `auto`) > `fake`.
- **Warna (hanya TTY):** provider = bold gold `#FFD700`
  (`sgr_bold_gold`), model = banner text `#FFF8DC` (`sgr_banner_text`),
  label/marker = dim brown `#B8860B` (helper baru `sgr_dim_brown`,
  `crates/hermes-cli/src/tui/welcome.rs`). Piped stdout tetap tanpa ANSI
  (invariant sama dengan banner/status bar Spec 013).
- **Read-only:** tidak membuka `state.db`, tidak menulis apa pun.
- Helper `name()`/`placeholder()` dipindah dari `main.rs` ke modul ini
  (placeholder T01 untuk T03-T06 tetap ada, test pin ikut pindah).

## Bukti
- **Unit test** (`subcommands.rs`, 9 test): daftar terurut + marker aktif;
  filter; error filter tak dikenal; tanpa config → fake; filter `fake`;
  provider tanpa model; SGR tema saat colored (gold/banner/dim-brown/reset);
  precedence `active_provider`; pin nama + pesan placeholder (10 case).
- **E2E** (`tests/subcommands_e2e.rs`, 3 test baru menggantikan 1 test
  placeholder): `model` dengan fixture config.yaml 2 provider → marker aktif
  + daftar model + **tanpa ANSI saat piped** + `state.db` tidak dibuat;
  `model --provider openai` → hanya provider itu, `anthropic` tak muncul;
  tanpa config → `fake (active, built-in)`. Test `global_flags_parse_after_
  subcommand` kini mengharapkan output model nyata.
- **Gate byte-exact (commit ini):** `cargo test --workspace --lib --bins
  --tests` = **422/422** (23 target, 0 gagal; baseline T01 412 + 10 baru);
  `cargo clippy --workspace --all-targets -- -D warnings` bersih.
- Note: `serde_yaml = "0.9"` ditambahkan ke **dev-dependencies** hermes-cli
  (fixture test membangun `ProviderConfig.models`); tidak menambah dependensi
  runtime.
