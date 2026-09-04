# 01: Integrasi token counting ke runner

**What to build:** Wire helper `estimate_turns_tokens` (Spec 006 #04) ke
`ConversationRunner` sehingga total estimasi token per-kontek tersedia & bisa
di-query, dan REPL bisa menampilkannya (token accounting sebagai fondasi semua
tiket Spec 008).

**Blocked by:** — (Spec 006 #04 sudah landed sebagai fondasi).

**Status:** done — commit di VM, 175/175 test hijau (`cargo test --workspace`),
`clippy --workspace --all-targets -D warnings` bersih.

## Kondisi sekarang (terverifikasi)

- `crates/hermes-core/src/conversation/context.rs` punya `estimate_turns_tokens`.
- `ConversationRunner<P: Provider>` (`conversation/mod.rs`) memegang
  `turns: Vec<Turn>`; tidak ada penghitung token yang terekspos.
- REPL (`crates/hermes-cli/src/repl.rs`) mencetak baris sambutan
  `Hermes-RS session {id} (provider {name})`; tidak menampilkan ukuran konteks.

## Kriteria (per /ask-matt)

- [x] `ConversationRunner` menghitung total token dari turns via
      `estimate_turns_tokens()` → method `estimated_tokens()`.
- [x] Token count ditampilkan di REPL (baris sambutan `[context ~N tokens /
      limit L]` + perintah `/info`).
- [x] Token count update setiap turn baru (accounting dihitung langsung dari
      `self.turns` saat dipanggil, tak ada cache yang bisa basi).
- [x] `check_context_limit()` dipanggil sebelum kirim request ke provider —
      di `chat_agentic` (satu kali per user turn) via `context_warning()`.
- [x] Warning muncul jika estimasi > context_length (non-blocking, `tracing::warn`).
- [x] context_length dibaca dari `ProviderConfig` → `ModelConfig` → None
      (precedence) via `resolve_context_limit` di REPL, di-set ke runner saat
      startup & saat `/provider <name>` switch.
- [x] 175/175 tests green, clippy clean.

## Perubahan

- `crates/hermes-core/src/conversation/mod.rs`: field `context_limit:
  Option<u64>` pada `ConversationRunner`; `set_context_limit`, `context_limit`,
  `estimated_tokens`, `context_warning`; `chat_agentic` emit `tracing::warn!`
  non-blocking saat estimasi melewati batas. +5 unit test.
- `crates/hermes-cli/src/repl.rs`: `resolve_context_limit(config, active)` helper
  (ProviderConfig.context_length > model.context_length > None); set ke runner
  saat startup & switch provider; tampilkan token di baris sambutan; perintah
  `/info` (provider, estimasi, limit, warning). +4 unit test.

## Catatan desain

- Murni **accounting + advisory**: belum ada pemangkasan (sliding window = tiket
  02). Estimasi token memakai helper `context.rs` yang sama (`char/4`),
  satu-satunya tempat definisi heuristik.
- `context_limit` default `None` = perilaku lama (tanpa limit), backward
  compatible; bila tak ada limit, `/info` menampilkan `limit: none` dan tak ada
  warning.
- Precedence limit dieja eksplisit di test (angka di-pin).

## Dependency

Spec 006 #04 (context helper) — sudah done.
