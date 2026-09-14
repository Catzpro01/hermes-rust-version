# W4 — Kontrak referensi dan capture completion dropdown

- Status: OPEN
- Type: wayfinder:grilling
- HITL: yes
- Owner:
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
