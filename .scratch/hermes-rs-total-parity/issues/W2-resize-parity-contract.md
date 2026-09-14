# W2 — Kontrak parity resize/long-list/clear-filter picker Rust

- Status: OPEN
- Type: wayfinder:grilling
- HITL: yes
- Owner:
- Parent map: [Wayfinder map — Rute penuntasan Spec017](WAYFINDER-spec017-closure.md)
- Blocked-by: [W1 — Perilaku referensi resize/long-list/clear-filter pada Python v0.21.0](W1-resize-reference-behavior.md)

## Question

Setelah bukti referensi W1 ada: untuk setiap perilaku (resize, daftar panjang,
clear-filter) — parity penuh terhadap Python, atau adaptasi terdokumentasi
(seperti `Active`/`ID`)? Termasuk: apakah perilaku itu butuh gate live baru di
CI (pola `picker_redraw_on_input`) atau cukup bukti capture berpinned, dan
fixture mana yang sah (dummy fixtures terisolasi sesuai batasan T12).
