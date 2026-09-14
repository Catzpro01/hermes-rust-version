# Milestone Hermes — visual parity Spec017

**Pembaruan: 14 September 2026 · Aktif: header bantuan picker mode normal.**

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
| Header bantuan mode normal | 🔄 TDD dimulai | Referensi palette3 + bold; bukan header kolom atau header filter |
| Header filter, seleksi, konfirmasi hapus, sisa tata letak picker | ⏳ Belum selesai | Siklus terpisah; tidak ikut dianggap selesai oleh warna footer |
| Sisa perbedaan wizard/completion dan kelengkapan bukti | ⏳ Belum selesai | T12/T13 tetap terbuka |
| Penerimaan akhir Spec017 | 🔒 Belum siap | Memerlukan bukti lengkap, CI relevan GREEN, dan persetujuan eksplisit pengguna |

## Milestone aktif — header bantuan mode normal

Scope: baris bantuan `Browse sessions` ketika filter kosong, **palette3 + bold**.
Header yang sama juga tampak saat konfirmasi hapus. Warna prompt hapus sendiri,
header filter/kolom, seleksi, footer dan posisi teks tidak ikut diubah.

| Tahap | Status | Bukti / kriteria |
|---|---|---|
| H1 · Kontrak dan referensi | ✅ Selesai | CLI/PTY nyata normal100×30 disepakati; referensi Python4/4 normal/delete pada100/80 memakai palette3 + bold |
| H2 · RED nyata | ✅ Selesai | [34836863589 / 4aa0858](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34836863589): tes primer FAIL3, default/tidak bold; fmt/check/build PASS |
| H3 · Perubahan minimal | 🔄 Kandidat | DarkYellow + Bold untuk header ketika filter kosong, lalu reset; source belum diterapkan |
| H4 · GREEN resmi | 🔄 Menunggu runner | Tes primer, fmt/check/clippy/full workspace sebelum patch diterapkan |
| H5 · Capture dan review | ⏳ Menunggu H4 | 10 pasangan normal/filter/no-match/delete/empty; style header dan isolasi perubahan, semua perbaikan footer tetap benar |

## Milestone warna footer picker — M1–M5 selesai

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

### Hasil yang bisa dicek

- **Empat milestone perbaikan picker terverifikasi:** hint hapus, counter, posisi,
  dan warna footer. Ini bukan empat dari total seluruh fitur Hermes.
- Hanya foreground baris30 berubah pada enam kasus footer. Baris lain tetap sama.
- Empat gambar delete/empty byte-identik dengan paket sebelumnya: tidak ada
  kebocoran warna footer pada prompt hapus atau pesan empty.
- Counter6/6, hint6/6, geometry10/10, color6/6 PASS;29 tes pendukung PASS.
- Kedua regresi CLI (posisi dan warna) sekarang wajib lolos di CI biasa.
- [Laporan dan seluruh gambar](evidence/picker-color-ae220ff/REPORT.md).

### Sisa milestone berikutnya

1. Styling header, seleksi, prompt hapus/no-match dan sisa tata letak picker —
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
