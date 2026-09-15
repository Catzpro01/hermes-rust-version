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
  Crate version: 0.21.0
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

## Penyelarasan versi 0.1.0 → 0.21.0 (2026-09-16)
Baris 1 (`VERSION_LABEL`) sudah `Hermes-RS v0.21.0 …` sejak Ticket 07,
tetapi `Crate version:` di bawahnya mencetak 0.1.0 — dua angka berbeda
dalam satu layar. Disamakan ke 0.21.0 di lima tempat:

- `Cargo.toml` — `workspace.package.version` (satu-satunya sumber; kedua
  crate mewarisinya).
- `Cargo.lock` — kedua member workspace menyimpan versinya di lock, dan
  `cargo check --workspace --locked` gagal keras bila lock tidak cocok.
  Hanya dua field `version` yang berubah (member path tanpa `source`/
  `checksum`).
- `crates/hermes-cli/tests/subcommands_e2e.rs` — asersi mematok string lama.
- `crates/hermes-core/src/mcp/client.rs` — `CLIENT_VERSION` = versi yang
  dikirim sebagai `clientInfo` pada handshake MCP `initialize`; ikut
  dinaikkan atas instruksi Matt. `MCP_PROTOCOL_VERSION` tidak tersentuh;
  satu-satunya tes handshake mengasersi `clientInfo.name`, bukan versi.
- Tiket ini sendiri (blok spesifikasi keluaran di atas).

Dibiarkan: log `Checking hermes-rs v0.1.0` di
`docs/hermes-ui-spec/017/evidence/` = bukti yang dipertahankan; catatan
parity `hermes-rs 0.1.0` di `.scratch/hermes-rs-ui-parity/issues/06-parity-docs-closure.md`
= perbandingan historis; `nibble_vec` 0.1.0 di `Cargo.lock` = crate pihak
ketiga.

## Verifikasi
- **VPS bare-metal, dijalankan Matt (bukan agen):**
  `make check` → `Finished dev profile [unoptimized + debuginfo] target(s)
  in 2m 03s`. Ini sekaligus membuktikan suntingan tangan `Cargo.lock`
  tepat: dengan `--locked`, lock yang meleset menggagalkan perintahnya.
- **Belum dijalankan:** `make test` (`cargo test --workspace --no-fail-fast`,
  yang menjalankan asersi `Crate version: 0.21.0`) dan `make build` +
  `./target/debug/hermes-rs version` untuk melihat keluarannya langsung.
- **Tidak pernah diklaim:** tidak ada `cargo` di sandbox agen, jadi nol
  verifikasi lokal. Job GitHub `CI gate regression tests` → pass, tetapi
  job itu hanya skrip Python dan tidak menyentuh cargo; `fmt + clippy +
  test` (pemilik `cargo check --locked`) berstatus `if: false`.

### Koreksi 2026-09-16 — `make check` gagal di shell ber-rustc 1.85.1
Di shell `fern@master`, `make check` dan `make test` keduanya berhenti
dengan `error: rustc 1.85.1 is not supported by the following packages`:
`darling` 0.24.1, `home` 0.5.12, `icu_*` 2.3.x, `icu_provider` 2.3.1,
`idna_adapter` 1.2.2 (≥1.86), `instability` 0.3.13, `time` 0.3.55
(≥1.88). **Bukan akibat bump 0.1.0 → 0.21.0.** Bukti:
- `git diff e50fdf5 HEAD -- Cargo.lock` = **dua baris saja** (field
  `version` milik `hermes-core` dan `hermes-rs`);
- `time` sudah di 0.3.55 pada `8ad14a7` — sebelum karya sesi ini — jadi
  `main` akan gagal persis sama di toolchain ini;
- workspace tidak mendeklarasikan `rust-version`, dan `ci.yml` tidak
  memin toolchain: yang dipakai adalah apa pun yang menang di PATH shell.

Karena itu hasil "2m 03s" di atas **belum teratribusi** — ia hanya sah
sebagai bukti bila dijalankan dengan toolchain ≥1.88 (stabil 1.98.1) dan
sudah memuat `48a12b8`. Verifikasi ulang yang menentukan:
`rustc --version` → lalu `RUSTUP_TOOLCHAIN=stable make check && make test`.
