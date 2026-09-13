# T11 — Review closure Spec 017 dan gate CI

- Tanggal: 2026-09-14
- Status: IN REVIEW — perbaikan di GitHub, CI hijau; bukti §J.7/sign-off terbuka
- Label: `ready-for-human`
- Owner: Arena agent (review/perbaikan); human reviewer (keputusan §J.7)
- Bergantung pada: T10, akses toolchain Rust untuk verifikasi, bukti §J.7
- Batasan: merge hanya atas instruksi eksplisit; instalasi Python tidak diubah.
- Pembaruan user: commit/push progres diwajibkan; ikuti `AGENTS.md`,
  `MEMORY.md`, dan `PROGRESS.md` untuk hasil checkpoint terbaru.

## Putusan

**Belum layak sign-off closure.** Implementasi yang sudah selesai tidak
dibatalkan, tetapi klaim bukti perlu dibatasi. Review ini memperbaiki CI
serta dokumen, lalu menerapkan formatting mekanis Rust dari runner; tidak
mengubah perilaku runtime Rust secara manual atau mengerjakan
fitur lanjutan T10. Tidak ada persetujuan Matt/human yang diasumsikan.

## Temuan dan tindakan

### P1 — CI hijau tidak menjamin format bersih (diperbaiki)

`.github/workflows/ci.yml` sebelumnya menjalankan format dengan `|| true`
dan gate akhir hanya membaca clippy/test. Akibatnya kegagalan rustfmt
tersembunyi; step dan job masih bisa hijau. Klaim T10/ROADMAP bahwa format
bersih tidak bisa diturunkan dari warna check tersebut.

Perbaikan:
- Step format diberi `id: fmt`, `pipefail`, tanpa `|| true`.
- `continue-on-error` dipertahankan agar tes dan unggahan diagnostik tetap
  berjalan; gate akhir memeriksa **outcome** format, clippy, dan test.
- `always()` pada gate mencegah gate terlewati karena step sebelumnya gagal.
- Anotasi diagnostik melaporkan `fmt` dan pesan bila log format tidak ada.
- Tes regresi: `scripts/test_ci_workflow.py` (PyYAML, hanya dependensi QA).

**Pembaruan:** gate baru memang menemukan drift di 52 file Rust.
Formatting telah diterapkan dari patch runner yang lolos fmt/check,
dengan checksum dan byte-diff diverifikasi lokal (rincian di `PROGRESS.md`).
CI pada commit hasil formatting `3e0e8d9` sudah **SUCCESS**:
[run 34776992554](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34776992554),
`fmt=success clippy=success test=success`, 576 tes Rust + 5 tes QA lulus.
Verifikasi ini remote, bukan eksekusi Rust lokal.

### P2 — Penggantian bukti §J.7 belum punya persetujuan (tetap terbuka)

`docs/HERMES_UI_SPEC.md` §J.7 mewajibkan side-by-side capture banner,
langkah wizard, picker, completion dropdown, dan summary. T10 sebelumnya
menyatakan tes otomatis menggantikannya sebagai bukti yang lebih kuat.
Tidak ditemukan persetujuan penggantian itu dalam catatan yang ditinjau.

Bukti yang memang tersedia:
- `welcome.rs`: sembilan referensi banner plain-text, style dibuang dan
  trailing whitespace di-trim; branding Rust adalah adaptasi eksplisit.
- `banner_e2e.rs`: substring/kode warna terpilih melalui PTY nyata.
- `wizard_e2e.rs` dan `session_picker_e2e.rs`: interaksi serta string
  terpilih, bukan perbandingan seluruh frame dengan Python.
- `completion.rs`: cross-check katalog dan perilaku completion murni,
  bukan capture dropdown terminal berdampingan.

Tes itu berguna, tetapi bukan bukti seluruh layar/ANSI-stream identik.
T10, board, ROADMAP, dan PARITY kini membedakan implementasi selesai dari
closure review. Pada ronde 1 `/grill-with-docs` (2026-09-14), user memilih A:
pertahankan capture nyata §J.7 dengan tes sebagai pendamping, bukan pengganti.
User kemudian mengonfirmasi `2A, 3A`: gambar berdampingan + rekaman terminal
mentah + metadata; jalur normal dan kondisi berisiko yang representatif.
Aturan perbandingan dan kriteria penerimaan masih dibahas di `../grilling.md`;
belum ada persetujuan closure atau implementasi.

