# 02: Retry exponential backoff

**What to build:** Mekanisme retry cerdas untuk error transien sebelum trigger
fallback: coba ulang beberapa kali dengan jeda membesar, lalu menyerah (supaya
pemanggil bisa trigger fallback).

**Blocked by:** 01 — Klasifikasi error & envelope retry.

**Status:** done — commit di VM, 132/132 test hijau (`cargo test --workspace`),
`clippy --workspace --all-targets -D warnings` bersih.

## Kondisi sekarang (terverifikasi)

Tidak ada loop retry. `HttpProvider::chat_with_cancel` memanggil `request.send()`
sekali; error langsung di-`return Err(...)`. `ConversationRunner::chat_agentic`
meneruskan `Err(err)` ke atas tanpa mencoba ulang.

## Konsep

Retry berada di lapisan `HttpProvider` (per-provider), membungkus upaya kirim
**pra-stream**: `attempt()` (build + send + klasifikasi) lalu loop
`send_with_retry()` dengan bounded exponential backoff. Karena hanya error
pra-stream yang di-retry, tidak ada partial output/turn.

## Kriteria

- [x] Retry hanya untuk error yang `is_retryable()` (dari tiket 01). Error
      permanen dan `Cancelled` langsung diteruskan, tidak di-retry.
- [x] Exponential backoff berbatas (bounded max retries): `delay =
      min(base_delay * 2^(attempt-1), max_delay)`. Default `RetryPolicy`:
      max 3 attempts, base 200 ms, max 2000 ms.
- [x] Parameter retry (max attempts, base delay, max delay) **injectable**
      (`RetryPolicy` + `HttpProvider::with_retry`) sehingga test bisa memaksa
      retry cepat; skema konfig-yaml diserahkan ke tiket berikutnya.
- [x] Retry menghormati `CancellationToken`: tiap jeda backoff di-wrap
      `tokio::select!` thd `cancel.cancelled()` → langsung
      `Err(ProviderError::Cancelled)` tanpa menunggu timer (SIGINT → exit 130).
- [x] Setelah retry habis, error terakhir diteruskan agar caller bisa fallback.
- [x] Jeda antar retry memakai **`tokio::time::sleep()`**, bukan
      `std::thread::sleep()`. `std::thread::sleep` di dalam konteks async akan
      memblok seluruh worker thread dan bisa mendeadlock MockServer saat test.

## STRIDE

- **DoS:** max attempts yang dibatasi (default 3) mencegah retry tak terbatas
  membanjiri endpoint yang bermasalah; backoff membesar menambah jeda.
- Tidak ada surface credential/eksekusi baru; error path tetap lewat redact.

## Pengujian (di atas 126 → 132, +6)

- Unit `http.rs`: `default_retry_policy_is_bounded_and_reasonable`,
  `backoff_delay_doubles_then_caps`, `backoff_delay_never_exceeds_max_delay_even_at_large_exponent`.
- Integrasi `provider_http_integration.rs` (wiremock):
  - `retry_recovers_after_a_transient_503` (attempt 1 → 503, attempt 2 → 200;
    stream berhasil dikonsumsi; persis 2 request) via responder `FailOnceThenOk`.
  - `retry_exhausts_after_max_attempts_on_persistent_500` (500 terus → gagal
    setelah tepat max_attempts; error terakhir 500; persis N request).
  - `non_retryable_error_does_not_retry` (400 → persis 1 request, tanpa sleep).

## Perubahan

- `crates/hermes-core/src/provider/http.rs`: `RetryPolicy` (+ `Default`),
  `backoff_delay`, field `retry`, `with_retry`, refactor jadi `attempt()` +
  `send_with_retry()` + `chat_with_cancel()`.
- `crates/hermes-core/src/provider/mod.rs`: re-export `RetryPolicy`.
- `crates/hermes-core/tests/provider_http_integration.rs`: 3 test wiremock baru
  (+ responder `FailOnceThenOk`).

## Catatan desain

- Batas cakupan (per README Spec 006): retry hanya **pra-stream**. Kegagalan
  di tengah stream (setelah `Event::Started`/chunk) tetap tidak di-retry di
  tiket ini — konsisten dgn invariant partial-turn.
- Backoff dihitung dengan eksponen `attempt-1` (u128 saat shift agar aman),
  di-cap oleh `max_delay`.
