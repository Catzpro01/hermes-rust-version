# 02: Sliding window (drop turns tertua)

**What to build:** Ketika estimasi konteks melebihi `context_length`, sebelum
mengirim turn berikutnya, jatuhkan turn tertua (selain yang diproteksi: pesan
yang di-pin, lihat 04) agar permintaan ke provider tetap muat. Drop hanya
berlaku pada **salinan yang dikirim ke provider**, bukan menghapus turn dari
`state.db`/sesi.

**Blocked by:** 01 (token accounting).

**Status:** done — commit di VM, 180/180 test hijau (`cargo test --workspace`),
`clippy --workspace --all-targets -D warnings` bersih.

## Kondisi sekarang (terverifikasi)

- `ConversationRunner::chat_agentic`/`chat`/`chat_with_cancel` mengirim
  `self.turns` penuh ke `provider.chat_with_cancel(...)`.
- Tidak ada pemangkasan: percakapan panjang mengirim seluruh sejarah tanpa batas.
- `context_limit: Option<u64>` kini tersimpan di runner (Ticket 01), di-set dari
  precedence config di REPL.

## Kriteria (per /ask-matt)

- [x] `turns_to_send()` mengembalikan **salinan** turns yang di-trim (tidak
      memutasi `self.turns`).
- [x] Drop turn tertua (dari depan) saat `estimate_turns_tokens > context_limit`.
- [x] Pertahankan N turns terakhir (turn paling baru — pertanyaan aktif — tidak
      pernah di-drop). System prompt saat ini tidak ada di aliran turns
      (`ephemeral_system_prompt` belum ter-wire); dicatat untuk masa depan.
- [x] `state.db` TIDAK berubah setelah sliding window — test integrasi
      byte-hash membuktikan read/resume tak menyentuh file.
- [x] `/messages <id>` tetap menampilkan SEMUA turns — test integrasi resume
      menunjukkan 100 turns utuh meski window memangkas kiriman.
- [x] Test: 100-turn conversation → `turns_to_send()` ≤ context_limit.
- [x] Test: `context_limit None` → tanpa trimming (backward compat).
- [x] 180/180 tests green, clippy clean.

## Perubahan

- `crates/hermes-core/src/conversation/mod.rs`: method `turns_to_send()` pada
  `ConversationRunner`; `chat`, `chat_with_cancel`, dan loop `chat_agentic` kini
  mengirim `turns_to_send()` (salinan ter-trim), sementara `self.turns` tetap
  penuh. +4 unit test `turns_to_send_*`.
- `crates/hermes-core/tests/conversation_session_integration.rs`: +1 integrasi
  `sliding_window_trims_send_but_state_db_keeps_full_history`.

## Catatan desain

- **Invariant kunci:** window hanya mempengaruhi yang dikirim ke LLM, bukan yang
  disimpan. `self.turns` & `state.db` menyimpan penuh; REPL mem-persist dari
  `runner.turns()` (full), `/messages` membaca `state.db` (full).
- Algoritma: jika `context_limit` ada & estimasi penuh > limit, buang turn dari
  depan sampai muat atau tersisa 1 turn (tak pernah kosong / tak pernah drop
  pertanyaan aktif). Rekomputasi tiap iterasi `chat_agentic` (tool result bisa
  menumbuhkan konteks).
- `context_limit None` → `turns_to_send()` = full clone (tanpa window), backward
  compatible.
- Pin (04) & summarization (03) akan memodifikasi pilihan turn yang di-drop pada
  tiket berikutnya; saat ini drop-front polos.
- Tech-debt Ticket 01 (Matt): `tracing_subscriber` sudah di-init di `main.rs`
  (default level menampilkan `warn!`), jadi warning konteks terlihat — terverifikasi.

## Dependency

01.
