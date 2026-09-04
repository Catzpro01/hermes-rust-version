# 01: Goal extraction & tracking

**What to build:** Model fondasi "apa yang ingin dicapai" dari prompt user dan
lacak kemajuan terhadapnya di tiap iterasi agentic, sebagai lapisan yang
dipakai tiket 02–04 (plan dibentuk utk goal, refleksi menilai hasil thd goal).

**Blocked by:** —

**Status:** done — commit di VM, 209 test hijau, clippy clean.

## Kondisi sekarang (terverifikasi)

- `chat_agentic` (`conversation/mod.rs`) menerima satu `content` (prompt user),
  mendorongnya sebagai `Turn::User`, lalu langsung mengeksekusi loop tool tanpa
  pernah menormalkan "tujuan". Setelah selesai ia hanya mengembalikan
  `AgenticResult::{Done{text,..}|MaxIterations|Cancelled}` — tak ada status
  "goal tercapai vs tidak".
- `AgenticResult` tidak membawa informasi pencapaian; REPL hanya mencetak teks
  akhir + `[iter N/10]`.
- Tidak ada struktur penyimpanan status per-iterasi yang bisa dikueri user.

## Konsep

- Model internal `GoalTracker` (in-memory, per-runner) berisi: ringkasan goal
  (teks pendek yang dapat disimpan sementara), kapan dibentuk, dan skor/status
  kemajuan yang di-update tiap iterasi.
- Ekstraksi goal bisa **heuristik** (tanpa LLM tambahan: potong prompt ke
  klausa imperatif pertama; char-safe, deterministik, uji-able) sebagai langkah
  pertama, dengan jalur opsional meminta LLM menormalkan goal bila diaktifkan
  (lihat 02). Default heuristik agar tidak menambah round-trip dan tetap
  backward compatible.
- `/goal` menampilkan status goal (teks + progres). Goal **tidak** dipersist ke
  `state.db` tanpa ADR; in-memory per-sesi (konsisten pola Opsi 3).
- Token: teks goal hanya ringkas; jika disimpan sbg catatan internal tidak
  perlu masuk konteks. Bila dipakai dalam permintaan (02), dihitung via
  `estimate_tokens`.

## Kriteria

- [x] Ekstraksi goal heuristik dari `Turn::User` pertama: deterministik &
      char-safe (uji dengan CJK).
- [x] `GoalTracker` mencatat status kemajuan per iterasi (progress `0.0..=1.0`
      atau enum `NotStarted|InProgress|Achieved|Blocked`) dan bisa
      di-set/di-kueri dari `chat_agentic`.
- [x] `/goal` di REPL menampilkan goal + status; kosong bila belum ada.
- [x] Default (tanpa aktivasi) tidak mengubah perilaku `chat_agentic`
      (regresi nol).
- [x] Test unit + integration mem-pin angka/status; seluruh suite tetap hijau.
- [x] Clippy `-D warnings` bersih.

## STRIDE

- **Prompt-injection/Integritas:** goal berasal dari prompt user sendiri; tidak
  ada source eksekusi baru. Ekstraktor heuristik tidak menjalankan apa pun.
- **Informasi:** `/goal` melewati sanitasi + redaksi yang sama seperti `/info`
  (tidak bocorkan konten di luar jalur render).
- Tidak ada surface credential/eksekusi baru.

## Risiko

- Mengira "goal" bila user cuma bertanya factual — mitigasi: hanya aktif pada
  mode terencana (02) atau perintah eksplisit; default heuristik konservatif.

## Dependency

—
