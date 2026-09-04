# 04: `/provider <name>` — switch mid-session

**What to build:** Perintah REPL untuk mengganti provider tanpa kehilangan session yang sedang berjalan.

**Blocked by:** 01 — Provider registry.

**Status:** done — commit di VM, 114/114 test hijau (`cargo test --workspace`), `clippy --workspace --all-targets -D warnings` bersih.

## Kondisi sekarang (terverifikasi)

`repl::run_repl(&home, provider, args.resume)` menerima **satu**
`Box<dyn Provider>` yang diikat saat startup. Tidak ada mekanisme penggantian.
Perintah yang ada sekarang:

```
/new, /sessions, /inspect <id>, /messages <id>, /tool-calls <id>, /search <query>, /resume <id>, /exit
```

## Kriteria

- [x] `/provider` tanpa argumen mendaftar provider yang tersedia dan menandai yang aktif.
      `list_providers` menampilkan `registry.available()` (terurut) dengan marker `(active)`; bila aktif berasal dari fallback model-level dan tak terdaftar, tetap dicetak agar marker selalu tampil.
- [x] `/provider <name>` mengganti provider aktif; nama tak dikenal menghasilkan error tanpa mengubah provider aktif.
      `resolve_provider` memakai `registry.select(Some(name), None, override, config)`; pada error, provider aktif dibiarkan (rollback).
- [x] Session dan riwayat `Turn` **tidak** terpengaruh oleh penggantian provider.
      `ConversationRunner::replace_provider` hanya menukar `self.provider`; `self.turns` utuh. Diuji di `provider_switch_integration.rs`.
- [x] Provider baru tidak menyimpan state apa pun yang diwarisi dari provider lama.
      Provider (Fake/Http) stateless; hanya ditukar di runner. Tak ada state yang disalin antar provider.
- [x] SIGINT di tengah stream tetap keluar 130 setelah penggantian provider.
      Mekanisme pembatalan stream (`turn_cancel`/`CancellationToken` + return `Err(interrupted)` → kode 130 di `main`) tidak menyentuh variabel provider; tes `smoke_sigint_returns_130` dan `sigint_stream` tetap hijau.
- [x] Gagal inisialisasi provider (mis. `key_env` kosong) **tidak** merusak provider yang sedang aktif — rollback, bukan setengah jalan.
      Build dilakukan dulu ke `Ok(new_provider)`; hanya setelah sukses baru `replace_provider` dipanggil. Diuji di `provider_switch_e2e.rs`.

## Batas yang harus ditegakkan

Invariant project: *partial turn tidak pernah disimpan saat cancellation*.
Penggantian provider hanya boleh terjadi **di batas turn**. Karena REPL membaca
perintah berikutnya hanya setelah satu turn selesai (read-loop idle di sini),
`/provider` praktis hanya bisa dijalankan saat idle — tak ada jalur untuk
menyisipkannya di tengah stream aktif. Jika pengguna menekan Ctrl+C di tengah
stream, jalur SIGINT (bukan perintah) yang aktif → cancel + exit 130. Dengan
begitu satu turn tak pernah tercatat berasal dari dua provider. Hal ini
didokumentasikan di kode (`repl.rs`).

`state.db` tetap satu-satunya canonical storage — provider aktif **tidak**
disimpan ke DB maupun file lain; ia hanya hidup di variabel `provider_name`
dalam proses.

## Perubahan

- `conversation/mod.rs`: tambah `ConversationRunner::replace_provider(P)`.
- `repl.rs`: `run_repl` kini menerima `provider_name`, `registry`, `config`, dan `base_url_override`; tambah perintah `/provider [name]`, helper `resolve_provider` & `list_providers`, dan unit test.
- `main.rs`: hitung `provider_name` (presedensi sama dengan `select`) dan teruskan argumen REPL baru.
- Tes baru: `crates/hermes-core/tests/provider_switch_integration.rs` (2, riwayat + pergantian), `crates/hermes-cli/tests/provider_switch_e2e.rs` (2, list/rollback & switch mempertahankan session), + 3 unit test di `repl.rs`.

## Pengujian

- `cargo test --workspace` → 114/114 (naik dari 107; +7).
- `clippy --workspace --all-targets -D warnings` → bersih.
