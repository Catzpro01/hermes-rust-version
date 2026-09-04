# 01: Klasifikasi error & envelope retry

**What to build:** Fondasi Spec 006 — kemampuan membedakan error "transien yang
bisa di-retry" dari error "permanen/parah" pada `ProviderError`, plus timeout
reqwest yang benar-benar dipasang di `HttpProvider`.

**Blocked by:** —

**Status:** done — commit di VM, 126/126 test hijau (`cargo test --workspace`),
`clippy --workspace --all-targets -D warnings` bersih.

## Kondisi sekarang (terverifikasi)

```rust
// crates/hermes-core/src/provider/mod.rs
pub enum ProviderError {
    Message(String),          // transport/send error — tidak tahu apakah retry-able
    Http { status: u16, message: String }, // punya status code, belum diklasifikasi
    Cancelled,
}
```

`crates/hermes-core/src/provider/http.rs` membangun `reqwest::Client::new()`
tanpa timeout; `fn _timeout() -> Duration { 30 }` adalah dead code
(`#[allow(dead_code)]`). Tidak ada cara mengetahui permintaan gagal karena
timeout vs karena respon error.

## Kriteria

- [x] `ProviderError` punya klasifikasi eksplisit apakah suatu error retry-able:
      `ProviderError::is_retryable(&self)`; varian `Timeout` baru ditambahkan
      agar timeout (retry-able) terbedakan dari `Message` generik (tidak
      retry-able).
- [x] Ambang status transien disebut eksplisit (429, 500, 502, 503, 504) dan
      disimpan sebagai konstanta `pub const RETRYABLE_HTTP_STATUS: &[u16]`;
      test `retryable_status_codes_are_exact` mengunci nilai array.
- [x] `HttpProvider` memasang timeout nyata 30 dtk (`HTTP_CLIENT_TIMEOUT`) pada
      `Client` via `default_client()`, menggantikan `_timeout()` dead code;
      error timeout `request.send()` dipetakan ke `ProviderError::Timeout`.
      Test `http_client_timeout_is_thirty_seconds` mengunci nilai.
- [x] `Cancelled` tidak pernah diklasifikasikan retry-able (test
      `timeout_is_retryable_but_cancelled_is_not`).
- [x] Klasifikasi tidak mengubah jalur redaction credential: `Message`/`Http`
      masih lewat `redact`; `Timeout` tak membawa isi pesan.

## STRIDE

- **Information disclosure:** pesan HTTP tetap lewat redact; `Timeout` tanpa
  isi pesan tidak membocorkan apa pun.
- **DoS / Elevation:** timeout nyata 30 dtk menghilangkan risiko hang tanpa
  batas (sebelumnya request bisa menggantung selamanya). Tidak ada eksekusi
  baru.

## Risiko yang ditangani

- Timeout 30 dtk mengubah perilaku `chat` normal (sebelumnya tanpa batas).
  Test streaming lama tetap lulus; SIGINT path tak berubah (timeout jauh lebih
  kecil pengaruhnya karena cancel di `tokio::select!`).

## Pengujian (di atas 119 sebelumnya → 126)

- Unit `mod.rs`: `retryable_status_codes_are_exact`,
  `non_retryable_http_and_others_are_not_retried`, `timeout_is_retryable_but_cancelled_is_not`.
- Unit `http.rs`: `http_client_timeout_is_thirty_seconds`.
- Integrasi `provider_http_integration.rs` (wiremock):
  `transient_http_errors_are_retryable` (429/503 → `Http` retryable),
  `permanent_http_errors_are_not_retryable` (400/401 → tidak retryable),
  `transport_timeout_yields_a_retryable_timeout` (client short-timeout di-inject
  via `with_client` terhadap server ber-delay → `ProviderError::Timeout`).

## Perubahan

- `crates/hermes-core/src/provider/mod.rs`: varian `ProviderError::Timeout`,
  `RETRYABLE_HTTP_STATUS`, `is_retryable()`, unit test.
- `crates/hermes-core/src/provider/http.rs`: `HTTP_CLIENT_TIMEOUT`,
  `default_client()`, mapping send-timeout → `Timeout`, hapus `_timeout()`.
- `crates/hermes-core/tests/provider_http_integration.rs`: 3 test wiremock baru.