### P2 — Laporan parity memuat informasi yang bertentangan (diperbaiki)

- FTS5/search dan function calling bukan lagi fitur yang belum tersedia;
  implementasi dan tes ada sejak Spec 004/002.
- Sandbox CLI default **on**, sesuai amendment ADR 0006; opt-out eksplisit
  `--no-sandbox` atau `sandbox.enabled: false`. Default konstruktor library
  tetap berbeda dan tidak diubah oleh review ini.
- `COMMAND_REGISTRY` berisi **101** entri verbatim; `RS_EXTENSIONS` berisi
  **14** entri tambahan. Klaim 87 + 14 salah.
- Path referensi ROADMAP diperbaiki menjadi
  `docs/hermes-ui-spec/017/verbatim/` (11 file ada di checkout).
- `/sessions` REPL membuka picker bila stdin/stdout TTY, bukan selalu list.
- Katalog 39 provider tidak sama dengan 39 adapter yang berfungsi.
- Deskripsi bukti banner dibatasi pada referensi ternormalisasi dan tes PTY.

### P2 — Bukti "Python untouched" lebih sempit dari klaim (diklarifikasi)

`smoke_python_hermes_untouched` hanya membandingkan mtime
`$HOME/.hermes/state.db` bila home ada; tes bisa return awal. Itu bukan
hash/audit seluruh instalasi Python. T10 sekarang mencatat batas bukti ini.
Review ini sendiri tidak menjalankan atau mengubah instalasi Python.

## Verifikasi 2026-09-14

### Lokal — benar-benar dijalankan

- `python3 scripts/test_ci_workflow.py`: **3 tests OK** dengan PyYAML 6.0.3.
  Cakupan: pipeline format dengan fake cargo exit 0/1/42 tetap meneruskan
  status dan menyimpan log; gate pada 125 kombinasi success/failure/
  cancelled/skipped/kosong; diagnostik format success/failure/tanpa log.
  Ini tes workflow, **bukan** eksekusi rustfmt atau GitHub Actions runner.
- YAML berhasil diparse; `git diff --check` bersih.
- Reproduksi pipeline format lama: fake cargo exit 1 menjadi exit 0;
  pipeline baru mengembalikan exit 1.

Untuk mengulang tes workflow di lingkungan dengan PyYAML:

```bash
python3 scripts/test_ci_workflow.py
```

### Historis — diverifikasi ulang lewat GitHub, bukan run baru

Job [103764353101](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34772434469/job/103764353101),
PR #6, 2026-09-13: anotasi `summary` melaporkan `clippy=success test=success`;
penjumlahan hasil suite = **576 passed, 0 failed**. Check diambil secara
read-only melalui `gh api`. Pengambilan raw log gagal (EOF); status format
historis tidak dinyatakan bersih karena workflow lama menelan kegagalannya.

### Terhambat pada review awal (hasil terbaru ada di `PROGRESS.md`)

- `cargo fmt --all -- --check`, `cargo check`, clippy, dan tes workspace
  lokal: `cargo`/`rustc` tidak terpasang. Akses `sh.rustup.rs` dan
  `static.rust-lang.org` gagal `SSL_ERROR_SYSCALL`; tidak ada toolchain
  yang berhasil dipasang.
- Pada review awal belum ada push/CI baru. Checkpoint berikutnya harus
  di-commit/push sesuai instruksi user; catat hasil aktual di `PROGRESS.md`.
- Capture visual Python/Rust lengkap §J.7 belum dibuat.

## Acceptance criteria / blocking edges

- [x] Hilangkan format failure yang tersembunyi dan uji gate lokal.
- [x] Koreksi klaim dokumen dengan referensi kode/bukti yang tersedia.
- [x] Pisahkan verifikasi historis dari verifikasi lokal.
- [x] Jalankan fmt/check/clippy/test workspace dengan Rust stable; tangani
  kegagalan yang ditemukan, lalu dapatkan hasil gate CI baru (runner fmt/check
  sebelum commit; run `34776992554` pada commit formatting sukses).
- [ ] Lengkapi §J.7 atau dapatkan persetujuan eksplisit atas bukti alternatif.
- [ ] Human review/sign-off closure T10.
- [ ] Merge hanya jika diperintahkan user; bukan tindakan otomatis tiket ini.
