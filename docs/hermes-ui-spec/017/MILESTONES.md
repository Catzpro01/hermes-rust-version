# Milestone Hermes — visual parity Spec017

**Pembaruan: 16 September 2026 · Tinta kolom status, lifecycle W5 dan kontrol browse (resize/daftar panjang/clear-filter) kini terverifikasi oleh gate live di CI resmi; job Rust berat (`fmt + clippy + test`) sudah diaktifkan kembali.**

Halaman ini melacak pekerjaan visual parity yang sedang aktif, **bukan persentase
seluruh proyek Hermes**. “Terverifikasi” berlaku untuk scope yang disebutkan;
bukan berarti seluruh layar identik atau Spec017 sudah diterima pengguna.

## Ringkasan milestone

| Milestone | Status | Bukti / batasan |
|---|---|---|
| Paket pembanding UI awal | Tersedia, masih ada perbedaan | [48 pasangan empat area](evidence/ui-3b39bd7/REPORT.md); cakupan nested wizard/full Python CLI belum lengkap |
| Petunjuk hapus picker sesuai kondisi filter | ✅ Terverifikasi | [c07f0c5](evidence/picker-hint-c07f0c5/REPORT.md) |
| Counter tanpa hasil tetap menunjukkan total sesi | ✅ Terverifikasi | [a8d5e8c](evidence/picker-counter-a8d5e8c/REPORT.md) |
| Footer berada di baris terakhir terminal | ✅ Terverifikasi | [0c0704d: 10 pasangan diperiksa](evidence/picker-position-0c0704d/REPORT.md) |
| Warna footer sesuai Python | ✅ Terverifikasi | [ae220ff: 10 gambar, 6 footer pixel-identik](evidence/picker-color-ae220ff/REPORT.md) |
| Header bantuan mode normal | ✅ Terverifikasi | [7fef514: palette3 + bold, 4 header pixel-identik](evidence/picker-header-7fef514/REPORT.md) |
| Header filter picker (palette6 + bold) | ✅ Terverifikasi | [9cc5cb4: 4 piksel row1 identik, slot6+bold](evidence/picker-filter-header-9cc5cb4/REPORT.md) |
| Tata letak kolom picker (kolom kursor, baris pemisah, medan nama, tinta header) | ✅ Terverifikasi | [b732d22: 14 region piksel identik, 2 PNG byte-identik](evidence/picker-column-layout-b732d22/REPORT.md) |
| Prompt konfirmasi hapus (palette1 + bold) dan pesan no-match (atribut dim) | ✅ Terverifikasi | [fd674dc: 28 dari 28 region piksel identik](evidence/picker-message-style-fd674dc/REPORT.md) |
| Baris terpilih picker (palette2 + bold, tanpa reverse video) | ✅ Terverifikasi | [b4cb408: 34 dari 34 region piksel identik](evidence/picker-selection-b4cb408/REPORT.md) |
| Warna kolom status picker | ✅ Terverifikasi | [19bbbd5: 34/34 region kontrol identik](evidence/picker-status-ink-19bbbd5/REPORT.md); gate `picker-status-ink` wajib di CI |
| Lifecycle status W5 (opsi C: `done`/`intr`/`err`/`empty`) | ✅ Terverifikasi | Gate `scripts/test_picker_status_tags.py` GREEN pada run [35046192977](https://github.com/Catzpro01/hermes-rust-version/actions/runs/35046192977) (`fmt=clippy=test=picker=success`) bersama 13 gate PTY lain; adaptasi role tool dicatat di ADR 0007 |
| Resize / daftar panjang / clear-filter | ✅ Terverifikasi | Gate `scripts/test_picker_browse_control.py`, lima skenario (`resize-too-small`, `resize-redraw`, `long-list`, `clear-filter-esc`, `clear-filter-backspace`) dipatok ke [upstream-browse-control](evidence/upstream-browse-control/); GREEN pada run yang sama, ditambah `picker-terminal-size` (40 kolom menggambar picker, 39 kolom hanya `Terminal too small`) |
| Konten kolom `Active`/`ID` | ⚠️ Adaptasi terdokumentasi | `sid` 8 karakter untuk UUIDv7; `Active` waktu relatif ≤10 sel. Dinyatakan sebagai adaptasi, bukan diklaim identik |
| Sisa perbedaan wizard/completion dan kelengkapan bukti | ⏳ Belum selesai | T12/T13 tetap terbuka |
| Penerimaan akhir Spec017 | 🔒 Belum siap | Memerlukan bukti lengkap, CI relevan GREEN, dan persetujuan eksplisit pengguna |

## Header filter picker — F1–F5 selesai

Scope: baris bantuan `Browse sessions` ketika filter sedang diketik
(`picker-filter` dan `picker-no-match`), **palette slot 6 + bold**. Header
normal (tanpa filter, palette3), footer, counter, hint hapus dan geometri tidak
ikut diubah.

| Tahap | Status | Bukti / kriteria |
|---|---|---|
| F1 · Kontrak dan referensi | ✅ Selesai | Referensi Python v0.21.0 yang dipertahankan (`ui-3b39bd7`): `SGR 0;1` + `36` = palette6 + bold di empat kasus |
| F2 · RED nyata | ✅ Selesai | [34858664487](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34858664487) menolak percobaan slot14; RED pertama [34857289160](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34857289160) mencatat kegagalan atribut default |
| F3 · Perubahan minimal | ✅ Selesai | Patch 598 byte (`Color::DarkCyan`) diterapkan persis setelah validasi runner |
| F4 · GREEN resmi | ✅ Selesai | [34858865863](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34858865863): regresi primer3×, fmt/check/clippy/suite PASS |
| F5 · Capture dan review | ✅ Selesai | [Capture34859140850](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34859140850) + [CI34859140646](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34859140646) GREEN; 10 kasus, 4 piksel row1 identik |

Tes regresi live keempat kini diwajibkan di CI biasa bersama posisi, warna
footer, dan header normal. Paket mencatat percobaan yang ditolak
(`first-attempt-bright-variant.txt`) sebagai bukti bahwa slot warna tidak
dinormalisasi.

## Header bantuan mode normal — H1–H5 selesai

Scope: baris bantuan `Browse sessions` ketika filter kosong, **palette3 + bold**.
Header yang sama juga tampak saat konfirmasi hapus. Warna prompt hapus sendiri,
header filter/kolom, seleksi, footer dan posisi teks tidak ikut diubah.

| Tahap | Status | Bukti / kriteria |
|---|---|---|
| H1 · Kontrak dan referensi | ✅ Selesai | CLI/PTY nyata normal100×30 disepakati; referensi Python4/4 normal/delete pada100/80 memakai palette3 + bold |
| H2 · RED nyata | ✅ Selesai | [34836863589 / 4aa0858](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34836863589): tes primer FAIL3, default/tidak bold; fmt/check/build PASS |
| H3 · Perubahan minimal | ✅ Selesai | Patch1171 byte resmi diterapkan persis; DarkYellow + Bold untuk header filter kosong, lalu reset |
| H4 · GREEN resmi | ✅ Selesai | [34837102702 / 645f86d](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34837102702): primer3, fmt/check/clippy/full workspace PASS |
| H5 · Capture dan review | ✅ Selesai | [Capture34837424495](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34837424495) + [CI34837424464](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34837424464) GREEN; 10 gambar diperiksa, audit isolasi PASS |

### Baris terpilih picker — S1–S5 selesai

Scope: seluruh baris kursor (termasuk penanda ` → `) memakai palette slot 2 + bold
tanpa reverse video. Kolom status, konten `Active`/`ID`, geometri dan teks tidak
ikut diubah.

| Tahap | Status | Bukti / kriteria |
|---|---|---|
| S1 · Kontrak dan referensi | ✅ Selesai | Referensi v0.21.0: satu run `palette2 + bold`, tanpa reverse, span0…93 (100 kolom) dan0…75 (80 kolom) |
| S2 · RED nyata | ✅ Selesai | [34866264371](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34866264371): tes primer FAIL3× dengan nama benar, tanpa error setup |
| S3 · Perubahan minimal | ✅ Selesai | Patch1.171 byte `8eed758f…` diterapkan persis (`Color::DarkGreen` + bold menggantikan reverse) |
| S4 · GREEN resmi | ✅ Selesai | [34866921566](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34866921566): gate live3×, fmt/check, clippy `-D warnings` dan seluruh suite PASS |
| S5 · Capture dan review | ✅ Selesai | [Capture34868306211](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34868306211) + CI GREEN; 34/34 region piksel identik,9 checker PASS,4 PNG byte-identik |

Gate live ketujuh (`picker_selection`) menyusul CI biasa. Siklus ini juga
memunculkan dan menutup tiga masalah nyata: pemetaan SGR `38;5;2` sebagai dim di
checker kami sendiri, flake start-up PTY (kini 45 s + pesan diagnostik), dan satu
percobaan GREEN yang gagal tanpa dapat direproduksi — semuanya tercatat di
`attempts-result.txt` paket. Sisa picker: warna kolom status (butuh bukti
terpinn), konten `Active`/`ID` (adaptasi), resize/daftar panjang/clear-filter.

### Baris prompt/pesan picker — P1–P5 selesai

Scope: prompt konfirmasi hapus (palette slot 1 + bold) dan pesan
`  No sessions match the filter.` (atribut **dim**). Gaya baris terpilih, warna
kolom status, geometri dan teks tidak ikut diubah.

| Tahap | Status | Bukti / kriteria |
|---|---|---|
| P1 · Kontrak dan referensi | ✅ Selesai | Referensi v0.21.0 (`ui-3b39bd7`): prompt `SGR 31` + bold di baris terakhir; dim dibuka `ESC[0;2m` di baris4 kolom1 dan baru di-reset saat footer digambar ulang |
| P2 · RED nyata | ✅ Selesai | [34864078749](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34864078749): tes primer FAIL3× dengan nama benar, 4 masalah, tanpa error setup |
| P3 · Perubahan minimal | ✅ Selesai | Patch2.216 byte `d1ed7f99…` diterapkan persis (dim + reset di tempat; `Color::DarkRed` + bold untuk prompt) |
| P4 · GREEN resmi | ✅ Selesai | [34864360124](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34864360124): gate live3×, fmt/check, clippy `-D warnings` dan seluruh suite PASS |
| P5 · Capture dan review | ✅ Selesai | [Capture34864672852](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34864672852) + CI34864669012/34864672933 GREEN; 28/28 region piksel identik, 6 PNG byte-identik, 8 checker PASS di sumber ter-commit |

Temuan `dim` dari slice tata letak kolom kini tertutup: baris pesan yang tadinya
berbeda1.135 piksel di kedua lebar sudah identik, dan urutan escape referensi
(dim aktif sampai redraw footer) terbukti tidak terlihat. Gate live keenam
(`picker_message_style`) menyusul di CI biasa pada siklus berikutnya; yang masih
terbuka adalah baris terpilih, warna kolom status, konten `Active`/`ID`, serta
resize/daftar panjang/clear-filter.

### Tata letak kolom picker — K1–K5 selesai

Scope: kolom kursor 3 sel (` → `/tiga spasi), baris kosong antara header dan
badan, medan nama badan (width-62) dan header (width-59, `Stat` di width-54),
serta tinta header palette8 tanpa bold. Isi kolom status/`Active`/`ID`, gaya
baris terpilih dan warna prompt hapus tidak ikut diubah.

| Tahap | Status | Bukti / kriteria |
|---|---|---|
| K1 · Kontrak dan referensi | ✅ Selesai | Referensi Python v0.21.0 (`ui-3b39bd7`) mematok100×30 dan80×30: indent3, `Stat` width-54, kolom status width-57, baris2/3/4 |
| K2 · RED nyata | ✅ Selesai | [34860668321](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34860668321): gate baru FAIL3× dengan nama tes benar, tanpa error setup |
| K3 · Perubahan minimal | ✅ Selesai | Usulan pertama ditolak clippy (`too_many_arguments 8/7`) dan disimpan; patch12824 byte `006bdcc7…` diterapkan persis |
| K4 · GREEN resmi | ✅ Selesai | [34861285022](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34861285022): gate live3×, clippy `-D warnings` dan seluruh suite PASS |
| K5 · Capture dan review | ✅ Selesai | [Capture34861588181](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34861588181) + [CI34861587979](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34861587979) GREEN; 14 region pixel-identik,2 PNG byte-identik |

Gate live kelima (`picker_column_layout`) kini dijalankan CI biasa bersama posisi,
warna footer, header normal dan header filter. Perbandingan piksel menemukan
temuan terpisah: pesan no-match digambar referensi dengan atribut **dim**;
`pyte0.8.2` tidak menyimpannya, jadi temuan itu dicatat terbuka (1.135 piksel
berbeda di baris tersebut) dan fix-nya menjadi siklus berikutnya.

### Hasil terbaru

- **Lima koreksi picker terverifikasi:** hint hapus, counter, posisi footer,
  warna footer, dan header bantuan normal. Ini bukan persentase seluruh Hermes.
- Header normal4/4 PASS; hanya foreground/bold pada baris1 berubah di normal/delete.
- Enam gambar filter/no-match/empty byte-identik dengan paket sebelumnya.
- Footer color6/6, geometry10/10, counter6/6, hint6/6 tetap PASS.
- 34 tes pendukung PASS. CI biasa wajib menjalankan ketiga regresi CLI picker.
- [Laporan, seluruh gambar dan cara reproduksi](evidence/picker-header-7fef514/REPORT.md).

## Milestone warna footer picker — M1–M5 selesai (ae220ff)

Scope: footer normal/filter/tanpa hasil memakai warna terminal indeks8 seperti
rekaman Python; tetap baris30, counter/hint benar, tidak mewarnai tabel atau
prompt hapus. Referensi memakai **warna abu-abu, bukan atribut ANSI dim**.

| Tahap | Status | Kriteria selesai / hasil |
|---|---|---|
| M1 · Kontrak dan referensi | ✅ Selesai | CLI/PTY nyata disepakati melalui permintaan TDD; enam rekaman referensi memakai palette8 |
| M2 · RED nyata | ✅ Selesai | [34832948842 / 5515789](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34832948842): tes nyata gagal3 kali karena foreground default; build/setup PASS |
| M3 · Perubahan minimal | ✅ Selesai | Patch916 byte diterapkan persis setelah validasi resmi; hanya foreground footer + reset |
| M4 · GREEN resmi | ✅ Selesai | [34833157938 / f633f59](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34833157938): tes primer3, fmt/check/clippy dan workspace PASS |
| M5 · Capture dan review | ✅ Selesai | [Capture34833465322](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34833465322) + [CI34833465347](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34833465347) GREEN; 10 gambar diperiksa; audit warna/posisi/style isolation PASS |

Tes primer membaca state terminal, bukan ejaan escape ANSI. pyte mempunyai dua
representasi untuk palette8 (`brightblack` / `7f7f7f`). Audit xterm membuktikan
Python memakai palette16 dan Rust palette256, **keduanya indeks8, dim=false**.
Encoding tidak disamakan secara paksa: enam area footer pada gambar asli juga
dibandingkan dan pixel-identik. Tidak ada normalisasi warna/geometry.

### Hasil historis slice warna footer

- **Empat milestone perbaikan picker terverifikasi:** hint hapus, counter, posisi,
  dan warna footer. Ini bukan empat dari total seluruh fitur Hermes.
- Hanya foreground baris30 berubah pada enam kasus footer. Baris lain tetap sama.
- Empat gambar delete/empty byte-identik dengan paket sebelumnya: tidak ada
  kebocoran warna footer pada prompt hapus atau pesan empty.
- Counter6/6, hint6/6, geometry10/10, color6/6 PASS;29 tes pendukung PASS.
- Kedua regresi CLI (posisi dan warna) sekarang wajib lolos di CI biasa.
- [Laporan dan seluruh gambar](evidence/picker-color-ae220ff/REPORT.md).

### Slice tinta kolom status picker (fix `1781404`, capture `19bbbd5`)

- Sumber peta warna: sumber upstream terpinned (`hermes_cli/main.py` di `63279301`)
  disimpan bersama provenance di `evidence/upstream-status-attr/`; §F sebelumnya hanya
  menyebut `_status_attr` tanpa pemetaan.
- Lengkap: RED `34870302745` → GREEN `34871381451` → capture `34871743793` →
  CI `34871741338` SUCCESS. Sepuluh checker PASS pada sumber ter-commit.
- Paket [laporan](evidence/picker-status-ink-19bbbd5/REPORT.md):
  `STATUS_TAG_INK_FIXED_ALL_CONTROL_REGIONS_EQUAL` — 34/34 region kontrol identik,
  4 region span tag dicatat sebagai perbedaan yang dinyatakan, delta sel hanya baris 5
  (20 sel: fg default→2 + mode palet256, teks tetap).
- Dua kegagalan jujur (underflow `n - 3` pada patch pertama, geometri unit test pada
  percobaan kedua) dan satu flake start-up PTY tercatat di `attempts-result.txt`.

### Slice cadence redraw picker (fix `bbd943c`, capture `b2db435`)

- Referensi menggambar sekali lalu memblokir di `stdscr.getch()`; port Rust kini
  memakai flag `dirty` sehingga poll 100 ms tetap ada tetapi repaint hanya saat
  layar berubah.
- Lengkap: RED `34873144309` → GREEN `34873480643` (percobaan pertama) →
  capture `34873776470` → CI `34873775366`/`34873776473` SUCCESS.
- Paket [laporan](picker-redraw-on-input-b2db435/REPORT.md):
  `REDRAW_ON_INPUT_FIXED_FRAME_CONTENT_UNCHANGED_ALL_REGIONS_EQUAL` — gate menolak
  8/10 kasus paket sebelumnya dan menerima 10/10 rekaman referensi; kesepuluh cell
  map dan PNG identik byte dengan paket siklus sebelumnya.
- Flake start-up PTY hilang sebagai efek samping; tidak ada lagi kegagalan
  `missing readiness/timeout at stage 0` pada CI sumber tetap.

### Sisa milestone berikutnya

1. Styling header filter/kolom, seleksi, prompt hapus/no-match dan sisa tata letak picker —
   dikerjakan satu perilaku per siklus, bukan satu perubahan besar.
2. Sisa perbedaan wizard/completion dan bukti nested fields/full Python CLI.
3. Review kelengkapan semua area, CI relevan GREEN, lalu permintaan acceptance.

Belum ada implementasi untuk sisa milestone tersebut dalam slice warna footer.

## Verifikasi resmi W5 + kontrol browse (16 September 2026)

Sebelum tanggal ini, baris ringkasan di atas menyebut warna kolom status dan
resize/daftar panjang/clear-filter "belum selesai", dan catatan sesi W5 menyebut
gate-nya "sengaja dibiarkan RED". Keduanya sudah tidak berlaku, dan alasannya
bukan perubahan kode picker:

- **Penyebabnya gerbang CI, bukan picker.** `.github/workflows/ci.yml`
  mematikan job Rust berat dengan `if: false`, sehingga "hijau" hanya berarti
  job workflow ringan lolos. Commit `0286bff` mengaktifkannya kembali
  (`scripts/test_ci_workflow.py::test_heavy_rust_job_is_not_disabled` menjaga
  agar tidak mati lagi).
- **Run [35046192977](https://github.com/Catzpro01/hermes-rust-version/actions/runs/35046192977)**
  (`0286bff`) melaporkan anotasi `fmt=success clippy=success test=success
  picker=success`: seluruh suite workspace **dan** 14 gate PTY/piksel live,
  termasuk `picker-status-tags` (bentuk lifecycle W5) dan
  `picker-browse-control` (lima skenario resize/daftar panjang/clear-filter).
  Jadi opsi C W5 terverifikasi tanpa satu baris pun perubahan classifier.
- **Run [35047313528](https://github.com/Catzpro01/hermes-rust-version/actions/runs/35047313528)**
  (`749d6d6`, onboarding first-run) mengulang hasil yang sama: keempat gate
  `success`, tidak ada regresi picker.
- Decoder suite kedua checker (24 tes) juga dijalankan lokal dengan
  `pyte==0.8.2` + `wcwidth==0.8.3`; hanya capture PTY yang membutuhkan biner.

Yang **tidak** diklaim slice ini: tidak ada bukti piksel baru, tidak ada
perubahan tinta/geometri, konten `Active`/`ID` tetap adaptasi terdokumentasi,
T12/T13 (wizard/completion) tetap terbuka, dan Spec017 belum diterima pengguna.

## Aturan penutupan

- Tes otomatis mendukung bukti visual, tidak menggantikannya.
- Rekaman Python yang dipakai ulang harus disebutkan; bukan capture Python baru.
- Perubahan resize, daftar panjang dan clear-filter belum dibuktikan oleh slice
  redraw-cadence ini; ketiganya kemudian dipatok gate `picker-browse-control`
  (lihat bagian verifikasi 16 September 2026).
- Tidak ada estimasi persen keseluruhan atau tanggal selesai yang belum berdasar.
- **Tidak melakukan merge tanpa instruksi pengguna.** Selesai satu slice tidak
  menutup Spec017 dan tidak berarti pengguna telah memberikan acceptance.

Catatan kronologis: [PROGRESS.md](../../../PROGRESS.md).

Rangkuman kemampuan proyek di luar slice ini: [docs/PARITY.md](../../PARITY.md).
