# 03: Routing api_mode — chat_completions vs completions

**What to build:** `HttpProvider` memilih endpoint dan bentuk request/response berdasarkan `api_mode` provider, bukan selalu `v1/chat/completions`.

**Blocked by:** 01 — Provider registry.

**Status:** todo

## Kondisi sekarang (terverifikasi)

`crates/hermes-core/src/provider/http.rs` hard-coded satu endpoint:

```rust
.base_url
.join("v1/chat/completions")
```

`api_mode` sudah diparse ke `ProviderConfig` tapi tidak pernah dibaca. Tidak
ada jalur kode untuk mode `completions`.

## Kriteria

- [ ] `api_mode` menjadi tipe bertag (enum), bukan `Option<String>` bebas yang dicocokkan di banyak tempat.
- [ ] Nilai tak dikenal ditolak saat load config, dengan error yang menyebut nilai yang valid — bukan gagal saat request pertama.
- [ ] `chat_completions`: endpoint dan payload seperti sekarang (tidak ada regresi).
- [ ] `completions`: endpoint `v1/completions`, prompt dibentuk dari `Turn`, dan responnya dinormalisasi ke `Event` yang sama.
- [ ] Nilai default terdefinisi dan terdokumentasi kalau `api_mode` absen.
- [ ] Streaming SSE kedua mode menghasilkan urutan `Event` yang identik untuk input setara.
- [ ] Kedua mode diuji dengan `wiremock` (sudah ada di dev-dependencies) tanpa jaringan nyata.

## Risiko normalisasi

Mode `completions` tidak punya konsep `role` per chunk. Pastikan
`tool_aware_stream` di `provider/mod.rs` tetap berfungsi: ia mem-buffer
mencari `<tool_call`, dan buffer itu tidak peduli dari mode mana chunk
berasal. Uji eksplisit bahwa tool call tetap terparse di mode `completions`.
