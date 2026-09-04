# Spec 005 — Multi-Provider Runtime Routing

Vertical slice: pengguna dapat mendefinisikan beberapa provider di
`config.yaml`, memilih salah satu saat memulai CLI, dan menggantinya di tengah
session lewat `/provider <name>`.

## Batas scope (disepakati)

- Routing berdasarkan section `providers` di `config.yaml` — sudah diparse sejak Spec 001.
- `/provider <name>` untuk switch mid-session.
- Routing `api_mode` (`chat_completions` vs `completions`).
- Resolusi `key_env` per provider.
- **Tidak ada** perubahan security boundary.
- **Tidak ada** tool execution baru.

## Tiket

| # | Tiket | Blocked by |
|---|---|---|
| 01 | [Provider registry dan resolusi berbasis config](issues/01-provider-registry.md) | — |
| 02 | [Resolusi `key_env` per provider](issues/02-key-env-resolution.md) | 01 |
| 03 | [Routing `api_mode`](issues/03-api-mode-routing.md) | 01 |
| 04 | [`/provider <name>` switch mid-session](issues/04-provider-switch-repl.md) | 01 |
| 05 | [Bukti parity, dokumentasi, penutupan](issues/05-parity-docs-closure.md) | 01–04 |

01 adalah satu-satunya tiket tanpa dependensi; 02, 03, dan 04 bisa dikerjakan
paralel setelahnya.

## Celah nyata yang memotivasi breakdown ini

Semua terverifikasi dari kode, bukan asumsi:

1. `config/schema.rs` mem-parse `ProviderConfig { api, name, api_mode, key_env, models, context_length }`, tetapi `main.rs` tidak pernah membaca `config.providers` — ia mencocokkan literal `"fake" | "openai" | "custom"`.
2. `key_env` diparse tapi diabaikan; `main.rs` hard-coded `OPENAI_API_KEY` → `HERMES_API_KEY` → `model.api_key`.
3. `provider/http.rs` hard-coded `.join("v1/chat/completions")`; tidak ada jalur untuk mode `completions`.
4. `repl::run_repl` menerima satu `Box<dyn Provider>` saat startup; tidak ada mekanisme penggantian.

Jadi Spec 005 bukan menambah kemampuan baru di atas fondasi yang siap —
ia menyambungkan konfigurasi yang **sudah diparse** ke jalur eksekusi yang
selama ini mengabaikannya.

## Invariant yang tetap berlaku

Semua invariant di `docs/ROADMAP.md` berlaku tanpa pengecualian. Dua yang
paling relevan di spec ini: credential terredaksi di semua jalur output, dan
partial turn tidak pernah disimpan.
