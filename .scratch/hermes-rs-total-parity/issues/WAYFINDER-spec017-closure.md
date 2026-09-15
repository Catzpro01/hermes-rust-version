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

- [W1 — Perilaku referensi resize/long-list/clear-filter pada Python v0.21.0](W1-resize-reference-behavior.md): resize = satu frame redraw geometri baru + clamp minimal + "Terminal too small" keluar; daftar panjang = kursor modulo + scroll minimal; clear-filter = reset cursor/offset ke 0. Bukti: `docs/hermes-ui-spec/017/evidence/upstream-browse-control/`.
- [W2 — Kontrak parity resize/long-list/clear-filter picker Rust](W2-resize-parity-contract.md): parity penuh untuk ketiganya (termasuk keluar saat terminal <5×40) dan tiga live gate baru pola RED→GREEN; fallback capture-only resize hanya dengan alasan tercatat.
- [W3 — Batas medan nested/common-model yang wajib tampak pada wizard](W3-wizard-nested-fields-boundary.md): medan terrender-per-frame wajib capture; nested model-only via tes unit/registry (secret tak pernah dicapture); matriks T12 per section × {normal, cancel} + unavailable-feature; vendoring `setup.py` = tugas bukti terpisah terblokir jaringan upstream.
- [W4 — Kontrak referensi dan capture completion dropdown](W4-completion-reference-contract.md): referensi = rekaman PTY Python v0.21.0 dari mesin pengguna (tugas pengguna); matriks 4 kasus (filter >1 hasil / tanpa-cocokan / pilih+commit / dismiss) × 2 lebar (80×24, 160×48); satu live gate deterministik + capture berdampingan pola `picker-redraw-on-input`.

## Catatan eksekusi fase berikut

Semua tiket keputusan W1–W4 CLOSED. Lane kerja berikutnya (urutan Q2):
1. **Picker residu** — SELESAI 2026-09-15 (run `34910464818` hijau penuh:
   fmt=clippy=test=picker=success). Gate live `picker-browse-control`
   (5 skenario × 2 lebar) menjaga kontrak W2: redraw geometri baru +
   "Terminal too small" mid-sesi, kursor modulo + jendela clamp minimal,
   clear-filter reset kursor/offset (footer = cursor+1/total). Bukti lintasan:
   RED `b494990`, hardening `755368f`, GREEN `a916b73`, koreksi kontrak
   `ccf157b`, merge insiden fmt `3917834`.
2. **Wizard V1** — SELESAI 2026-09-15 (run `34916506533` hijau penuh:
   fmt=clippy=test=picker=success, 214 tes cargo, 12 gate live). Kontrak W3
   dijaga gate live `wizard-fields` (4 skenario: urutan field model lengkap
   provider→API base URL→env-var key→model name→selesai; cancel field
   pertama; docker-image pasca notice "Docker not found"; cancel multiselect
   gateway) + 10 unit test checker. Bukti lintasan: gate `6800ce7`,
   hardening toolchain `3e8aae9` (guard MSRV ≥ 1.88 + pin `$GITHUB_PATH`),
   4 insiden infra tercatat jujur (MSRV drift, disk penuh, cancel mid-clippy,
   runner putus). Vendoring `setup.py` tetap tugas bukti terpisah terblokir
   jaringan upstream.
3. **Completion (sisi Rust)** — SELESAI 2026-09-15 (run `34925228949` hijau
   penuh: fmt=clippy=test=picker=success, 214 tes cargo, 13 gate live).
   Kontrak W4 dijaga gate live `completion-dropdown` (5 skenario REPL ×
   2 lebar: unique completion `/mod`→`/model`, inserksi terfilter-prefix
   `/s`→`/save ` lalu `/ski`→`/skin` tanpa spasi, subcommand
   `/skills sea`→`search`, skill seeded `/demo`→`demo-skill`, ghost text
   `nality` tanpa Tab, `/sessions` diterima membuka browse picker) +
   14 unit test checker. Bukti lintasan: gate `6c2239e`, RED fix semantik
   rustyline `818ebe6` (menu tak terrender; Tab menyisipkan kandidat
   pertama urutan registri), RED fix token-pertama skill `953890c`.
   Sisi Python (perbandingan byte vs rekaman referensi) tetap menunggu
   rekaman pengguna per W4-Q1.
4. **Kelengkapan T12** — SEPARUH JALAN: matriks wizard §J.7 (tiap section ×
   {normal, cancel} + unavailable) SELESAI 2026-09-15 (run `34949715611`
   hijau penuh; gate `wizard-fields` kini 8 skenario: model fields/cancel,
   docker-image unavailable, gateway cancel, terminal local, gateway empty,
   tools accept/cancel; komit `1c2f3a6`, satu flake 0-byte
   `completion-alternatives-80x30` di run `34948203974` hilang saat retry).
   Sisa T12: pasangan empat area + rekaman mentah + metadata repro —
   termasuk bundle referensi Python Lane 3 yang masih direkam di VPS.
   2026-09-15 (sesi ini): matriks empat area diselaraskan dengan gate (30 nama,
   60 kasus; sisi Python hijau penuh direkam lokal terhadap `63279301`), 9
   deferral tercatat dengan alasan — dua di antaranya dipulihkan oleh marker
   penyelesaian per-sisi (`hermes_cli/setup.py:3210`), dan satu temuan produk lahir dari probe itu
   (daftar sesi Python dipotong `list_sessions_rich(limit=20)` vs Rust 30 baris)
   → tiket keputusan sendiri bila pengguna ingin parity, bukan adaptasi diam-diam.
   Belum ada paket baru: setengah Rust masih butuh run capture di runner.
   Lanjut → **acceptance bertahap per area** (Q3).

## Not yet specified

- Detail perilaku/visual dropdown completion (scroll, posisi, highlight) —
  baru bisa ditiketan setelah kontrak referensi completion diputus.
- Bentuk fixture live untuk daftar panjang/resize/clear-filter — kontrak
  perilaku sudah dipatok W2; detail fixture PTY diputuskan di lane implementasi.
- Batas bukti "full Python CLI run" vs component-level bila pengguna meminta
  lebih dari yang T12 spesifikasikan saat acceptance — area ini kabur sampai
  paket bukti wizard/completion ada.

## Out of scope

- OAuth/backend/registry wizard — deferral eksplisit T13 V1, bukan rute ini.
- Merge ke `main` — keputusan terpisah di luar destinasi.
