# Review dua siklus picker terakhir (10 & 11)

Dokumen ini untuk peninjauan manusia. Isinya ringkas, tanpa klaim yang tidak ada
buktinya. Sumber rinci: `picker-status-ink-19bbbd5/REPORT.md` dan
`picker-redraw-on-input-b2db435/REPORT.md` di direktori yang sama.

## 1. Siklus 10 — tinta kolom status (`1781404` / capture `19bbbd5`)

**Yang berubah.** Baris yang bukan kursor kini menggambar tag status 5 sel dengan
tinta sesuai pemetaan upstream: `done`/complete → pair1 hijau, `intr`/interrupted →
pair2 kuning, `err`/error → pair5 merah, `empty` → pair4 (palette8), tag lain →
A_NORMAL. Posisi `3 + name_width + 2` (kolom 43…48 @100, 25…30 @80), tidak pernah
bold, tidak pernah di baris kursor (baris kursor tetap palette2 + bold).

**Mengapa butuh siklus ini.** §F menyebut `_status_attr` tanpa memetakan paletnya.
Pemetaannya dibaca dari sumber upstream yang dipatok dan **disimpan sebagai bukti**
di `evidence/upstream-status-attr/` (file `hermes_cli/main.py` commit `63279301`,
blob `8281cbdd…`, sha256 `89cde75d…`) beserta provenance. Capture lama menguatkan:
`intr` terukur di slot 3, prompt delete di slot 1.

**Rantai bukti.** RED `34870302745` (gagal 3×, tanpa error setup) → GREEN
`34871381451` (patch 6.007 byte `e88347c8…`; hasil ekspor 6.109 byte `73d0322e…`,
bedanya hanya pembungkusan `cargo fmt`) → capture `34871743793` (10 kasus,
**10 checker PASS** di sumber ter-commit) → CI `34871741338` SUCCESS.

**Hasil audit** (`STATUS_TAG_INK_FIXED_ALL_CONTROL_REGIONS_EQUAL`): 20 round trip
raw/cast, 10 hash PNG, 14 jalan checker, **34/34 region kontrol identik**, 6 PNG
byte-identik, delta sel hanya baris 5 pada normal/delete di kedua lebar (20 sel:
`fg default→2`, mode `palette256`; teks tidak berubah), sel & rekaman Python tidak
berubah. Empat region span tag dicatat sebagai **perbedaan yang dinyatakan** (270
piksel masing-masing) karena kata tag adalah data fixture — tintanya dipatok
checker dua sisi, bukan oleh piksel.

**Kegagalan jujur yang tersimpan** (`attempts-result.txt`): patch pertama panik
`attempt to subtract with overflow` (`n − 3` di baris pemisah; teks panik hanya ada
di byte PTY dan dipulihkan dari anotasi trace `bc0b233a…`), lalu unit test di
percobaan kedua mencampur geometri 20 kolom dengan offset 100 kolom.

## 2. Siklus 11 — cadence redraw (`bbd943c` / capture `b2db435`)

**Yang berubah.** Picker menggambar **hanya ketika layar berubah** (flag `dirty`
dinyalakan oleh setiap tombol yang diterima dan setiap resize); poll 100 ms tetap
ada supaya sinyal tetap dilayani. Alasannya bukan estetika: referensi menggambar di
awal loop lalu **memblokir di `stdscr.getch()`** — satu gambar per tombol, tidak ada
output selama menunggu.

**Angka sebelum → sesudah** (frame per jendela input): normal 5 → 1 tanpa input apa
pun; no-match 25 lalu 9 → 1 lalu 4; filter 80 kolom 41 lalu 8 → 1 lalu 5. Ukuran
rekaman ikut turun, mis. no-match 13.815 → 1.463 byte.

**Rantai bukti.** RED `34873144309` → GREEN `34873480643` (**percobaan pertama
berhasil**; patch 2.801 byte `925b963e…`, ekspor identik byte dengan yang diikat)
→ capture `34873776470` (10 kasus, **11 checker PASS**) → CI `34873775366` dan
`34873776473` SUCCESS. Jejak RED 279.052 byte jadi 45.748 byte di GREEN.

