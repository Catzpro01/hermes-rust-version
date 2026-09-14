# Tinta kolom status picker (fix `1781404`, capture `19bbbd5`)

RED [34870302745](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34870302745)
→ GREEN resmi [34871381451](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34871381451)
→ capture sumber ter-commit
[34871743793](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34871743793)
→ CI biasa
[34871741338](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34871741338) SUCCESS.

Scope: **tinta tag status** pada baris yang bukan kursor. Teks tag, kolom
`Active`/`ID`, geometri baris dan gaya baris kursor tidak diubah.

## Kontrak yang dipatok referensi

§F hanya menyebut `_status_attr` tanpa memetakan paletnya. Pemetaannya dibaca
dari sumber upstream yang dipatok — commit `63279301`, file
`hermes_cli/main.py` (blob `8281cbdd…`, sha256 `89cde75d…`, 625.460 byte,
tersimpan di `docs/hermes-ui-spec/017/evidence/upstream-status-attr/` bersama
provenance) — dan dikonfirmasi silang oleh capture yang sudah ada.

| Status sesi | Tag | Pair (upstream) | Slot | 100 kolom | 80 kolom |
|---|---|---|---|---|---|
| `complete` | `done` | `color_pair(1)` | 2 (green) | kolom 43…48 | kolom 25…30 |
| `interrupted` | `intr` | `color_pair(2)` | 3 (yellow) | sama | sama |
| `error` | `err` | `color_pair(5)` | 1 (red) | sama | sama |
| `empty` | `empty` | `color_pair(4)` | 8 (palette8) | sama | sama |
| lainnya | `-` | `A_NORMAL` | default | sama | sama |

Posisi: `tag_x = 3 + max(20, (max_x − 3) − _FIXED_COLS) + 2`, lima sel, dan
hanya digambar ulang bila baris itu **bukan** baris kursor. Bukti silang dari
capture lama: `intr` terukur di slot 3 dan prompt delete di slot 1 — persis
pair2 dan pair5.

## Perubahan (patch teruji `73d0322e…`, 6.109 byte)

`crates/hermes-cli/src/session_picker.rs`:

- `status_ink(status)` memetakan `done`/`intr`/`err`/`empty` ke slot pair-nya
  dan sisanya ke `Color::Reset` (A_NORMAL);
- `status_tag_span(name_width)` menurunkan `3 + name_width + 2 .. +5`;
- cabang baris non-kursor mencetak kepala baris, tag lima sel berwarna, lalu
  ekornya — hanya bila baris itu punya tag (baris pemisah dan baris kosong
  dilewati), dan baris kursor tetap memakai palette2 + bold seperti siklus
  sebelumnya;
- unit test mengunci pemetaan, span, dan konversi char→byte untuk pemotongan.

Patch yang diminta (`e88347c8…`, 6.007 byte) dan patch yang diuji ekspor
(`73d0322e…`) berbeda hanya karena pelari menjalankan `cargo fmt --all`;
keduanya disimpan (`request.patch`, `tested.patch`).

## Verifikasi

- RED: gate live gagal 3× dengan nama tes benar dan **tanpa** error setup —
  Rust menggambar `done` dengan tinta default, sedangkan pemetaan hasil
  menetapkan slot 2.
- GREEN: patch diekspor identik byte dengan yang diterapkan ke tree; gate live
  3×, fmt/check, `clippy --workspace --all-targets -D warnings` dan seluruh
  suite PASS.
- Capture: 10 kasus; **sepuluh** checker (hint, counter, posisi, warna, header
  normal, header filter, tata letak kolom, prompt/message, baris terpilih,
  tinta status) PASS pada sumber ter-commit, dan capture juga memverifikasi
  regresi 3×. Bundle `119e299b…`, 204.818 byte.
- `verify.py` → `audit.json` `STATUS_TAG_INK_FIXED_ALL_CONTROL_REGIONS_EQUAL`:
  20 round trip raw/cast, 10 hash PNG, 14 jalan checker, **34 dari 34 region
  kontrol tetap identik** dan 6 PNG kasus tanpa baris tak terpilih byte-identik.
- Delta sel terhadap paket `picker-selection-b4cb408`: **hanya baris 5** pada
  skenario normal dan delete di kedua lebar — 5 sel per baris (20 sel). Tiap
  sel: `fg default→2` dan mode palet256 `0→33554432`; **teks baris tidak
  berubah**, begitu juga seluruh sel dan rekaman sisi Python.
- Empat region baru mendokumentasikan span tag sebagai **perbedaan yang
  dinyatakan**: kata tag adalah data fixture (sesi Python yang dipertahankan
  berstatus interrupted, fixture Rust complete), jadi glifnya memang tidak bisa
  sama (masing-masing 270 piksel berbeda). Tintanya dipatok checker dua sisi,
  bukan oleh piksel.

## Percobaan yang gagal (disimpan, bukan disembunyikan)

1. GREEN [34870642732](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34870642732):
   semua kasus `process exit 101 at stage 0`. Biner yang dipatch panik
   `attempt to subtract with overflow` di `session_picker.rs:526` — baris
   pemisah kosong masuk ke cabang redraw yang sama dan `n − 3` underflow untuk
   `n == 2`. Teks panik hanya bisa dipulihkan dari anotasi trace
   (`bc0b233a…`), karena byte PTY membawa pesan panik sedangkan harness hanya
   melaporkan exit code.
2. GREEN [34871081677](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34871081677):
   gate live lulus 3×, tetapi unit test baru memotong baris 20 kolom di offset
   tag 100 kolom (`"go   "` vs `"done "`) sehingga langkah 24 gagal pada
   assertion itu saja.
3. CI [34871743697](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34871743697)
   (headSha `19bbbd5`): flake start-up PTY yang sudah dikenal — layar sudah
   tergambar penuh tetapi `missing readiness/timeout at stage 0` setelah 207.901
   byte, karena picker Rust menggambar ulang seluruh frame setiap poll timeout
   sehingga aturan snapshot "quiet atau stable" bisa kalah balapan saat pelari
   sibuk. Bukan kegagalan produk: capture pada commit yang sama dan CI pada
   sumber identik (`1781404`) hijau. Penyimpangan repaint terus-menerus itu
   sendiri masuk antrean sebagai siklus picker berikutnya.
4. CI selama jendela RED memang merah: sumber ter-commit belum dipatch. Karena
   langkah picker memakai `continue-on-error: true`, daftar langkah tetap
   menunjukkan success sementara `steps.picker.outcome` failure dan langkah
   agregat terakhir menggagalkan job. Itulah juga "misteri" GREEN siklus 9
   (`34866468131`): bacalah anotasi `picker terminal`, bukan daftar langkah.

## Masih terbuka di luar region ini

- Capture hidup hanya menjalankan pemetaan `complete→done`; tinta
  `interrupted`/`error`/`empty` dipatok oleh sumber upstream, unit test, dan
  fixture checker — belum oleh capture berpasangan.
- Kolom `Active` (`2y ago` vs `2023-11-14`) dan `ID` (8 vs 18 karakter) tetap
  adaptasi terdokumentasi.
- Resize, daftar panjang, dan clear-filter belum dibuktikan fixture ukuran
  tetap ini.
- Picker menggambar ulang setiap poll timeout (referensi hanya menggambar
  setelah tombol); kandidat siklus berikutnya.
