# Tata letak kolom picker (fix `641c304`, capture `b732d22`)

RED [34860668321](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34860668321)
→ GREEN resmi [34861285022](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34861285022)
→ capture sumber ter-commit
[34861588181](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34861588181)
→ CI biasa [34861587979](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34861587979) SUCCESS.

Scope: **tata letak kolom** picker — kolom kursor 3 sel, baris kosong pemisah,
lebar medan nama badan dan header, serta tinta header. Isi kolom status/`Active`/`ID`
dan gaya baris terpilih tidak diubah di sini.

## Koordinat yang dipatok referensi

Referensi Python v0.21.0 yang dipertahankan (`ui-3b39bd7`) hanya memuat lebar 100
dan 80 kolom. Pada keduanya:

| Fakta | 100 kolom | 80 kolom |
|---|---|---|
| Indent header | 3 sel | 3 sel |
| `Stat` | x=46 (= width-54) | x=26 (= width-54) |
| Kolom status badan | x=43 (= width-57) | x=25 (= width-57) |
| Baris header / baris kosong / baris badan pertama | 2 / 3 / 4 | 2 / 3 / 4 |
| Kolom kursor | ` → ` terpilih, tiga spasi lainnya | sama |
| Tinta header | palette8 brightblack, tanpa bold | sama |

Medan nama badan = `width-62` dan medan header = `width-59`, keduanya di-floor ke
nilai 80 kolom (20). Formula ini **diturunkan dari dua lebar yang dipatok saja**;
terminal lebih sempit dari 80 kolom tidak dibuktikan referensi dan tidak diklaim.

## Perubahan (patch teruji `006bdcc7…`, 12.824 byte)

`session_picker.rs`: `header_name_width` dan `row_prefix` baru; `column_header`
memakai indent 3 sel dan medan sendiri; `format_row` mengambil `&SessionRow`,
lebar medan dan flag kursor; `frame_lines` menerima lebar terminal dan menyisipkan
baris kosong; `browse()`/`browse_numbered()` menyesuaikan; header digambar dengan
`Color::DarkGrey`; indeks baris kursor bergeser ke belakang pemisah; unit test
dipin ulang ke literal referensi (`name_width` 38/20, `header_name_width` 41/21).

## Verifikasi

- GREEN resmi menjalankan gate live 3×, `cargo clippy -D warnings` dan seluruh
  suite workspace; patch hasil ekspor identik byte dengan proposal
  (`candidate patch digest` = `006bdcc7…`, 12.824 byte) dan diff lokal juga identik.
- Capture sumber ter-commit bersih: 10 kasus, plus hint/counter/posisi/warna/header
  normal/header filter/tata letak kolom semuanya PASS pada sumber ter-commit.
- `verify.py` (dijalankan ulang untuk menghasilkan `audit.json`): 20 round trip
  raw/cast, 10 hash PNG, 8 jalan checker (2 sisi untuk gate baru + 6 gate lama),
  14 region piksel pinned-renderer identik, 2 PNG (kasus empty) byte-identik dengan
  paket sebelumnya, dan peta baris berubah per kasus seperti tercatat di audit.
- Rekaman Python dipakai ulang tanpa perubahan (`python_origin` di
  `paired-bundle.json`); sel Python juga tidak berubah dibanding paket baseline.

## Temuan baru — belum diperbaiki di slice ini

Perbandingan piksel menemukan bahwa pesan `  No sessions match the filter.`
digambar referensi dengan atribut **dim** (`ESC[0;2m`): 1.135 piksel berbeda pada
baris itu di kedua lebar, sementara baris header identik. `pyte 0.8.2` tidak
menyimpan atribut dim, jadi seluruh gate berbasis pyte (dan audit sebelumnya)
tidak dapat melihatnya — hanya perbandingan piksel pinned-renderer yang
menangkapnya. Fix dan gate-nya (ejaan SGR-2 pada aliran + kesetaraan piksel)
dijadwalkan sebagai siklus berikutnya; temuan ini dicatat, bukan dinormalisasi.
Baris footer pada kasus yang sama justru pixel-identik, jadi state dim Python
tidak bocor ke footer secara visual.

## Batasan

- Siklus ini tidak menutup picker: gaya baris terpilih (` → ` hijau+bold),
  warna prompt hapus, warna kolom status (tidak ada bukti terpinn untuk `done`),
  serta konten kolom yang merupakan adaptasi terdokumentasi (sid 8 karakter,
  `done`/`empty`, `Active` relatif) tetap terbuka.
- Tidak ada adaptasi baru, klaim resize/daftar panjang/clear-filter, PASS seluruh
  picker, acceptance, penutupan Spec017, atau merge.

Reproduksi: `verify.py` di ruang kerja ini (`SHA256SUMS` untuk seluruh berkas);
`verify-regions.cjs` + `region-spec*.json` mereproduksi perbandingan piksel.
