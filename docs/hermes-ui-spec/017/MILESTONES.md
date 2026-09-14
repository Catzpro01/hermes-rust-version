# Milestone Hermes — visual parity Spec017

**Pembaruan: 14 September 2026 · Header bantuan mode normal dan header filter picker selesai diverifikasi.**

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
| Seleksi, konfirmasi hapus, header kolom, sisa tata letak picker | ⏳ Belum selesai | Siklus terpisah; tidak ikut dianggap selesai oleh header filter |
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

### Sisa milestone berikutnya

1. Styling header filter/kolom, seleksi, prompt hapus/no-match dan sisa tata letak picker —
   dikerjakan satu perilaku per siklus, bukan satu perubahan besar.
2. Sisa perbedaan wizard/completion dan bukti nested fields/full Python CLI.
3. Review kelengkapan semua area, CI relevan GREEN, lalu permintaan acceptance.

Belum ada implementasi untuk sisa milestone tersebut dalam slice warna footer.

## Aturan penutupan

- Tes otomatis mendukung bukti visual, tidak menggantikannya.
- Rekaman Python yang dipakai ulang harus disebutkan; bukan capture Python baru.
- Perubahan resize, daftar panjang dan clear-filter belum dibuktikan oleh slice ini.
- Tidak ada estimasi persen keseluruhan atau tanggal selesai yang belum berdasar.
- **Tidak melakukan merge tanpa instruksi pengguna.** Selesai satu slice tidak
  menutup Spec017 dan tidak berarti pengguna telah memberikan acceptance.

Catatan kronologis: [PROGRESS.md](../../../PROGRESS.md).

Rangkuman kemampuan proyek di luar slice ini: [docs/PARITY.md](../../PARITY.md).
