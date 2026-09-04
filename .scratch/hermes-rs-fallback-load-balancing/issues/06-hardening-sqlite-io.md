# 06: Hardening IO — flaky SQLite concurrent test

**What to build:** Selesaikan tech-debt yang dicatat sejak review Ticket 01 dan
diulang di review Spec 005: test `concurrent_writes_to_same_sqlite_session_are_serialized`
bersifat intermittent di bawah beban CPU tinggi.

**Blocked by:** — (independen, bisa paralel kapan saja).

**Status:** todo

## Kondisi sekarang

`crates/hermes-core/tests/sqlite_parity_integration.rs` memuat helper
`save_turn_retry_busy` (ditambahkan saat Ticket 02) yang melakukan retry pada
`DatabaseBusy`/`DatabaseLocked`. Namun sebagian flake masih muncul di full-suite
karena `busy_timeout` (5 dtk di `SessionStore::open`) kadang tidak cukup saat dua
koneksi terpisah bersaing di bawah load, dan helper retry berbasis `thread::sleep`
bisa menambah latensi.

## Kriteria

- [ ] Root-cause flake tersisa didiagnosis (cek apakah `busy_timeout` terpasang
      benar untuk semua koneksi termasuk yang dibuka test, dan apakah journal
      mode / `SQLITE_BUSY` handling konsisten).
- [ ] Perbaikan tidak mengubah semantik produksi `SessionStore` (hanya
      membuatnya benar-benar menunggu write lock) bila memungkinkan; alternatif
      paling bersih: naikkan `busy_timeout` atau pasang retry di lapisan
      koneksi, bukan menutupi di test.
- [ ] Test berjalan stabil (mis. 10x berurutan) di full-suite, bukan hanya isolasi.
- [ ] Tidak melemahkan assert (jumlah turn akhir tetap 20).
- [ ] Tidak menambah dependency baru.

## Catatan

Jika perbaikan produksi lebih tepat (mis. `busy_timeout` naik / WAL), itu boleh —
selama seluruh test `sqlite_parity_integration` dan `search` tetap hijau.

## Dependency

— (bisa dikerjakan paralel).
