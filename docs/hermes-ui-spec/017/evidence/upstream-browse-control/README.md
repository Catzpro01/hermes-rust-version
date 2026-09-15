# Referensi upstream — kontrol browse: resize, daftar panjang, clear-filter

**Provenance.** Sumber upstream Python v0.21.0 commit `63279301` (blob
`8281cbdd…`, sha256 `89cde75d…`, 625.460 byte) tersimpan di
`../upstream-status-attr/hermes_cli_main.py`; fungsi `_curses_browse` pada
baris 1401–1631 berkas itu. Dokumen ini memetakan perilaku kontrolnya; tidak
ada salinan sumber tambahan.

## 1. Resize terminal

- **Tidak ada penanganan `KEY_RESIZE` eksplisit.** Loop utama: `clear()` →
  `getmaxyx()` → gambar penuh → `refresh()` → `getch()` memblokir.
- Saat resize terjadi ketika `getch()` memblokir, ncurses mengembalikan
  `KEY_RESIZE`; kunci itu tidak cocok cabang mana pun sehingga tidak ada
  mutasi state — iterasi loop berikutnya membaca `getmaxyx()` baru dan
  menggambar ulang seluruh layar dengan geometri baru. **Tepat satu frame
  tambahan per kejadian resize** (konsisten dengan kadens redraw referensi:
  satu frame per tombol).
- State bertahan across resize: `cursor`, `scroll_offset`, `search_text`,
  `confirm_delete`, `flash`.
- Clamp geometri setelah resize: `visible_rows = max(max_y - 4, 1)`; jendela
  digeser **minimal** hanya bila kursor keluar jendela (`cursor < offset` →
  `offset = cursor`; `cursor >= offset + visible_rows` → `offset = cursor -
  visible_rows + 1`). Tidak ada recentering lain.
- **Batas minimum**: bila `max_y < 5 or max_x < 40` layar diganti pesan
  `Terminal too small`, menunggu satu tombol, lalu **keluar dari picker**
  (return). Ini berlaku kapan pun, termasuk setelah resize.

## 2. Daftar panjang (scroll window)

- `visible_rows = max_y - 4` (header baris0, header kolom baris1, blank
  baris2, footer baris `max_y-1`); baris sesi digambar dari y=3 dan dipotong
  di `y >= max_y - 1`.
- Navigasi kursor **modulo** (wrap-around): `(cursor ± 1) % len(filtered)`.
  Tidak ada PageUp/PageDown/Home/End.
- Kebijakan scroll **minimal** seperti resize: `scroll_offset` berubah hanya
  untuk menjaga kursor tetap tampak. Konsekuensi wrap: turun dari item
  terakhir melompat ke item 0 → `offset` menjadi 0 (jendela melompat ke atas).
- Tidak ada indikator scroll; footer selalu `cursor+1/len(filtered) sessions`
  (+ `(filtered from N)` bila filter aktif).

## 3. Clear-filter (dan mutasi filter lain)

Dua jalan clear-filter, keduanya **mereset kursor dan scroll**:

- **Esc saat filter aktif**: `search_text=""`, `filtered=list(sessions)`,
  `cursor=0`, `scroll_offset=0`. (Esc kedua, saat filter kosong, keluar.)
- **Backspace sampai kosong**: `filtered=list(sessions)`, `cursor=0`,
  `scroll_offset=0`.

Aturan umum: setiap mutasi `search_text` (tambah karakter printable, hapus
sampai kosong, Esc) me-reset `cursor=0` dan `scroll_offset=0`. Frame
berikutnya menggambar header normal kembali (palette2+bold sesuai bukti
header yang sudah terpinn) dengan daftar penuh.

## Implikasi kontrak untuk Rust

1. Resize = satu redraw dengan geometri baru; state tidak di-reset; clamp
   jendela minimal.
2. Terminal < 5 baris atau < 40 kolom = layar `Terminal too small` + keluar
   pada tombol berikutnya.
3. Daftar panjang = kursor modulo + jendela scroll minimal; lompatan wrap ke
   atas adalah perilaku referensi, bukan bug.
4. Clear-filter = reset kursor/offset ke 0 + daftar penuh + header normal.
