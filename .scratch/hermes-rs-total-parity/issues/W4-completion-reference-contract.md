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

## Addendum eksekusi — 2026-09-15

Geometri capture yang benar-benar dipakai (memenangkan angka tiket demi
konsistensi harness): lebar **100 dan 80 kolom × 30 baris**,
`TERM=xterm-256color`, HOME terisolasi tanpa kredensial — identik di sisi
Rust maupun referensi Python.

**Sisi Rust SELESAI & HIJAU** (run `34925228949` + konfirmasi `34926269312`):
gate live `completion-dropdown` (5 skenario × 2 lebar) + 14 unit test
checker; temuan semantik rustyline: Tab menyisipkan kandidat PERTAMA urutan
registri (menu tidak ter-render di permukaan capture); skills diselesaikan
di token pertama.

**Kontrak perbandingan semantik** (di-pin `check_completion_reference.py`
setelah bundle referensi mendarat; penilaian = himpunan kandidat + transisi
UI, BUKAN byte-for-byte, karena engine render berbeda):

| Skenario | Input | Kontrak kedua sisi |
|---|---|---|
| completion-command | `/mod` | kandidat unik `{model}`, terkomplet tanpa spasi trailing |
| completion-alternatives | `/s` | himpunan kandidat ⊇ {save, snapshot, stop, steer, skin, skills, sessions, status, …}; urutan bebas |
| completion-subcommand | `/skills sea` lalu `/demo` | `{search}`; `{demo-skill}` bila skill ter-seed (kontrak seed: `skills/demo-skill/SKILL.md`, description 'Parity evidence fixture skill') |
| completion-ghost | `/perso` tanpa Tab | ghost `nality` ter-render |
| completion-picker-open | `/sessio` + Enter | UI browse sessions terbuka (cukup frame pembuka) |
| completion-no-match | `/zzzz` | kandidat kosong, tanpa menu |

Divergensi yang DIHARAPKAN (bukan kegagalan perekaman): Python menampilkan
menu dropdown saat mengetik (prompt_toolkit), Rust menyisipkan kandidat ke
baris (rustyline) — bila referensi membuktikan gap, tiket implementasi menu
Rust menyusul sebagai temuan parity.