**Hasil audit** (`REDRAW_ON_INPUT_FIXED_FRAME_CONTENT_UNCHANGED_ALL_REGIONS_EQUAL`):
16 jalan checker, 34/34 region kontrol identik, 4 perbedaan yang dinyatakan tetap
270 piksel, dan — yang paling penting untuk Anda periksa — **kesepuluh cell map dan
kesepuluh PNG identik byte** dengan paket siklus sebelumnya. Siklus ini mengubah
*kapan* menggambar, bukan *apa* yang digambar.

**Gate yang baru itu menolak paket sebelumnya**: 8 dari 10 kasus di paket siklus 10
ditandai gagal (mis. 5 frame tanpa input), sementara 10/10 rekaman referensi Python
lolos. Jadi gate mengukur perilaku, bukan mencocokkan satu capture.

**Efek samping operasional (penting).** Flake start-up PTY — sumber tiga kegagalan
CI pada dua siklus sebelumnya (`missing readiness/timeout at stage 0` setelah
~207 KB padahal layar sudah penuh) — hilang, karena anak proses tidak lagi menulis
terus-menerus saat harness menunggu layar tenang. Dua CI pada sumber yang sama
kini hijau.

**Penjelasan "misteri" lama.** Kegagalan GREEN siklus 9 yang dulu tak dapat
direproduksi ternyata kegagalan gate live yang nyata, tersamar karena langkah
picker di `ci.yml` memakai `continue-on-error: true`: daftar langkah tetap
menampilkan "success" sementara `steps.picker.outcome` failure dan langkah agregat
yang menggagalkan job. Mulai sekarang bacalah anotasi `picker terminal`.

## 3. Yang BELUM dibuktikan (jangan dianggap selesai)

1. Capture hidup hanya menjalankan pemetaan `complete→done`. Tinta
   `interrupted`/`error`/`empty` dipatok oleh sumber upstream, unit test, dan
   fixture checker — belum oleh rekaman berpasangan.
2. Kolom `Active` (`2y ago` vs `2023-11-14`) dan `ID` (8 vs 18 karakter) tetap
   adaptasi terdokumentasi, belum ada keputusan untuk mengubahnya.
3. Resize, daftar panjang, dan clear-filter belum dibuktikan fixture ukuran tetap.
4. **Temuan baru (belum diperbaiki).** Ambang "terminal terlalu kecil" di Rust
   sekarang 60×8 dan pesannya dicetak lalu proses langsung kembali; sumber upstream
   yang dipatok menetapkan `max_y < 5 or max_x < 40`, menggambar `Terminal too
   small` di baris 1 di dalam layar curses, lalu **menunggu satu tombol** sebelum
   kembali. Jadi pada 40–59 kolom, referensi menampilkan picker penuh sementara
   Rust menolak. Ini masuk siklus berikutnya.
5. Belum ada PASS seluruh picker, belum ada acceptance, dan **tidak ada merge**.

## 4. Keputusan yang sudah Anda ambil (2026-09-15)

- Lanjut otomatis berurutan: sisa picker → wizard V1, laporan per siklus.
- Dropdown completion: ditunda sampai picker dan wizard selesai.
- Review paket: dokumen ini.

## 5. Apa yang mungkin Anda ingin lihat sendiri

- `picker-redraw-on-input-b2db435/audit.json` — blok `frames_per_window_before/after`
  dan `all_ten_cell_maps_identical_to_previous_packet`.
- `picker-status-ink-19bbbd5/REPORT.md` bagian "Kontrak yang dipatok referensi".
- `evidence/upstream-status-attr/provenance.json` — dari mana pemetaan tinta diambil.
- Verifikasi ulang lokal: `python3 docs/hermes-ui-spec/017/evidence/<paket>/verify.py`
  (butuh `PyYAML`, `pyte`, `wcwidth` dan repositori pada commit sesi ini).
