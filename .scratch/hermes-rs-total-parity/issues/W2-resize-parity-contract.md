# W2 — Kontrak parity resize/long-list/clear-filter picker Rust

- Status: CLOSED 2026-09-15
- Type: wayfinder:grilling
- HITL: yes
- Owner: Arena agent (sesi arena/01a0a14a)
- Parent map: [Wayfinder map — Rute penuntasan Spec017](WAYFINDER-spec017-closure.md)
- Blocked-by: [W1 — Perilaku referensi resize/long-list/clear-filter pada Python v0.21.0](W1-resize-reference-behavior.md)

## Question

Setelah bukti referensi W1 ada: untuk setiap perilaku (resize, daftar panjang,
clear-filter) — parity penuh terhadap Python, atau adaptasi terdokumentasi
(seperti `Active`/`ID`)? Termasuk: apakah perilaku itu butuh gate live baru di
CI (pola `picker_redraw_on_input`) atau cukup bukti capture berpinned, dan
fixture mana yang sah (dummy fixtures terisolasi sesuai batasan T12).

## Resolution — 2026-09-15 (CLOSED)

Grilling dengan pengguna (4 pertanyaan, semua memilih rekomendasi):
1. **Resize = parity penuh**, termasuk layar "Terminal too small" dan keluar
   dari picker pada tombol berikutnya bila terminal <5 baris / <40 kolom.
2. **Daftar panjang = parity apa adanya**: kursor modulo/wrap-around, jendela
   scroll minimal, lompatan jendela ke atas saat wrap adalah perilaku
   referensi (bukan bug), tanpa indikator scroll.
3. **Clear-filter = parity reset penuh**: Esc (saat filter aktif) atau
   backspace sampai kosong → daftar penuh + cursor=0 + scroll_offset=0 +
   header normal frame berikutnya.
4. **Moda bukti = tiga live gate baru** mengikuti pola gate picker yang ada
   (RED 3× → GREEN resmi → capture terpinn); bila resize dinamis via SIGWINCH
   terbukti tak layak di harness PTY, fallback capture-only untuk resize harus
   dicatat alasannya di paket bukti — bukan diam-diam dilewati.
Dasar fakta: riset W1 (`docs/hermes-ui-spec/017/evidence/upstream-browse-control/`).
