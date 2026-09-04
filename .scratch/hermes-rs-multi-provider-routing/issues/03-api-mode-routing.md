# 03: Routing api_mode — chat_completions vs completions

**What to build:** `HttpProvider` memilih endpoint dan bentuk request/response berdasarkan `api_mode` provider, bukan selalu `v1/chat/completions`.

**Blocked by:** 01 — Provider registry.

**Status:** done — commit di VM, 107/107 test hijau (`cargo test --workspace`), `clippy --workspace --all-targets -D warnings` bersih.

## Kondisi sekarang (terverifikasi)

`crates/hermes-core/src/provider/http.rs` hard-coded satu endpoint:

```rust
.base_url
.join("v1/chat/completions")
```

`api_mode` sudah diparse ke `ProviderConfig` tapi tidak pernah dibaca. Tidak
ada jalur kode untuk mode `completions`.

## Kriteria

- [x] `api_mode` menjadi tipe bertag (enum), bukan `Option<String>` bebas yang dicocokkan di banyak tempat.
      `ProviderConfig.api_mode` kini `Option<ApiMode>`; enum `ApiMode` unit variant dengan `serde(rename_all = "snake_case")`.
- [x] Nilai tak dikenal ditolak saat load config, dengan error yang menyebut nilai yang valid — bukan gagal saat request pertama.
      Karena varian enum ketat, nilai selain `chat_completions`/`completions` menggagalkan `load_config` (serde) dan menyebut kedua nilai valid. Konstruksi (env/URL) tetap lazy — keputusan "hybrid" atas tiket 01.
- [x] `chat_completions`: endpoint dan payload seperti sekarang (tidak ada regresi). `HttpProvider` default-nya mode ini; pengujian wiremock lama tetap hijau.
- [x] `completions`: endpoint `v1/completions`, prompt dibentuk dari `Turn`, dan responnya dinormalisasi ke `Event` yang sama.
      `render_completions_prompt` merangkai transkrip linear berlabel peran diakhiri cue `Assistant:`; parser SSE membaca `text` (mode lama) atau `delta.content` (chat) → `Event::Chunk`.
- [x] Nilai default terdefinisi dan terdokumentasi kalau `api_mode` absen. Default = `chat_completions` (`#[default]` + `unwrap_or_default`), kompatibel ke belakang.
- [x] Streaming SSE kedua mode menghasilkan urutan `Event` yang identik untuk input setara. Diuji `both_modes_normalize_to_identical_event_streams`.
- [x] Kedua mode diuji dengan `wiremock` tanpa jaringan nyata.

## Risiko normalisasi

Mode `completions` tidak punya konsep `role` per chunk. Pastikan
`tool_aware_stream` di `provider/mod.rs` tetap berfungsi: ia mem-buffer
mencari `<tool_call`, dan buffer itu tidak peduli dari mana mode chunk
berasal. Diuji eksplisit: `tool_calls_are_parsed_in_completions_mode`
mengembalikan `Event::ToolCall` dari teks `text` yang membawa XML tool tag.

## Keputusan implementasi

- **Waktu penolakan nilai tak dikenal — "hybrid".** Schema (parse seluruh file config di `load_config`) menolak `api_mode` tak dikenal; konstruksi provider (env `key_env` kosong, base URL tak valid) tetap lazy di `registry::build`. Memuaskan kriteria "reject at load" tanpa mencabut jaminan tiket 01 bahwa satu provider yang salah konfigurasi tidak menggagalkan startup.
- **Prompt mode `completions`.** Tidak ada kontrak template baku di spec; dipilih transkrip linear deterministik yang mempertahankan penulis tiap `Turn` (`User:`, `Assistant:`, `Tool result (name):`) dan berakhir dengan cue `Assistant:` agar model tahu gilirannya. Hanya berpengaruh ke mode `completions`; chat tidak berubah.
- **SSE.** `sse.rs` menormalisasi kedua bentuk payload (`delta.content` untuk chat, `text` untuk completions) ke `Event::Chunk` yang sama, sehingga `tool_aware_stream` dan pemanggilnya mode-agnostik.
