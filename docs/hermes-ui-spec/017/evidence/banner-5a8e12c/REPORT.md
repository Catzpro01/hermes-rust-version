# Banner setelah koreksi centering — 2026-09-14

**Hasil: BANNER_FIXTURE_MATCH_WITH_DOCUMENTED_BRANDING_NOT_CLOSED.**
Empat pasangan fixture banner yang direkam cocok pada pemeriksaan di bawah,
selain label branding Hermes Agent → Hermes-RS yang sudah didokumentasikan.
Ini bukan klaim raw ANSI/pixel identity, seluruh state banner, lima area UI,
review independen, atau acceptance user.

## Sumber dan validasi

- Rust aktual `5a8e12c97dd760bf05f411a5d59585a97f0b5ad6`, bukan candidate overlay.
  [CI 34785921216](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34785921216)
  dan [capture 34785921221](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34785921221)
  SUCCESS. Hash welcome.rs lokal cocok; hash diff Rust adalah hash byte kosong.
- rustc/Cargo 1.98.1; versi lengkap di bundle/integrity.json.
- Python upstream `63279301bcbdc185c1b07b98a9312eb0c862f26d`, v0.21.0:
  fungsi display publik, **bukan startup penuh CLI Python**. Rust adalah REPL
  offline asli di PTY. Fixture fake provider, empat tools/nol skills; session
  UUID dan cwd Rust diberikan sebagai input Python, bukan diganti dalam output.
- RED centering 34785680910; GREEN 34785789939. Lebar centering menggunakan
  teks terlihat tanpa separator wrap akhir; tidak mengubah expected Python.
  Patch 3644 byte SHA-256
  `d306f7bb2d8861e1c1a50dc6cc1e2f2871f5b4a9c11539058e4c6119652095c8`
  diterapkan persis setelah fmt/check, regresi lama/baru, clippy dan tes workspace.
- [Review langsung terbatas](../review-059cd65.md), termasuk addendum centering.

## Pemeriksaan gambar dan sel

Kedelapan PNG atas/bawah telah dibuka dan diperiksa langsung. Posisi label,
wrap, caduceus, border, bold/dim, dan tanda baca diperiksa bersama dump sel.

| Terminal | Tools Python/Rust | Session Python/Rust | Baris Python/Rust | Beda atribut glyph sama |
|---|---|---|---|---|
| 100×30 | 51/51 | 4/4 | 33/33 | 0 |
| 80×30 | 41/41 | 17/17 | 30/30 | 0 |
| 94×30 | 48/48 | 21/21 | 30/30 | 0 |
| 95×30 | 49/49 | 21/21 | 38/38 | 0 |

Di luar baris judul branding, seluruh koordinat glyph non-spasi cocok.
Background, mode background, inverse dan underline cocok pada semua sel,
termasuk sel kosong. Atribut penuh diperbandingkan untuk glyph non-whitespace
sama pada koordinat sama; statistik ini sendiri bukan bukti geometri.
Pemeriksaan geometri terpisah menyertakan U+2800 dan mengecualikan hanya
space/empty cell yang tidak membawa glyph. Semua perbedaan teks mentah,
termasuk padding/branding, tetap tercatat di comparison.json; tidak ada
normalisasi atau modifikasi rekaman/gambar.

Defek Session kolom 21/20 pada bukti b56c9a3 telah hilang. Paket historis itu
tetap FAIL dan tidak ditimpa atau diberi label ulang.

| Ukuran | Atas scrollback | Viewport akhir |
|---|---|---|
| 100×30 | [PNG](banner-100x30-top.png) | [PNG](banner-100x30-bottom.png) |
| 80×30 | [PNG](banner-80x30-top.png) | [PNG](banner-80x30-bottom.png) |
| 94×30 | [PNG](banner-94x30-top.png) | [PNG](banner-94x30-bottom.png) |
| 95×30 | [PNG](banner-95x30-top.png) | [PNG](banner-95x30-bottom.png) |

## Integritas dan reproduksi

Bundle asli 81053 byte, SHA-256
`705a6aee68b0a346e8f696a06bbe5b4f2af562220b7dca3ad4e95afbe24bbf93`.
Delapan raw/event/cast round trip terverifikasi. PNG mereplay byte asli melalui
xterm 5.5.0, Playwright 1.55.0, Chromium 138.0.7204.0, DejaVu Mono 5.2.5,
16px/scale 1; TERM xterm-256color, COLORTERM truecolor, locale C.UTF-8,
30 baris tetap. Versi/hash/provenance lengkap ada di renderer.json,
integrity.json, rust-bundle.json dan paired-bundle.json.

```sh
PYTHONPATH=/home/user/.cache/hermes-visual-python python3 scripts/capture_banner.py pair \
  RUST_BUNDLE REFERENCE_CHECKOUT NEW_PAIRED_JSON
NODE_PATH=/home/user/.cache/hermes-visual-renderer/node_modules \
LD_LIBRARY_PATH=/home/user/.cache/hermes-visual-renderer/libraries/lib \
  node scripts/visual-renderer/render.cjs NEW_PAIRED_JSON NEW_DIRECTORY
```

Gunakan direktori baru; jangan menimpa paket ini. Wizard, picker, completion,
dan summary nol/non-nol masih memerlukan bukti berpasangan. Tidak ada merge.
