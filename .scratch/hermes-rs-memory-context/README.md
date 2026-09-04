# Spec 008 — Memory & Context Management

Vertical slice: ketika percakapan tumbuh melampaui `context_length` provider,
Hermes-RS mempertahankan percakapan yang valid dengan cara menjatuhkan turn
tertua (sliding window) dan merangkumnya, sambil menjaga pesan yang di-pin oleh
pengguna dan tetap kompatibel dengan `config.yaml` Python (bagian `compression`).

## Motivasi (terverifikasi dari kode)

- `context.rs` (Spec 006 #04) sudah menyediakan `estimate_tokens(text)` (char/4),
  `estimate_turns_tokens(&[Turn])`, dan `check_context_limit(&[Turn], Option<u64>)`.
  Saat ini helper **tidak di-wire** ke `ConversationRunner` (advisory-only, belum
  dipakai).
- `ConversationRunner<P: Provider>` memegang `turns: Vec<Turn>` dan selalu
  mengirim **seluruh** turns ke provider pada tiap panggilan
  (`chat`/`chat_agentic`). Tidak ada sliding window, summarization, atau pin.
  Percakapan panjang otomatis melampaui `context_length` tanpa mitigasi.
- Skema config (`schema.rs`) sudah punya `HermesConfig.compression:
  Option<CompressionConfig>` dengan `enabled: Option<bool>` dan
  `target_max_tokens: Option<u64>`, serta `context_length: Option<u64>` di
  `ModelConfig` dan `ProviderConfig`. Semua field sudah diparse tapi **belum
  dibaca untuk perilaku kompresi**.
- Tidak ada konsep pesan "di-pin". `ephemeral_system_prompt: Option<String>`
  ada di config namun belum punya jalur penggunaan/penghitungan token.

## Batas scope (disepakati)

- Berlaku **per-percakapan/runner** (memory jangka-pendek untuk durasi
  interaksi), BUKAN memory lintas-sesi durabel (indeks & retrieval — open
  decision di CONTEXT.md, di luar slice ini).
- Estimasi token tetap char-based `char/4` (Spec 008 memakai heuristik yang
  sama, tidak menambah tokenizer/dependency baru pada tiket awal).
- Kompresi **advisory & dijalankan pada batas turn** (sebelum kirim berikutnya),
  tidak pernah menyela stream yang sedang berjalan.
- Tidak ada perubahan security boundary / eksekusi tool.
- `state.db` tetap satu-satunya canonical storage untuk turns yang **benar-benar
  dikirim**; ringkasan/state window bersifat in-memory & keputusan dijelaskan.

## Tiket

| # | Tiket | Blocked by |
|---|---|---|
| 01 | [Integrasi token counting ke runner](issues/01-token-counting-integration.md) | — |
| 02 | [Sliding window (drop turns tertua)](issues/02-sliding-window.md) | 01 |
| 03 | [Summarization turn yang di-drop](issues/03-turn-summarization.md) | 02 |
| 04 | [Pinned messages (/pin)](issues/04-pinned-messages.md) | 02 |
| 05 | [Wiring config compression + konteks](issues/05-compression-config-wiring.md) | 01, 02 |
| 06 | [Parity, docs, penutupan](issues/06-parity-docs-closure.md) | 01–05 |

01 fondasi (token accounting) independen dari config; 05 tergantung bentuk
akuntansi 01 + perilaku window 02. 06 terakhir (closure).

## Invariant yang tetap berlaku

Semua invariant di `docs/ROADMAP.md` tetap berlaku: `state.db` canonical untuk
turn yang dikirim; SIGINT exit 130; credential terredaksi; turn yang dibatalkan
tidak pernah dipersist parsial. Sliding window tidak boleh menghapus turn dari
`state.db` yang sudah disimpan hanya karena dikeluarkan dari konteks aktif —
turn yang di-drop dari window tetap utuh di penyimpanan sesi; hanya kiriman ke
provider berikutnya yang memakai versi terpotong/diringkas.
