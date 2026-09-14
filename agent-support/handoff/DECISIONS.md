# Keputusan dan batasan — indeks kesinambungan

Ini ringkasan navigasi, bukan pengganti jawaban pengguna/tiket/laporan. Tujuan
Wayfinder baru: rute penuntasan Spec017, dipilih pengguna sebelum tugas handoff.

## Jangan dibuka ulang sebagai pertanyaan baru

- Python/data asli harus dipertahankan; compatibility-first tetap berlaku.
- Q1: bukti pasangan aktual untuk semua lima area §J.7; tes hanya pendukung.
- 2A/3A: pasangan gambar + rekaman mentah + metadata; normal dan risiko representatif
  dengan dummy fixtures terisolasi, termasuk cakupan langkah wizard yang disepakati.
- 4A/5A/6A: hanya allowlist adaptasi yang sudah ada; original immutable; normalisasi
  dinamis hanya terpisah/tercatat;100×30/80×30 dan threshold banner94/95.
- 7A/8A: perbaikan pada scope UI/direct regression; fitur tertunda terpisah;
  acceptance akhir diberikan pengguna secara eksplisit.
- Pengguna kemudian menyetujui rencana pelaksanaan. Itu tidak menerima bukti yang
  belum ada, tidak menutup Spec017, dan tidak otomatis memberi izin merge.
- Review yang tersedia adalah limited direct Standards/Spec; bukan subagent
  independen atau personal Matt Pocock approval.
- Skill Matt asli sudah dipasang. `/ask-matt` adalah router, `/wayfinder` planning,
  `/tdd` satu perilaku pada seam yang disepakati, RED sebelum minimal GREEN.
- Selalu commit/push kemajuan kecil yang bermakna; sekarang diminta mekanisme
  auto-push setelah commit. Jangan auto-stage semua file atau mengirim secrets.

## Referensi detail

- [Grilling scope](../../.scratch/hermes-rs-total-parity/grilling.md)
- [Closure review](../../.scratch/hermes-rs-total-parity/issues/T11-closure-review.md)
- [Evidence requirements](../../.scratch/hermes-rs-total-parity/issues/T12-visual-evidence.md)
- [Remaining differences](../../.scratch/hermes-rs-total-parity/issues/T13-visual-differences.md)
- [Integration/skill limitations](../guidance/matt-pocock-skills.md)
- [Historical progress](../memory/PROGRESS.md)

## Nuansa bukti warna

Footer: palette8, dim=false. Header normal: palette3, bold=true, dim=false.
Python palette16 dan Rust palette256 boleh memilih indeks yang sama; perbedaan
mode encoding dicatat, bukan dihapus. Pyte memberi alias berbeda; audit xterm
memeriksa mode/index, atribut dan pixel. Ini bukan izin normalisasi warna/geometri
atau penggunaan truecolor sembarang. Baca laporan asli sebelum menulis regresi.

## Permintaan main terbaru

Pengguna meminta seluruh progres disimpan ke main. Pelaksanaan tidak mungkin
pada sesi sumber yang branch-nya dikunci. Permintaan itu dicatat sebagai target
integrasi, bukan diam-diam dilaksanakan, bukan waiver CI/Spec017, dan bukan izin
untuk agent berikutnya melanggar pembatasan platformnya sendiri.

## Instruksi integrasi terbaru

Pengguna secara eksplisit meminta merge PR setelah meminta penyimpanan di main.
Instruksi pengguna untuk merge kini ada, tetapi tidak mengatasi batas platform
sesi sumber atau konflik PR. [Status dan langkah integrasi](MAIN-INTEGRATION.md)
adalah sumber detail operasional; Spec017 tetap WIP, belum diterima/ditutup.
