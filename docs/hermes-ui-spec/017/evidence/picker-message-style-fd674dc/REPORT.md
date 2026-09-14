# Gaya baris prompt picker (fix `549fc8d`, capture `fd674dc`)

RED [34864078749](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34864078749)
→ GREEN resmi [34864360124](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34864360124)
→ capture sumber ter-commit
[34864672852](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34864672852)
→ CI biasa [34864669012](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34864669012)
dan [34864672933](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34864672933) SUCCESS.

Scope: **dua baris prompt/pesan** picker — prompt konfirmasi hapus (palette slot 1 +
bold) dan pesan no-match (atribut **dim**). Gaya baris terpilih dan warna kolom
status **tidak** diubah di sini; keduanya tetap siklus berikutnya.

## Kontrak yang dipatok referensi

| Fakta | Sumber |
|---|---|
| Prompt `  Delete session '…'? [y/N]` digambar dengan palette1 (`SGR 31`) + bold di baris terakhir | Referensi `ui-3b39bd7`, kasus delete 100/80 |
| Pesan `  No sessions match the filter.` digambar dengan atribut dim (`ESC[0;2m`) | Byte stream referensi: dim dibuka di baris4 kolom1, baru di-reset saat footer digambar ulang |
| Dim **tidak dapat didekode** `pyte 0.8.2` | Karena itu gate membaca byte stream + decoder renderer, dan paket membuktikan pikselnya |
| Bentuk indexed (`38;5;1`, `7f7f7f`) dan ANSI (`31`, `brightblack`) diterima sebagai pasangan palet | Preseden palette3/palette6/palette8 yang sudah rilis; piksel tetap identik |

## Perubahan (patch teruji `d1ed7f99…`, 2.216 byte)

`crates/hermes-cli/src/session_picker.rs`: bran baru untuk baris `NO_MATCH` yang
mencetak pesan dengan `Attribute::Dim` lalu me-reset **di tempat**, dan prompt hapus
kini dicetak dengan `SetForegroundColor(Color::DarkRed)` + `Attribute::Bold` + reset.
Tidak ada perubahan lain pada loop penggambaran, geometri, atau teks.

## Verifikasi

- RED: tes primer gagal 3× dengan nama benar dan **tanpa** error setup; 4 masalah
  tercatat (dua baris dim hilang, dua prompt belum berwarna). Trace RED disimpan
  (`fceac7f5…`, 84.235 byte).
- GREEN: patch hasil ekspor identik byte dengan proposal (`d1ed7f99…`, 2.216 byte);
  gate live 3×, `cargo fmt`/`check`, `clippy --workspace --all-targets -D warnings`
  dan seluruh suite workspace PASS. Trace GREEN `e91d1c8d…`, 62.602 byte.
- Capture: 10 kasus dari sumber ter-commit; **delapan** checker (hint, counter,
  posisi, warna, header normal, header filter, tata letak kolom, prompt/message)
  semuanya PASS pada sumber itu. Bundle `a8f37c46…`, 166.599 byte.
- `verify.py` → `audit.json` `MESSAGE_STYLE_FIXED_ALL_PINNED_REGIONS_EQUAL`:
  20 round trip raw/cast, 10 hash PNG, 10 jalan checker (2 sisi untuk gate baru dan
  tata letak kolom), **28 dari 28 region piksel identik** — termasuk baris pesan
  no-match yang sebelumnya berbeda 1.135 piksel di kedua lebar dan dua region baru
  untuk baris prompt hapus — serta 6 PNG kasus empty byte-identik dengan paket
  sebelumnya.
- Delta sel terhadap paket `picker-column-layout-b732d22`: hanya dua baris Rust
  berubah per lebar. Baris pesan no-match: 31 sel, **hanya** flag `dim`
  (`False → True`). Baris prompt hapus: 38 sel, **hanya** foreground (`-1 → 1`),
  bold (`False → True`) dan mode palet256 (`0 → 33554432`). Sel dan rekaman Python
  tidak berubah sama sekali.
- Perbedaan urutan escape (referensi membiarkan dim aktif sampai redraw footer,
  Rust mereset di tempat) terbukti tidak terlihat: region baris pesan dan baris
  footer keduanya pixel-identik. Rust membuka lebih banyak window dim pada rekaman
  yang sama (7/10 vs 4) karena jumlah frame redraw berbeda; setiap window dibuka di
  baris4 kolom1 dan window pertama tepat berisi pesan.

## Batasan

- Siklus ini **bukan** PASS seluruh picker: baris terpilih (` → ` palette2 hijau +
  bold, hari ini reverse video), warna kolom status (belum ada bukti terpinn untuk
  nilai `done`), konten `Active`/`ID` (adaptasi terdokumentasi), serta resize,
  daftar panjang dan clear-filter tetap terbuka.
- Tidak ada adaptasi baru, normalisasi, klaim acceptance, penutupan Spec017, atau
  merge.

Reproduksi: `verify.py` di ruang kerja ini; `SHA256SUMS` memuat seluruh berkas;
`verify-regions.cjs` + `region-spec.json` mereproduksi perbandingan piksel, dan
`pair-bundle.py` mereproduksi pasangan dengan rekaman Python yang dipakai ulang.
