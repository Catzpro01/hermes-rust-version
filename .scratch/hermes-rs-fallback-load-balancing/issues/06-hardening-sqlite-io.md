# 06: Hardening IO — flaky SQLite concurrent test

**What to build:** Selesaikan tech-debt yang dicatat sejak review Ticket 01 dan
diulang di review Spec 005: test `concurrent_writes_to_same_sqlite_session_are_serialized`
bersifat intermittent di bawah beban CPU tinggi.

**Blocked by:** — (independen, bisa paralel kapan saja).

**Status:** done — commit di VM, 165/165 test hijau (`cargo test --workspace`),
`clippy --workspace --all-targets -D warnings` bersih. Test konkurrensi stabil
3× full-suite + 12× berurutan.

## Root cause (didiagnosis)

Sebelum perbaikan, `save_turn` memakai `self.conn.transaction()` (rusqlite
default = **`BEGIN DEFERRED`**), dan `busy_timeout` 5 dtk di `open()`.

Dalam mode rollback-journal (default), dua koneksi terpisah yang masing-masing
mulai transaksi DEFERRED bisa jatuh ke deadlock lock-ordering klasik:

- Koneksi A `BEGIN` (belum pegang lock), lalu `INSERT` → naik ke lock **RESERVED**.
- Koneksi B juga `BEGIN DEFERRED` (mengambil lock **SHARED** baca) sebelum mencoba
  `INSERT`; untuk menulis ia perlu upgrade SHARED→RESERVED, tapi A memegang RESERVED.
- A untuk `COMMIT` perlu naik RESERVED→**EXCLUSIVE**, yang menuntut tak ada reader
  — padahal B masih memegang SHARED. Maka A tak bisa commit, B tak bisa menulis.

SQLite memecah kebuntuan ini dengan mengembalikan `SQLITE_BUSY` yang **tidak bisa
dituntaskan `busy_timeout`** (bukan busy sementara biasa). Di bawah beban CPU
tinggi, timing-nya kadang memicu, sehingga test intermittent. Workaround lama
(helper test `save_turn_retry_busy` + `thread::sleep` dari Ticket 02) hanya
menambal gejala dan menambah latensi.

## Perbaikan (production code)

`SessionStore::save_turn` kini membuka transaksi dengan **`BEGIN IMMEDIATE`**
(`transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)`): lock
tulis diambil **di awal**, bukan ditunda ke `INSERT` pertama. Dengan begitu tidak
ada koneksi yang memegang lock baca SHARED sambil berniat menulis, sehingga
deadlock upgrade-deferred hilang total dan `busy_timeout` membersihkan serialisasi
dua writer. Tidak ada semantik berubah: tetap satu INSERT + commit per turn,
tabel/foreign keys sama. Tidak memakai WAL (menghindari perubahan header file
fixture yang di-commit ke repo) dan tidak menambah dependency.

Test dibersihkan: helper `save_turn_retry_busy`/`is_transient_busy` (workaround)
dihapus; kedua worker kini memanggil **`store.save_turn` langsung** (jalur
produksi), membuktikan produksi menangani konkurensi sendiri. Assert `20 turns`
final tidak berubah.

## Kriteria

- [x] Root-cause flake tersisa didiagnosis (deadlock DEFERRED-upgrade di
      rollback-journal; `busy_timeout` tak bisa menuntaskannya).
- [x] Perbaikan di **production** (`SessionStore::save_turn` → `BEGIN IMMEDIATE`),
      bukan workaround test.
- [x] Semantik produksi tidak berubah (tetap satu INSERT+commit; schema sama).
- [x] Test stabil berulang: 3× full-suite + 12× berurutan `concurrent_writes...`.
- [x] Tidak melemahkan assert (jumlah turn akhir tetap 20).
- [x] Tidak menambah dependency baru.

## STRIDE

- Tidak ada surface credential/eksekusi baru; murni mengubah strategi transaksi
  SQLite agar konsisten dengan `busy_timeout`.
- Read-only inspection test tetap hijau (`inspection_queries_are_read_only...`),
  memastikan tak ada penulisan kanonik dari query baca.

## Pengujian

- `cargo test --workspace` → **165/165** hijau (berulang 3× full-suite).
- `concurrent_writes_to_same_sqlite_session_are_serialized` → ok 12× berurutan.
- `inspection_queries_are_read_only_and_isolate_sessions`, `search::*` semua hijau.
- `cargo clippy --workspace --all-targets -D warnings` → bersih.

## Perubahan

- `crates/hermes-core/src/session/store.rs`: `save_turn` kini
  `transaction_with_behavior(TransactionBehavior::Immediate)` + komentar akar
  masalah.
- `crates/hermes-core/tests/sqlite_parity_integration.rs`: hapus helper retry
  busy test-level; worker memakai `save_turn` langsung.

## Catatan desain

- `busy_timeout` 5 dtk tetap dipertahankan; dengan `BEGIN IMMEDIATE` ia kini
  efektif karena kontensi tersisa hanyalah busy-sementara antar dua writer.
- Menghindari WAL sengaja: `PRAGMA journal_mode=WAL` bersifat persisten dan akan
  menulis ulang header DB fixture `tests/fixtures/hermes_state.db` yang di-commit
  ke repo — tidak diinginkan. Jika konkurensi read-heavy dibutuhkan nanti, WAL
  bisa jadi keputusan terpisah.

## Dependency

— (independen).
