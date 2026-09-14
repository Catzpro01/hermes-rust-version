# Milestone Hermes — visual parity Spec017

**Pembaruan: 14 September 2026 · Fokus aktif: warna footer picker.**

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
| Warna footer sesuai Python | 🔄 Sedang dikerjakan | TDD CLI/PTY nyata; rincian tahap di bawah |
| Header, seleksi, konfirmasi hapus, sisa tata letak picker | ⏳ Belum selesai | Siklus terpisah; tidak ikut dianggap selesai oleh warna footer |
| Sisa perbedaan wizard/completion dan kelengkapan bukti | ⏳ Belum selesai | T12/T13 tetap terbuka |
| Penerimaan akhir Spec017 | 🔒 Belum siap | Memerlukan bukti lengkap, CI relevan GREEN, dan persetujuan eksplisit pengguna |

## Milestone aktif — warna footer picker

Scope: footer normal/filter/tanpa hasil memakai warna terminal indeks8 seperti
rekaman Python; tetap baris30, counter/hint benar, tidak mewarnai tabel atau
prompt hapus. Referensi memakai **warna abu-abu, bukan atribut ANSI dim**.

| Tahap | Status | Kriteria selesai / hasil |
|---|---|---|
| M1 · Kontrak dan referensi | ✅ Selesai | CLI/PTY nyata disepakati melalui permintaan TDD; enam rekaman referensi memakai palette8 |
| M2 · RED nyata | ✅ Selesai | [34832948842 / 5515789](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34832948842): tes nyata gagal3 kali karena foreground default; build/setup PASS |
| M3 · Perubahan minimal | ✅ Selesai | Patch916 byte diterapkan persis setelah validasi resmi; hanya foreground footer + reset |
| M4 · GREEN resmi | ✅ Selesai | [34833157938 / f633f59](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34833157938): tes primer3, fmt/check/clippy dan workspace PASS |
| M5 · Capture dan review | 🔄 Sedang berjalan | Menunggu capture source committed dan CI; kemudian audit mode/index8, 10 kasus di100×30/80×30, style isolation dan gambar pasangan |

Tes primer membaca state terminal, bukan ejaan escape ANSI. pyte mempunyai dua
representasi untuk palette8 (`brightblack` / `7f7f7f`); audit xterm akhir wajib
memastikan mode/index warna serta dim=false. Tidak ada perubahan/normalisasi
warna pada rekaman atau gambar asli.

## Aturan penutupan

- Tes otomatis mendukung bukti visual, tidak menggantikannya.
- Rekaman Python yang dipakai ulang harus disebutkan; bukan capture Python baru.
- Perubahan resize, daftar panjang dan clear-filter belum dibuktikan oleh slice ini.
- Tidak ada estimasi persen keseluruhan atau tanggal selesai yang belum berdasar.
- **Tidak melakukan merge tanpa instruksi pengguna.** Selesai satu slice tidak
  menutup Spec017 dan tidak berarti pengguna telah memberikan acceptance.

Catatan kronologis: [PROGRESS.md](../../../PROGRESS.md).
