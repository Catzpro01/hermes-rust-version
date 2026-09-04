# Spec 006 — Model Fallback & Load Balancing

Vertical slice: ketika provider yang sedang aktif gagal (HTTP 429/5xx, timeout,
atau error transien), Hermes-RS otomatis mengalihkan permintaan ke provider
berikutnya dalam rantai fallback yang terkonfigurasi, dengan retry cerdas dan
pengenalan batas konteks, sambil mencatat kesehatan provider agar endpoint yang
sedang bermasalah tidak dibanjiri permintaan berulang.

## Motivasi (terverifikasi dari kode)

- `ProviderError` (di `crates/hermes-core/src/provider/mod.rs`) sudah membedakan
  `Http { status: u16, message }`, `Message(String)` (transport/send), dan
  `Cancelled`. Namun tidak ada klasifikasi "transien vs permanen", tidak ada
  cara membedakan 429/5xx dari 4xx lain, dan tidak ada status timeout.
- `HttpProvider` membangun `reqwest::Client::new()` tanpa `.timeout(...)`;
  `fn _timeout()` di `http.rs` hanyalah konstanta `#[allow(dead_code)]` 30 dtk
  yang tidak pernah dipakai. Tidak ada mekanisme timeout/retry sama sekali.
- Satu `ConversationRunner` memegang satu provider (`provider: P`); `Provider`
  adalah trait async. Fallback paling bersih dibangun sebagai *wrapper*
  yang mengimplementasikan `Provider` dan mencoba daftar provider secara
  berurutan — tanpa mengubah `ConversationRunner`/`repl`.
- `ProviderConfig { api, api_mode, key_env, models: HashMap<String, serde_yaml::Value>, context_length }`
  sudah diparse. Nilai per-model adalah `serde_yaml::Value` bebas; `context_length`
  sudah ada di `ProviderConfig` dan `ModelConfig` tapi tidak dibaca untuk routing.

## Batas scope (disepakati)

- Fallback & retry pada **awal permintaan** (sebelum/menjelang `request.send()`),
  di mana `ProviderError::Http`/`Message` dapat dibedakan dan stream belum mulai
  mengalir. Kegagalan di tengah stream yang sudah menghasilkan output punya
  implikasi partial-turn dan di luar slice ini (harus diputuskan dengan hati-hati
  terhadap invariant "partial turn tidak pernah disimpan").
- Konfigurasi rantai fallback + parameter retry via `config.yaml`.
- **Tidak ada** perubahan security boundary / eksekusi tool baru.
- `state.db` tetap satu-satunya canonical storage; status kesehatan provider
  bersifat in-memory untuk durasi proses (tidak dipersist).

## Tiket

| # | Tiket | Blocked by |
|---|---|---|
| 01 | [Klasifikasi error & envelope retry](issues/01-error-taxonomy-retry-envelope.md) | — |
| 02 | [Retry exponential backoff](issues/02-retry-exponential-backoff.md) | 01 |
| 03 | [Rantai fallback antar provider](issues/03-fallback-chain.md) | 01, 02 |
| 04 | [Kesadaran batas konteks](issues/04-context-length-routing.md) | 01, 03 |
| 05 | [Status kesehatan & circuit breaker](issues/05-health-status-circuit.md) | 03 |
| 06 | [Hardening IO (flaky sqlite test)](issues/06-hardening-sqlite-io.md) | — |
| 07 | [Parity, dokumentasi, penutupan](issues/07-parity-docs-closure.md) | 01–06 |

01 adalah fondasi (tak bergantung); 06 (hardening sqlite) independen dan bisa
dikerjakan paralel kapan saja. 02–05 paralel setelah 01; 07 terakhir.

## Invariant yang tetap berlaku

Semua invariant di `docs/ROADMAP.md` berlaku tanpa kecuali. Dua yang paling
relevan: credential terredaksi di semua jalur output, dan partial turn tidak
pernah disimpan saat cancellation. Fallback/retry tidak boleh mengintroduksi
jalur credential baru (setiap provider tetap memakai `key_env`/`model.api_key`
sendiri) dan harus menyerah dengan bersih pada `Cancelled`/SIGINT (exit 130).

## Pertanyaan desain yang akan diputuskan saat coding (dicatat agar ter-review)

1. Apakah fallback memulai ulang dari `turns` yang sama ke provider berikutnya,
   atau meneruskan sebagian output? (Keputusan awal: mulai bersih ke provider
   berikutnya, karena provider belum menghasilkan output jika kita hanya retry
   error pra-stream.)
2. Nilai retry/backoff default dan cara konfigurasi (bounded max retries).
3. Ambang waktu timeout reqwest yang akan dipasang (30 dtk mengikuti `_timeout`).
4. Semantik mid-stream error: apakah stream kedua mode menghasilkan sebagian
   chunk sebelum gagal — batas slice memutuskan ini di luar cakupan.
