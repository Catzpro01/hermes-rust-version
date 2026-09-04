# 02: Retry exponential backoff

**What to build:** Mekanisme retry cerdas untuk error transien sebelum trigger
fallback: coba ulang beberapa kali dengan jeda membesar, lalu menyerah (supaya
pemanggil bisa trigger fallback).

**Blocked by:** 01 — Klasifikasi error & envelope retry.

**Status:** todo

## Kondisi sekarang (terverifikasi)

Tidak ada loop retry. `HttpProvider::chat_with_cancel` memanggil `request.send()`
sekali; error langsung di-`return Err(...)`. `ConversationRunner::chat_agentic`
meneruskan `Err(err)` ke atas tanpa mencoba ulang.

## Kriteria

- [ ] Retry hanya untuk error yang `is_retryable()` (dari tiket 01). Error
      permanen dan `Cancelled` langsung diteruskan, tidak di-retry.
- [ ] Exponential backoff berbatas (bounded max retries) dengan delay membesar
      (mis. `base * 2^attempt`, batas `max_retries` default terdefinisi).
- [ ] Parameter retry (max retries, base delay, max delay) terkonfigurasi lewat
      `config.yaml` dengan default terdokumentasi.
- [ ] Retry menghormati `CancellationToken`: jika dibatalkan di tengah backoff,
      langsung `Err(ProviderError::Cancelled)` dan keluar bersih (invariant:
      tidak pernah menyimpan partial turn, SIGINT → exit 130).
- [ ] Setelah retry habis, error terakhir diteruskan agar caller bisa fallback.
- [ ] Jeda antar retry memakai **`tokio::time::sleep()`**, bukan
      `std::thread::sleep()`. `std::thread::sleep` di dalam konteks async akan
      memblok seluruh worker thread dan bisa mendeadlock MockServer saat test.

## STRIDE

- **DoS:** max retries yang dibatasi mencegah retry tak terbatas membanjiri
  endpoint yang bermasalah.
- Tidak ada surface credential/eksekusi baru.

## Risiko

- Interaksi retry vs fallback: retry untuk satu provider, kalau habis, baru
  pindah ke provider berikutnya (tiket 03). Batasnya harus jelas agar tidak
  "retry di semua provider sekaligus".
- Konfigurasi tambahan menambah permukaan parse; error config harus tetap
  ketat seperti `api_mode` (tiket 03 Spec 005).

## Dependency

01 (klasifikasi retry-able).
