# 014-03: `hermes sessions` + `hermes inspect <id>`
**Status:** DONE — menunggu review Matt.

## Cakupan (Ticket 03)
Implementasi dua subcommand sesi di `crates/hermes-cli/src/subcommands.rs`
dengan **memakai fungsi data + render REPL yang sama** (`session_menu`),
sehingga shell dan REPL render identik secara struktural (satu fungsi,
satu format — dibuktikan e2e terhadap output REPL yang hidup):

- **`hermes sessions`** → `session_menu::list_sessions(&store)` (fungsi
  yang sama dengan `/sessions` REPL): semua sesi dari `SessionStore::list()`
  (urutan `started_at DESC`), satu baris per sesi
  `<id>  started=<epoch.3f>  <preview user turn pertama, disanitasi>`;
  store kosong atau file `state.db` tak ada → `No sessions.` (teks yang
  sama persis dengan REPL).
- **`hermes inspect <id>`** → `session_menu::inspect_session(&store, id)`
  (fungsi yang sama dengan `/inspect <id>` REPL): metadata satu sesi —
  `Session:` (id), `Source:` (disanitasi), `Started:` (waktu dibuat,
  format 3f), `Turns:` (jumlah turn dari tabel messages), `Tool calls:`
  (jumlah baris `tool_calls` sesi itu). Catatan: model data sesi tidak
  menyimpan `provider` per sesi (provider adalah pilihan runtime, bukan
  atribut sesi) dan schema `sessions` tidak punya kolom `updated_at` —
  metadata parity-nya adalah apa yang dicetak `/inspect` REPL.
- **Error + exit non-zero:**
  - id bukan UUID → `error: invalid session id '<raw>' (expected a UUID)`,
    exit 1;
  - id valid tapi tidak ada di store (atau store tak ada) →
    `error: session not found: <uuid>` (pesan `SessionStoreError::NotFound`
    yang sama dengan REPL), exit 1;
  - `inspect` tanpa id → error clap exit 2 (test pin T01 tetap hijau).
- **Read-only:** helper baru `open_existing_store` hanya membuka
  `state.db` bila file sudah ada — subcommand **tidak pernah membuat**
  store (pembuatan store milik REPL/TUI saat startup) dan tidak menulis
  baris state apa pun (bukti: snapshot baris kanonik
  sessions/messages/tool_calls sebelum/sesudah di e2e). Membuka store yang
  sudah ada menjalankan DDL idempoten yang sama persis dengan startup
  REPL (invariant "state.db canonical tak tersentuh" terpenuhi).
- **Dispatch & flag:** dispatch tetap **sebelum** provider resolution &
  session creation (tidak berubah sejak T02); global flags tetap posisi
  bebas (`--hermes-home` diuji setelah subcommand di e2e). `sessions`/
  `inspect` tidak memakai `--provider` (sesi tidak terikat provider).
- **Warna:** output sesi plain di REPL maupun shell (parity byte-level);
  invariant "warna hanya saat TTY / stdout piped bebas ANSI" terpenuhi
  trivial — tidak ada ANSI sama sekali, sama seperti `/sessions` &
  `/inspect` REPL.
- Placeholder `coming soon` untuk `sessions`/`inspect` dihapus dari jalur
  `run()`; `placeholder()`/`name()` tetap ada untuk T04-T06 (test pin
  10 case tidak berubah).

## Bukti
- **Unit test** (`subcommands.rs`, +2): `parse_session_id` (UUID valid
  diterima, `abc-123` ditolak dengan pesan tertahan); `open_existing_store`
  → `None` dan **tidak membuat** file `state.db`.
- **E2E** (`tests/subcommands_e2e.rs`, +6; fixture `state.db` disetel
  langsung via rusqlite, teknik yang sama dengan
  `search_credential_safety.rs`):
  - `sessions` dengan 2 sesi → stdout **byte-exact**
    `<idB>  started=1700000001.000  parity check B` +
    `<idA>  started=1700000000.000  parity check A` (urutan DESC),
    **identik dengan baris `  started=` dari stdout REPL `/sessions`**
    pada store yang sama; tanpa prompt `❯ `, tanpa ANSI; baris kanonik
    tidak berubah (snapshot sebelum/sesudah).
  - `sessions` tanpa `state.db` → `No sessions.\n`, exit 0, file
    `state.db` **tidak dibuat**.
  - `inspect <id>` → stdout byte-exact `Session:/Source:/Started:/Turns:/
    Tool calls:` **identik dengan baris metadata `/inspect <id>` REPL**;
    state tidak berubah.
  - `inspect <uuid-tak-dikenal>` (dengan store & tanpa store) → failure
    exit 1, stderr `session not found: <uuid>`; file store tidak dibuat.
  - `inspect abc-123` → failure exit 1, stderr `invalid session id
    'abc-123' (expected a UUID)`.
  - `sessions --hermes-home <dir>` (flag setelah subcommand, tanpa env
    `HERMES_HOME`) → daftar normal — posisi bebas global flag.
- **Gate byte-exact (commit ini):** `cargo fmt --all` bersih;
  `cargo check` 0 error; `cargo test --workspace --lib --bins --tests` =
  **459 passed / 0 failed** (termasuk 8 test baru T03: 2 unit + 6 e2e) —
  log VM `/tmp/t014_test.log`;
  `cargo clippy --workspace --all-targets -- -D warnings` bersih (RC=0) —
  log VM `/tmp/t014_clippy.log`.

## STRIDE
- Tidak ada surface eksekusi/network/input baru: kedua subcommand hanya
  membaca `state.db` yang sudah ada lewat `SessionStore` yang sama dengan
  REPL; tidak ada byte kanonik yang ditulis (bukti snapshot e2e).
- Output melewati jalur sanitasi yang sama dengan REPL
  (`sanitize_untrusted_output` di dalam `session_menu` — boundary CLI
  stdout). Id sesi format-terbatas (UUID); `source` dan preview konten
  user disanitasi. Redaksi credential adalah domain jalur pencarian
  (Spec 004) dan tidak berubah di T03.
- `--tui` + subcommand → subcommand menang (tanpa raw-mode, aman di
  pipa) — tidak berubah sejak T01.
