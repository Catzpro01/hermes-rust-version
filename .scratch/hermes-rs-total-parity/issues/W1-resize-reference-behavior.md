# W1 — Perilaku referensi resize/long-list/clear-filter pada Python v0.21.0

- Status: OPEN
- Type: wayfinder:research
- HITL: no
- Owner:
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
