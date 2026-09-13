# Bukti visual Spec 017 — capture awal dan temuan

**Terbaru: perbaikan spasi ANSI terverifikasi pada `a2a3d08`; parity masih FAIL.**

Lihat [laporan capture ulang](banner-a2a3d08/REPORT.md): RED/GREEN dan CI kode
lulus; kolom 100/80 cocok, tetapi 94/95 dan style masih berbeda. Capture dari
sumber committed sudah menyertakan compiler/Cargo. Bukti lama di bawah tetap
menjadi catatan diagnosis awal, bukan status validasi terbaru.

Kesepakatan Q1–Q8 dan izin eksekusi sudah dikonfirmasi user. Ini hasil tahap
pertama, bukan bukti lengkap kelima area dan bukan persetujuan closure.

## Hasil yang benar-benar dijalankan

- Rust: CLI asli, provider `fake`, PTY 30 baris pada 100/80/94/95 kolom.
  Sumber `814c235fd6391f1173e9d3d7bc6ec879c8bda5fd`; [capture runner
  34779200470](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34779200470)
  berhasil build dan capture.
- Python: fungsi publik upstream `build_welcome_banner`, sumber
  `63279301bcbdc185c1b07b98a9312eb0c862f26d`, v0.21.0. **Capture komponen,
  bukan startup penuh Python CLI.** Model, cwd, session ID, empat nama tool,
  ketersediaan kosong, dan skills kosong diberikan sebagai input eksplisit.
  Mapping fixture tool ke `other` menguji layout, bukan kesetaraan registry Python.
- Referensi diunduh ke cache terisolasi, tidak memakai instalasi/data Python
  user. Semua blob Python cocok persis dengan tree upstream; rincian verifikasi
  dan 12 konversi CRLF PowerShell sesuai `.gitattributes`: [reference.json](reference.json).
- Replay kedua sisi memakai xterm 5.5.0, Chromium 138.0.7204.0, DejaVu Mono,
  ukuran font 16 dan device scale 1. Tidak ada normalisasi isi stream.
  PNG adalah screenshot emulator yang mereplay byte PTY asli, bukan mockup.
- Capture Python awal memuat warning dependensi. Setelah requests/httpx
  dipasang hanya di cache, seluruh capture Python diulang tanpa warning.
  Gambar diagnostik awal yang terkotori itu tidak dipakai dalam paket ini.

## Temuan: spasi ANSI hilang

Writer `write_buffer_ansi` di `crates/hermes-cli/src/tui/welcome.rs`
melewati sel spasi dengan style kosong. Dalam stream terminal, spasi itu
seharusnya tetap memajukan cursor. Akibatnya label, isi kolom, dan border
Rust bergeser meskipun buffer internal cocok dengan referensi plain-text.

Posisi awal `Available Tools` dari buffer emulator (kolom dihitung dari 1):

| Terminal | Python | Rust | Hasil |
|---|---:|---:|---|
| 100×30 | 51 | 2 | FAIL |
| 80×30 | 41 | 2 | FAIL |
| 94×30 | 48 | 2 | FAIL |
| 95×30 | 49 | 2 | FAIL |

Ini bukan adaptasi branding yang diizinkan. Tes buffer dan pemeriksaan
substring warna sebelumnya tidak menangkap kesalahan serialisasi ANSI ini.

### Contoh 80×30 — kiri Python, kanan Rust

![Perbandingan nyata 80×30](banner-814c235/banner-80x30-top.png)

### Gambar lain

| Ukuran | Bagian atas scrollback | Viewport akhir |
|---|---|---|
| 100×30 | [PNG](banner-814c235/banner-100x30-top.png) | [PNG](banner-814c235/banner-100x30-bottom.png) |
| 80×30 | [PNG](banner-814c235/banner-80x30-top.png) | [PNG](banner-814c235/banner-80x30-bottom.png) |
| 94×30 | [PNG](banner-814c235/banner-94x30-top.png) | [PNG](banner-814c235/banner-94x30-bottom.png) |
| 95×30 | [PNG](banner-814c235/banner-95x30-top.png) | [PNG](banner-814c235/banner-95x30-bottom.png) |

Setiap viewport tetap 30 baris. Pada banner panjang, tampilan atas/bawah
scrollback disimpan terpisah; tinggi terminal tidak diubah untuk membuatnya muat.

## Paket bukti dan integritas

Direktori [banner-814c235](banner-814c235/) menyimpan:

- `rust-bundle.json`: byte asli runner, chunk bertimestamp, kondisi PTY,
  fixture, hash binary, source commit, dan batas snapshot.
- `paired-bundle.json`: bundle tadi ditambah rekaman komponen Python lokal.
- `*.ansi`: byte mentah tanpa perubahan; `*.cast`: rekaman asciinema v2 dari
  chunk yang sama, bukan teks UI hasil rekonstruksi.
- `*.png`: pasangan screenshot; `*-cells.json.gz`: buffer emulator termasuk
  posisi, warna, bold/dim/italic, untuk memeriksa diagnosis secara terstruktur.
- `renderer.json` dan `integrity.json`: versi, font/hash, provenance transport,
  script hash, dan checksum file. Asli tidak dinormalisasi atau ditimpa.

Transport Rust: tiga annotation base64/gzip, diverifikasi lengkap, ukuran
60.632 byte dan SHA-256
`311964327069028ef542075a8b9ca76d6b5e9e4f6c43dd90cc637aa86d0204d4`.
Signed artifact ZIP gagal di sandbox; tidak ada bagian hilang yang diabaikan.

## Hambatan dan pekerjaan tersisa

- Validasi kini memakai push yang diizinkan dan rustup resmi; dispatch manual
  403 bukan lagi prasyarat. RED/GREEN serta CI sumber `a2a3d08` sudah berhasil.
- Capture ulang masih menemukan layout session panjang dan style yang tidak
  cocok. Rincian/posisi sel ada di [laporan terbaru](banner-a2a3d08/REPORT.md).
- Wizard, picker, completion, serta variasi summary nol/non-nol belum lengkap.
  Ringkasan awal hanya empat tools / nol skills, tanpa MCP.
- Versi compiler Rust persis belum direkam pada capture awal (workflow memakai
  stable); sudah dicatat pada capture ulang `a2a3d08`. Referensi upstream tidak membuktikan
  tidak adanya modifikasi lokal historis pada VM.
- Semua bukti akhir harus ditinjau user sebelum closure. Tidak ada izin merge.

## Reproduksi

Lihat [instruksi tooling](../../../../scripts/visual-renderer/README.md).
Simpan setiap capture ulang dalam direktori baru; skrip menolak menimpa bukti.
