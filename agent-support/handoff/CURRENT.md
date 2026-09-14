# Current handoff — tujuan berikutnya sudah dipilih

## Tindak lanjut — riwayat disambungkan pada branch PR

Penyebab konflik PR #7 adalah unrelated histories. Main8b6a673 dan root source
6ded9dd dibandingkan: hanya empat perubahan dokumentasi T10, tanpa delta runtime
main yang terpisah. Riwayat disambungkan pada branch PR dengan seluruh tree source
sebelumnya dipertahankan identik. Tidak ada produk/evidence/vendor yang ditimpa.
Lihat [rekonsiliasi](MAIN-INTEGRATION.md); cek PR/CI hasil commit terbaru.
**Main belum diubah; merge akhir PR tetap memerlukan konteks berwenang.**
Tugas substantif setelah integrasi tetap Wayfinder dengan tujuan Spec017 terpilih.

## Pemeriksaan historis — handoff tersimpan, merge masih terblokir

Paket penataan sudah ada pada `b5facfc`, hasilnya dicatat pada `95f9319`.
Pemeriksaan ulang: verifier7 alias/37 skill/164 file PASS dan8 tes auto-push PASS;
auto-push aktif, receipt95f9319 sukses dan sesuai head saat diperiksa.
CI34843728495/95f9319 SUCCESS. Pengguna kini meminta merge secara eksplisit,
tetapi PR #7 masih draft/CONFLICTING dan sesi sumber tidak boleh menulis main.
**Belum merged.** Baca [status/jalur main](MAIN-INTEGRATION.md) sebelum integrasi.
Tugas berikutnya setelah handoff tetap wawancara Wayfinder, bukan coding baru.

## Tugas aktif (bukan implementasi)

Pengguna menjalankan `/wayfinder`, kemudian memilih **rute penuntasan Spec017**
pada pertanyaan tujuan. Pilihan ini sudah dikonfirmasi melalui UI; **jangan tanya
ulang apakah scope-nya seluruh Hermes atau hanya picker**.

Setelah itu pengguna meminta seluruh progres/ingatan/skill/konteks ditata dan
hand-off ke sesi lain, auto-push checkpoint kecil, serta penyimpanan di main.
Pekerjaan terakhir adalah **penataan dan handoff ini**. Wayfinder dijeda untuknya.

**Langkah substantif berikutnya:** lanjut wawancara breadth-first untuk menemukan
keputusan yang benar-benar belum jelas menuju penuntasan Spec017, lalu chart peta
keputusan di tracker Markdown. Jangan langsung coding header filter. Jangan
menyalin backlog implementasi menjadi tiket keputusan.

Belum ada peta Wayfinder atau child decision ticket. Belum ada tiket keputusan
yang di-claim/di-resolve. Tujuan peta adalah menghasilkan rute yang jelas, bukan
mengimplementasikan/menutup Spec017 di dalam peta. Read original
[Wayfinder](../skills/wayfinder/SKILL.md), [grilling](../skills/grilling/SKILL.md),
[domain-modeling](../skills/domain-modeling/SKILL.md), dan
[tracker](../guidance/issue-tracker.md). Tracker lokal belum mendokumentasikan
operasi Wayfinder; gunakan konvensi lokal yang dijelaskan dalam panduan sesi
berikutnya, bukan migrasi ke tracker lain diam-diam.

## Paket handoff dan checkpoint

