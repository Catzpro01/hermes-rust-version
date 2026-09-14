# W4 — Kontrak referensi dan capture completion dropdown

- Status: CLOSED 2026-09-15
- Type: wayfinder:grilling
- HITL: yes
- Owner: Arena agent (sesi arena/01a0a14a)
- Parent map: [Wayfinder map — Rute penuntasan Spec017](WAYFINDER-spec017-closure.md)
- Blocked-by: —

## Question

Standar parity completion sudah diputuskan (Q4, 2026-09-15): Rust inline
dropdown mereplikasi perilaku Python dengan bukti terpinn. Tiket ini
memutuskan kontrak detailnya: fungsi/komponen Python mana yang menjadi
referensi terpinn (blob + hash), apa arti "normal dan alternatif
representatif" pada capture T12 (kasus minimum yang sah), di mana dropdown
digambar relatif terhadap prompt, dan gate/checker apa yang mengawal regresi
sebelum implementasi masuk jalur T13 `/implement`+`/tdd`.

## Resolution — 2026-09-15 (CLOSED)

Grilling dengan pengguna (3 pertanyaan, semua memilih rekomendasi):
1. **Sumber referensi**: rekaman PTY dropdown Python v0.21.0 dibuat oleh
   pengguna di mesinnya sendiri, dipinn sebagai bukti (sesuai destinasi bukti
   rekaman mentah + metadata repro). Tugas pengguna: menyediakan rekaman ini.
2. **Matriks kasus minimum**: 4 kasus — filter kandidat (>1 hasil),
   tanpa-cocokan, pilih+commit kandidat, dismiss — masing-masing di dua lebar
   (80×24 dan 160×48).
3. **Moda gate**: satu live gate PTY deterministik untuk dropdown Rust +
   bukti capture terpinn berdampingan dengan referensi Python, mengikuti pola
   `picker-redraw-on-input`, sebelum implementasi masuk jalur T13.
