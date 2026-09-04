# 01: Klasifikasi error & envelope retry

**What to build:** Fondasi Spec 006 — kemampuan membedakan error "transien yang
bisa di-retry" dari error "permanen/parah" pada `ProviderError`, plus timeout
reqwest yang benar-benar dipasang di `HttpProvider`.

**Blocked by:** —

**Status:** todo

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

- [ ] `ProviderError` punya klasifikasi eksplisit apakah suatu error retry-able:
      tambah helper (mis. `ProviderError::is_retryable(&self)`) atau varian baru
      yang membedakan status transien (429, 500, 502, 503, 504) dan timeout
      dari 4xx permanen / Cancelled.
- [ ] Ambang status transien disebut eksplisit (429, 500, 502, 503, 504) dan
      disimpan sebagai konstanta agar perubahan angka langsung mematahkan test.
- [ ] `HttpProvider` memasang timeout nyata pada `Client` (mengikuti nilai
      `_timeout`, 30 dtk) sehingga error timeout dapat dihasilkan dan diuji.
- [ ] `Cancelled` tidak pernah diklasifikasikan retry-able.
- [ ] Klasifikasi tidak mengubah jalur redaction credential (`Message`/`Http`
      tetap lewat `redact`).

## STRIDE (surface kecil tapi credential lewat error path lagi)

- **Information disclosure:** pesan HTTP tetap harus lolos redact. Klasifikasi
  hanyalah membaca status code; tidak menambahkan isi pesan baru.
- **DoS / Elevation:** timeout nyata justru mengurangi risiko hang; tidak ada
  eksekusi baru.

## Risiko

Timeout 30 dtk mengubah perilaku `chat` normal (sebelumnya tanpa batas). Semua
test wiremock yang sengaja menunda harus mempertimbangkan ini; test streaming
lama harus tetap lulus.
