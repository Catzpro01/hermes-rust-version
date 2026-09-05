# T01 — `inquire` + Wizard Skeleton

**Status:** SELESAI — gate hijau, commit dibuat, menunggu review Matt.

## Ruang lingkup (sesuai approval Fase 0)

- Dependensi `inquire = "0.7"` (backend crossterm) di `hermes-rs`.
- `crates/hermes-cli/src/wizard/mod.rs` — skeleton 3 langkah:
  1. `confirm("Would you like to see what can be imported?", default: no)`
  2. `select("How would you like to set up Hermes?",
     [Quick Setup (Nous Portal), Full Setup, Blank])` (default: index 0)
  3. `multiselect("Select sections to configure:",
     [Model & Provider, Terminal Backend, Messaging Platforms, Tools])`
- String verbatim Python v0.21.0 dipin di unit test (T05 melanjutkan ke
  section sebenarnya; backend tetap **Local+Docker** dengan label
  "not wired yet" verbatim saat diimplementasikan di T05).
- Helper `confirm/select/multiselect/text_input` + `is_interactive()` +
  `WizardError{NotTty, Canceled, Interrupted, Other}` (`Display` =
  `setup wizard {msg}`).
- Entry point: flag tersembunyi `--setup-skeleton` (dispatch setelah
  subcommands, sebelum gate TUI). T05 menggantikan dengan subcommand
  `hermes setup`.
- Perilaku: ESC di level mana pun → rollback total, cetak
  `Setup cancelled.`, exit 0. Ctrl+C → `WizardError::Interrupted`
  → exit 130 (invariant SIGINT). Non-TTY → error jelas
  ("requires an interactive terminal"), exit 1 (invariant 8).
  Wizard tidak menulis apa pun: `HERMES_HOME` tetap kosong,
  tidak ada `state.db`.

## Desain E2E (putusan penting)

E2E pty di `tests/wizard_e2e.rs`: child di-spawn dengan stdin+stdout
= pty slave, stderr = pipe (inquire/crossterm merender prompt ke
**stderr**; summary `println!` wizard ke stdout pty).

- Versi awal memakai loop `nix::poll` di thread utama: hang — loop
  spin ~1,1 juta poll/3 detik dengan `n=0` padahal data ada di master
  (terbukti via harness Python: child merender prompt ke stderr di
  t=28 ms lalu menunggu normal di event-loop-nya sendiri; probe
  standalone membuktikan `nix::poll` pada master pty sendiri bekerja
  normal, termasuk timeout 200 ms). Akar spin tak pernah diidentifikasi.
- **Solusi akhir:** dua thread reader background (blocking `read` pada
  master pty + pipe stderr) yang memompa output ke `Mutex<Vec<u8>>`
  bersama; thread utama cek buffer tiap 50 ms dan mengirim key ke
  master. Pola ini identik dengan harness Python yang terbukti bekerja,
  dan menghilangkan seluruh kelas bug "poll spin". Hasil: 3/3 E2E
  lulus dalam ~0,2 s.
- Early-exit: `wait_for` juga gagal cepat jika child sudah keluar
  (`try_wait`) agar kegagalan terlihat sebagai output penuh, bukan
  timeout 15 s.

## Hasil gate

- Unit (mod `wizard`): 4/4 lulus.
- E2E `wizard_e2e`: 3/3 lulus
  (non-TTY exit 1 + error jelas; happy path `\r\r \r` → summary
  lengkap; ESC di step 2 → `Setup cancelled.` exit 0).
- `cargo test --workspace --lib --bins --tests`: 429/429 lulus (24 binary; 422 baseline + 7 T01), TEST_RC=0.
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 error/warning, CLIPPY_RC=0.
- fmt: file T01 clean (`cargo fmt -p hermes-rs -- --check` tidak
  menyentuh `main.rs`, `wizard/mod.rs`, `wizard_e2e.rs`). Catatan: HEAD
  repo punya drift fmt pre-existing di ~25 file lama (repl.rs, tui/*,
  test lama) tanpa konfigurasi rustfmt di repo — di luar cakupan T01.

## File

- `crates/hermes-cli/src/wizard/mod.rs` (baru)
- `crates/hermes-cli/tests/wizard_e2e.rs` (baru)
- `crates/hermes-cli/src/main.rs` (flag + dispatch)
- `crates/hermes-cli/Cargo.toml` (inquire; nix dev-dep +fs,poll,term)
- `Cargo.lock`