Penataan `agent-support/` sudah dilengkapi panduan bootstrap/keputusan/verifikasi.
Auto-push **setelah commit** telah diinstal di clone sumber dan lolos8 tes offline;
hook/config lokal tidak ikut clone. Sesi baru harus mengaktifkan secara eksplisit
sesuai [panduan](START-NEXT-SESSION.md).42 tes pendukung baru PASS;164 file upstream,
37 skill links dan bukti produk tetap utuh. Lihat [hasil](VERIFICATION.md) serta
entri progress terbaru untuk commit/push/CI aktual. Tidak ada watcher/auto-stage.
Checkpoint paket **b5facfc** terbukti auto-push dan SHA remote cocok;
[CI34843454979](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34843454979)
SUCCESS termasuk tes handoff, fmt/clippy/full suite dan actual picker regressions.
Clone baru juga lolos verifier/8 tes; hook/config memang perlu bootstrap.
[Draft PR #7](https://github.com/Catzpro01/hermes-rust-version/pull/7) menjadi jalur
review main, bukan merge/acceptance. Catatan hasil ini disimpan sesudah checkpoint
paket; cek `git log`/Actions untuk commit catatan terbaru.

## Status produk terverifikasi

Lima koreksi picker telah selesai diverifikasi:

1. Hint hapus sesuai filter — runtimec07f0c5.
2. Counter tanpa hasil mempertahankan total — runtimea8d5e8c.
3. Footer pada baris terakhir — runtime0c0704d.
4. Warna footer palette8 — runtimeae220ff.
5. Header bantuan mode normal palette3 + bold — **runtime7fef514**.

[Laporan terakhir](../../docs/hermes-ui-spec/017/evidence/picker-header-7fef514/REPORT.md):
10 pasangan gambar diperiksa langsung;4 header pixel-identik;6 gambar filter/
no-match/empty identik dengan paket sebelumnya.34 tes pendukung PASS saat delivery.
Capture [34837424495](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34837424495)
dan CI [34837424464](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34837424464)
SUCCESS pada source7fef514. Ini hasil historis source tersebut, bukan otomatis
hasil CI untuk semua commit setelahnya. Tiga regresi CLI nyata wajib di CI.

**Belum selesai:** header filter/kolom, selected-row, warna delete/no-match,
sisa geometry, wizard/completion, dan T12 bukti nested fields/full Python CLI.
Resize/long-list/clear-filter tidak dibuktikan oleh fixture tetap ini. Tidak ada
whole-picker PASS, acceptance akhir, penutupan Spec017, atau merge.

## Sumber kebenaran dan keputusan

- [Milestones](../../docs/hermes-ui-spec/017/MILESTONES.md): kemajuan visual terkini.
- [T12 — Visual evidence](../../.scratch/hermes-rs-total-parity/issues/T12-visual-evidence.md).
- [T13 — Visual differences](../../.scratch/hermes-rs-total-parity/issues/T13-visual-differences.md).
- [Keputusan/constraint](DECISIONS.md): indeks ke detail yang sudah disepakati.
- [Verifikasi/reproduksi](VERIFICATION.md): hasil dan cara mengecek kembali.
- [Memory](../memory/MEMORY.md) / [Progress](../memory/PROGRESS.md): riwayat penuh.

## Branch dan main

> **Koreksi 2026-09-14 (sesi `arena/01a0a052`).** Paragraf historis di bawah
> sudah **usang**: PR #7 ternyata **MERGED** ke `main` oleh `Catzpro01` pada
> 2026-09-14T14:28:06Z (merge commit `baa7158`, 1.691 file, +143.671/−954), dan
> CI `main` run `34855804061` pada `baa7158` **SUCCESS** dengan 584 tes Rust
> lulus. Sesi baru karena itu boleh memakai `main` sebagai basis bila platform
> memberikannya; analisis lengkap: [PROGRESS-ANALYSIS](PROGRESS-ANALYSIS.md).
> Larangan merge tanpa instruksi eksplisit dan larangan menyamakan permintaan
> menyimpan progres dengan acceptance visual tetap berlaku.

Sesi sumber terikat pada `arena/01a09c1e-hermes-rust-version`. Checkpoint sebelum
penataan adalah478570080f0064beab25885d2b3e500161120cc5; kode runtime7fef514 tidak
diubah oleh tugas handoff. Ambil commit penataan terbaru dengan `git log -1`.

Pengguna memang meminta main sekarang, tetapi aturan platform sesi ini hanya
mengizinkan commit/push ke branch sumber. **Main belum diperbarui oleh tugas ini.**
Integrasi main harus dilakukan melalui konteks yang berwenang dan gate review;
jangan menyamakan permintaan menyimpan progres dengan acceptance visual.
Sesi baru harus mematuhi branch yang ditugaskan platform kepadanya, bukan membabi
buta memakai nama branch historis atau berpindah ke main.

## Kondisi lingkungan / jebakan

- Tidak ada cargo lokal pada sesi ini. Gunakan runner resmi dan exact tested
  patch sebelum menerapkan Rust; jangan klaim tes Rust lokal.
- Skill adalah file vendored asli, bukan koneksi ke Matt atau jaminan native
  Skill/subagent tool. Jangan mengklaim review independen jika tool tidak ada.
- Cache/dependency renderer mungkin hilang pada sesi baru. Jangan arsipkan build
  besar; install pinned dependencies bila diperlukan, lihat VERIFICATION.
- Signed log/artifact URL pernah gagal; annotation transport dengan hash terbukti
  bekerja. Jangan cetak URL bercredential atau meminta token.
- Saat masuk tugas handoff, HEAD/index sempat kembali ke6ded9dd sementara file
  kerja tetap lengkap. Fetch branch sumber menemukan4785700; semua1830 blob/mode
  cocok, tanpa file ekstra. Baru setelah itu metadata diselaraskan tanpa menulis
  file kerja. **Jangan blind reset untuk mengatasi status kotor**; verifikasi dulu.
