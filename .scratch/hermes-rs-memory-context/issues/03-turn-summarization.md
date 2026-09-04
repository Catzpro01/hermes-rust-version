# 03: Summarization turn yang di-drop

**What to build:** Alih-alih membuang turn tertua mentah-mentah saat window
menyempit, sediakan ringkasan singkat turn yang dikeluarkan sebagai **visibility
tool** (bukan injeksi) — sehingga user tahu apa yang terpotong dari konteks aktif
tanpa mengubah kontrak `Turn`/`state.db`.

**Blocked by:** 02 (sliding window menentukan turn mana yang dikeluarkan).

**Status:** done — commit di VM, 187/187 test hijau (`cargo test --workspace`),
`clippy --workspace --all-targets -D warnings` bersih.

## Keputusan desain (dikunci /ask-matt — Opsi 3)

Ringkasan **dihitung & ditampilkan, injeksi ke konteks LLM ditunda**. Alasan:
enum `Turn` hanya punya `User`/`Assistant`/`Tool` — tidak ada `System`/`Summary`.
Menyuntik ringkasan sebagai pesan ber-peran yang tidak ada di enum akan
(a) berisiko ditolak provider (role `system` di tengah percakapan), dan
(b) menyamarkan ringkasan sebagai pesan asli → vektor prompt-injection
(Tampering + Elevation). Menambah varian `Turn::Summary` adalah refactor
lintas-arsitektur (match di conversation/provider/persist `state.db` role
column/redaction/search) yang butuh ADR formal + STRIDE + parity test — bukan
keputusan di satu tiket. Konsisten dengan pola Spec 006-04 (helper murni dulu,
wiring menyusul saat representasi diputuskan).

## Yang dibangun (Opsi 3)

- **`summarize_dropped(&[Turn]) -> String`** di `conversation/context.rs` —
  heuristic-only, deterministik:
  - ambil lead baris pertama dari s.d. `SUMMARY_MAX_TURNS = 3` turn,
    label peran (`User:`/`Asst:`/`Tool: nama`), potong ke `SUMMARY_MAX_CHARS
    = 100` (char-safe, aman untuk CJK multi-byte),
  - sisanya dijumlah `(+N more)`, prefix `[M turns dropped]`.
  - Ini BUKAN prompt yang dikirim ke LLM — murni untuk display/info.
- **Wiring `/info`** di REPL: setelah token count + window, tampilkan
  `dropped_turns()` lewat `summarize_dropped`, **melalui redaction +
  sanitization** (`redact_credentials` + `sanitize_untrusted_output`) sebelum
  sampai ke terminal.
- **`ConversationRunner::dropped_turns()`** → prefix turns yang akan di-drop
  window (untuk display), tanpa memutasi `self.turns`.

## Yang TIDAK dibangun (sengaja ditunda)

- ❌ Tidak ada injeksi ringkasan ke konteks LLM.
- ❌ Tidak ada varian `Turn` baru / role baru di `state.db`.
- ❌ Tidak ada LLM-recursive summarization (bisa jadi sub-tiket/Spec 009).

## Kriteria (per /ask-matt)

- [x] `summarize_dropped()` helper di `conversation/context.rs`.
- [x] Heuristik: first line + truncate, max 3 turns, remaining count.
- [x] Ditampilkan di `/info` saat ada dropped turns (visibility, bukan injeksi).
- [x] TIDAK diinjeksi ke konteks LLM (belum ada representasi formal).
- [x] Ringkasan melewati redaction + sanitization sebelum display.
- [x] Unit test: empty, 1 turn, 3 turns, >3 turns, long content truncation,
      multibyte char-safe, first-line-only.
- [x] `state.db` tidak berubah (display path read-only; tak menyentuh storage).
- [x] 187/187 tests green, clippy clean.

## STRIDE

- **Prompt-injection/Tampering:** ringkasan TIDAK dikirim sebagai pesan; hanya
  ditampilkan sebagai info ber-label, jadi tak bisa memanipulasi model. Saat
  injeksi diformalkan (ADR), wajib role terpisah — bukan User palsu.
- **Information disclosure:** output display dilewatkan `redact_credentials` +
  ANSI sanitize. Helper di core murni string, tak ada I/O baru.
- Tidak ada surface eksekusi baru.

## Perubahan

- `crates/hermes-core/src/conversation/context.rs`: `summarize_dropped`,
  `first_line_truncated`, konstanta `SUMMARY_MAX_TURNS`/`SUMMARY_MAX_CHARS`.
  +6 unit test.
- `crates/hermes-core/src/conversation/mod.rs`: `ConversationRunner::dropped_turns()`.
  +1 unit test.
- `crates/hermes-cli/src/repl.rs`: `/info` menampilkan dropped summary
  (sanitized + redacted).

## Dependency

02.
