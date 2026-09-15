# W1 — Perilaku referensi resize/long-list/clear-filter pada Python v0.21.0

- Status: CLOSED 2026-09-15
- Type: wayfinder:research
- HITL: no
- Owner: Arena agent (sesi arena/01a0a14a)
- Parent map: [Wayfinder map — Rute penuntasan Spec017](WAYFINDER-spec017-closure.md)
- Blocked-by: —

## Question

Apa perilaku referensi `_curses_browse` Python v0.21.0 (blob terpinn
`63279301…`) untuk: (1) resize terminal (apakah redraw dipicu KEY_RESIZE/
SIGWINCH, apa yang terjadi pada kursor/filter saat geometri berubah);
(2) daftar panjang melebihi tinggi terminal (jendela scroll, posisi kursor,
indikator); dan (3) clear-filter (perilaku redraw dan posisi kursor)?
Keluaran: dokumen bukti terpinn (blob + hash + provenance, pola
`evidence/upstream-status-attr/`) yang bisa dijadikan kontrak untuk gate.
Tiket AFK-research: dikerjakan inline oleh sesi yang meng-claim (tidak ada
subagent di platform ini).

## Resolution — 2026-09-15 (CLOSED)

Riset selesai dari sumber upstream terpinn (tanpa subagent; dikerjakan inline
sesuai batasan platform). Temuan lengkap + provenance:
`docs/hermes-ui-spec/017/evidence/upstream-browse-control/README.md`.

Inti jawaban:
1. **Resize**: tanpa penanganan KEY_RESIZE eksplisit — frame berikutnya membaca
   geometri baru dan menggambar ulang penuh; tepat satu frame per kejadian;
   state (cursor/offset/filter) bertahan; clamp jendela minimal; bila terminal
   < 5 baris atau < 40 kolom → layar "Terminal too small" lalu keluar pada
   tombol berikutnya.
2. **Daftar panjang**: visible_rows = max_y − 4; kursor modulo (wrap-around);
   scroll window digeser minimal hanya agar kursor tampak; wrap melompatkan
   jendela ke atas; tanpa indikator scroll.
3. **Clear-filter**: Esc saat filter aktif ATAU backspace sampai kosong →
   daftar penuh + cursor=0 + scroll_offset=0; setiap mutasi filter me-reset
   kursor/offset; header kembali normal di frame berikutnya.
