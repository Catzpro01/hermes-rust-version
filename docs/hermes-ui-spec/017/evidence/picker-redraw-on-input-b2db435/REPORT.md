# Cadence redraw picker: gambar karena input, bukan karena poll timeout (fix `bbd943c`, capture `b2db435`)

RED [34873144309](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34873144309)
→ GREEN resmi [34873480643](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34873480643)
→ capture sumber ter-commit
[34873776470](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34873776470)
→ CI biasa
[34873775366](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34873775366) /
[34873776473](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34873776473) SUCCESS.

Scope: **kapan** picker menggambar, bukan **apa** yang digambar. Konten, geometri,
warna, teks dan perilaku tombol tidak diubah — paket ini membuktikannya dengan
membandingkan seluruh hasil render terhadap paket siklus sebelumnya.

## Kontrak yang dipatok referensi

`_curses_browse` menggambar frame di awal loop lalu **memblokir di
`stdscr.getch()`**: satu frame per tombol, dan tidak ada output sama sekali
selama menunggu. Port Rust tetap melakukan poll 100 ms (agar tetap responsif
terhadap sinyal), jadi kontraknya adalah: poll boleh terjadi, tetapi menggambar
hanya ketika layar berubah.

Gate `picker_redraw_on_input` membelah setiap rekaman pada penulisan tombol
skenario (balasan terminal diabaikan) dan menghitung frame per jendela input:

| Kasus | Sebelum (paket `picker-status-ink-19bbbd5`) | Sesudah | Byte rekaman |
|---|---|---|---|
| `picker-normal-100x30` | 5 frame tanpa input apa pun | 1 | 2329 → 477 |
| `picker-filter-100x30` | 6 lalu 8 (`topic` = 5 tombol) | 1 lalu 5 | 5505 → 2200 |
| `picker-no-match-100x30` | 25 lalu 9 (`zzzz` = 4 tombol) | 1 lalu 4 | 13815 → 1463 |
| `picker-delete-100x30` | 14 lalu 0 | 1 lalu 0 | 6562 → 543 |
| `picker-normal-80x30` | 8 frame tanpa input | 1 | 3270 → 421 |
| `picker-filter-80x30` | 41 lalu 8 | 1 lalu 5 | 19092 → 1936 |
| `picker-no-match-80x30` | 7 lalu 7 | 1 lalu 4 | 4453 → 1327 |
| `picker-delete-80x30` | 21 lalu 0 | 1 lalu 0 | 8627 → 487 |
| `picker-empty-{100,80}x30` | 1 | 1 | 20 → 20 (kontrol) |

Anggaran jendela: 1 frame sebelum tombol pertama, lalu satu frame per tombol yang
diketik (angka ini diambil dari rekaman referensi Python: `topic` memang 5 frame,
`zzzz` 4 frame, `d` 1 frame).

## Perubahan (patch teruji `925b963e…`, 2.801 byte)

`crates/hermes-cli/src/session_picker.rs`: flag `dirty` (mulai `true`) membungkus
blok redraw; setiap tombol yang diterima dan setiap `Event::Resize` menandai layar
kotor, dan blok redraw mengosongkan flag setelah flush. Loop poll 100 ms tidak
berubah sehingga penanganan sinyal tetap sama. Patch yang diminta dan yang diuji
ekspor **identik byte** (rustfmt tidak mengubah apa pun), jadi hanya ada satu file
patch di paket ini.

## Verifikasi

- RED: gate live gagal 3× dengan assertion yang menyebut kadens referensi dan
  **tanpa** error setup; jejak RED 279.052 byte.
- GREEN: percobaan pertama lolos — gate live 3×, fmt/check, `clippy -D warnings`
  dan seluruh suite PASS; jejak turun ke 45.748 byte sebagai bukti langsung bahwa
  repaint berhenti.
- Capture: 10 kasus; **sebelas** checker (hint, counter, posisi, warna, header
  normal, header filter, tata letak kolom, prompt/message, baris terpilih, tinta
  status, cadence redraw) PASS pada sumber ter-commit, plus verifikasi 3×. Bundle
  `491713dd…`, 46.091 byte.
- `verify.py` → `audit.json`
  `REDRAW_ON_INPUT_FIXED_FRAME_CONTENT_UNCHANGED_ALL_REGIONS_EQUAL`:
  20 round trip raw/cast, 10 hash PNG, 16 jalan checker, 34/34 region kontrol
  identik, dan 4 region span tag tetap sebagai perbedaan yang dinyatakan.
- **Konten tidak berubah**: kesepuluh cell map dan kesepuluh PNG identik byte
  dengan paket `picker-status-ink-19bbbd5`. Siklus ini hanya mengubah kadens.
- Gate yang sama menolak 8 dari 10 kasus di paket sebelumnya dan menerima 10/10
  rekaman referensi Python — jadi gate-nya memang mengukur perilaku yang
  diperbaiki, bukan sekadar cocok dengan paket barunya.
- Efek samping yang penting: flake start-up PTY hilang. Dua CI pada sumber yang
  sama (`34873775366`, `34873776473`) hijau; sebelumnya
  (`34871743697`) gagal `missing readiness/timeout at stage 0` setelah 207.901 byte
  walaupun layarnya sudah penuh.

## Percobaan yang gagal (disimpan, bukan disembunyikan)

1. CI selama jendela RED (`34873144303` pada `7a9ab45`, `34873480642` pada
   `d69c97e`) merah karena gate kesembilan sudah dikirim sebelum fix-nya. Karena
   langkah picker memakai `continue-on-error: true`, daftar langkah tetap
   success sementara `steps.picker.outcome` failure dan langkah agregat
   menggagalkan job — bacalah anotasi `picker terminal`.
2. Tidak ada percobaan GREEN yang gagal di siklus ini: permintaan pertama lulus
   3× dan patch ekspornya identik byte dengan yang diikat. Ini pertama kalinya
   dalam pekerjaan picker sebuah permintaan GREEN langsung berhasil, dan itu
   memang efek yang diharapkan dari menghapus repaint terus-menerus.

## Masih terbuka di luar region ini

- Capture hidup hanya menjalankan pemetaan `complete→done`; tinta
  `interrupted`/`error`/`empty` dipatok sumber upstream, unit test dan fixture
  checker.
- Kolom `Active` dan `ID` tetap adaptasi terdokumentasi.
- Resize, daftar panjang, dan clear-filter belum dibuktikan fixture ukuran tetap
  ini; resize baru tercakup sebatas penandaan dirty.
