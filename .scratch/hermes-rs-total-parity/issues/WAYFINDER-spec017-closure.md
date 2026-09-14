# Wayfinder map — Rute penuntasan Spec017

- Status: OPEN
- Type: wayfinder:map
- Label: wayfinder:map
- Owner:
- Parent map: —

## Destination

Spec017 selesai berarti T12 dan T13 tertutup: semua sisa perbedaan visual
diperbaiki atau dinyatakan keluar scope secara eksplisit oleh pengguna; bukti
§J.7 lengkap (wizard tiap step: normal/cancel/unavailable; completion
dropdown; nested fields; semua pasangan empat area + rekaman raw + metadata
reproduksi); CI relevan GREEN; dan acceptance eksplisit pengguna diperoleh.
Merge BUKAN bagian destinasi ini.

## Notes

- Domain: parity visual Hermes-RS vs Python v0.21.0 (Spec017 §J.7).
- Skill yang dirujuk setiap sesi: `grilling` + `domain-modeling` untuk tiket
  HITL; `tdd`/`implement` hanya di jalur eksekusi (T13), bukan di peta ini.
- Preferensi berdiri (dikonfirmasi pengguna 2026-09-15, wawancara Q1–Q4):
  1. Urutan kerja: picker residu → wizard V1 → completion → kelengkapan T12
     → acceptance.
  2. Acceptance bertahap per area: setiap paket bukti area yang selesai
     langsung direview pengguna (accept/reject eksplisit per area).
  3. Completion = Rust inline dropdown mereplikasi perilaku referensi Python
     dengan bukti terpinn (pola TDD picker).
- Q1–Q8 grilling terdahulu TETAP tertutup; jangan dibuka ulang.
- Batasan platform: tidak ada toolchain Rust lokal; semua verifikasi Rust
  lewat runner resmi `vps-fern-hermes` (CI hijau per commit). Tool Skill/
  subagent tidak tersedia di platform ini — tiket `research` dikerjakan
  inline oleh sesi yang meng-claim (jujur, tanpa klaim eksekusi paralel).
- Bukti lama immutable; jangan menimpa paket evidence asli.
- Peta ini hanya indeks keputusan; pekerjaan implementasi berjalan lewat T13
  (`/implement` + `/tdd`), bukan lewat tiket di sini.

## Decisions so far

<!-- satu baris per tiket CLOSED: judul + gist jawaban + tautan -->

(belum ada tiket yang diresolusi)

## Not yet specified

- Detail perilaku/visual dropdown completion (scroll, posisi, highlight) —
  baru bisa ditiketan setelah kontrak referensi completion diputus.
- Bentuk fixture live untuk daftar panjang/resize/clear-filter — menunggu
  kontrak referensi resize ditetapkan.
- Batas bukti "full Python CLI run" vs component-level bila pengguna meminta
  lebih dari yang T12 spesifikasikan saat acceptance — area ini kabur sampai
  paket bukti wizard/completion ada.

## Out of scope

- OAuth/backend/registry wizard — deferral eksplisit T13 V1, bukan rute ini.
- Merge ke `main` — keputusan terpisah di luar destinasi.
