# W3 — Batas medan nested/common-model yang wajib tampak pada wizard

- Status: CLOSED 2026-09-15
- Type: wayfinder:grilling
- HITL: yes
- Owner: Arena agent (sesi arena/01a0a14a)
- Parent map: [Wayfinder map — Rute penuntasan Spec017](WAYFINDER-spec017-closure.md)
- Blocked-by: —

## Question

T12 mencatat gap "nested fields" dan T13 V1 meminta "capture required common
model fields rather than pretending they were shown". Keputusan apa yang
diperlukan: medan mana pada wizard yang WAJIB tampak di frame (bukti visual)
vs medan yang hanya hidup di model/data (cukup dibuktikan lewat tes unit/
registry), per langkah wizard yang terimplementasi. Cek inventaris
section/field terhadap Q1 grilling terdahulu tanpa membuka ulang Q1–Q8.

## Resolution — 2026-09-15 (CLOSED)

Grilling dengan pengguna (3 pertanyaan, semua memilih rekomendasi):
1. **Batas frame-vs-model**: setiap prompt/label/nilai yang dirender per
   section frame WAJIB tampak di capture; data nested yang hanya hidup di
   model (ModelAnswer/PlatformAnswers env-var pairs/toolset keys) dibuktikan
   lewat tes unit/registry; secret tidak pernah masuk capture.
2. **Kesenjangan referensi**: setup.py upstream belum ter-vendor; batas medan
   ditetapkan dari inventaris Q1 + rujukan baris setup.py yang sudah ada di
   kode Rust; vendoring setup.py dicatat sebagai tugas bukti terpisah yang
   terblokir jaringan upstream (masuk fog peta).
3. **Matriks kasus**: sesuai T12 — tiap section frame × {normal, cancel} +
   kasus unavailable-feature untuk fitur ber-gate. Quick/Blank bagian dari
   matriks mode bila step-nya terimplementasi.
