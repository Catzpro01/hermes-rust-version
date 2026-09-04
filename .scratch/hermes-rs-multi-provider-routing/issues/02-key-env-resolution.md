# 02: Resolusi key_env per provider

**What to build:** Setiap provider mengambil API key dari variabel lingkungan yang disebut field `key_env`-nya sendiri, bukan dari `OPENAI_API_KEY` yang hard-coded.

**Blocked by:** 01 — Provider registry.

**Status:** done — commit di VM, 117/117 test hijau (`cargo test --workspace`, 2x stabil), `clippy --workspace --all-targets -D warnings` bersih.

## Kondisi sekarang (terverifikasi)

`main.rs` mengabaikan `key_env` sepenuhnya dan memakai urutan tetap:

```rust
let key = std::env::var("OPENAI_API_KEY")
    .or_else(|_| std::env::var("HERMES_API_KEY"))
    .ok()
    .or_else(|| config.model.api_key.as_ref().map(|k| k.expose().to_owned()))
```

Field `ProviderConfig.key_env: Option<String>` sudah diparse tapi tidak
pernah dibaca.

## Kriteria

- [x] Nama variabel lingkungan diambil dari `key_env` provider yang dipilih. Provider terkonfigurasi dibangun lewat `build_configured` → `resolve_api_key(name, provider, fallback_key)`.
- [x] Urutan fallback terdefinisi eksplisit: `key_env` → `model.api_key` → error. Lihat matriks keputusan di bawah.
- [x] `key_env` yang kosong/tidak diset menghasilkan error yang **menyebut nama variabelnya**, bukan nilainya (`environment variable 'X' is not set or empty`).
- [x] Key disimpan sebagai `SecretString` sepanjang jalur; tidak pernah jadi `String` polos. `model.api_key` sudah `SecretString`; hasil `resolve_api_key` berupa `SecretString`.
- [x] Pesan error dan log lolos `redact_credentials` — teruji. `missing_key_env_names_the_variable_not_a_value` dan `pinned_but_empty_key_env_does_not_fall_back_to_model_key` memastikan tidak ada nilai yang bocor.
- [x] Test tidak membutuhkan credential nyata (pakai nilai dummy dan guard variabel lingkungan per-test).

## STRIDE (wajib per invariant — surface credential baru)

Dokumentasi penuh ditulis di `docs/SECURITY.md` (§ "Provider credentials").

- **Spoofing / credential confusion:** `key_env` yang dikunci (pinned) tapi
  variabelnya kosong/tidak diset menghasilkan **error**, BUKAN fallback ke
  `model.api_key`. Matriks perilaku yang diputuskan (aman):

  | key_env di YAML | Env di OS | model.api_key di YAML | Hasil |
  |---|---|---|---|
  | `ANTHROPIC_KEY` | terisi (`sk-ant-...`) | (diabaikan) | pakai `ANTHROPIC_KEY` |
  | `ANTHROPIC_KEY` | kosong/unset | ada (`sk-openai-...`) | ❌ ERROR — sebut nama var. Tidak kirim key OpenAI ke endpoint Anthropic |
  | (absen) | (diabaikan) | ada | fallback ke `model.api_key` |
  | (absen) | (diabaikan) | kosong | ❌ ERROR — `no 'key_env' declared and no 'model.api_key' fallback configured` |

- **Information disclosure:** pesan error menyebut nama variabel, tidak pernah
  nilainya; `SecretString` mencegah nilai masuk log/output.
- **Repudiation / Elevation:** tidak ada surface eksekusi baru di tiket ini.

## Keputusan tambahan

- **Legacy path dipertahankan.** `model_level_fallback` (config gaya lama tanpa
  section `providers:`) tetap memakai `OPENAI_API_KEY` → `HERMES_API_KEY` →
  `model.api_key` sebagai jalur kompatibilitas. Rantai `key_env → model.api_key
  → error` berlaku untuk provider terkonfigurasi.
- **Flaky SQLite test diperbaiki.** `concurrent_writes_to_same_sqlite_session_are_serialized`
  di-robust-kan dengan retry-berbounding pada kondisi busy/locked (`DatabaseBusy`
  / `DatabaseLocked`), backoff naik; tidak mengubah semantik produksi. Diverifikasi
  stabil 2x di full-suite.

## Perubahan

- `crates/hermes-core/src/provider/registry.rs`: `build_configured` dan
  `resolve_api_key` menerima `fallback_key: Option<SecretString>` (captured dari
  `config.model.api_key` di `from_config`); `resolve_api_key` mengimplementasikan
  matriks di atas + 3 test baru.
- `docs/SECURITY.md`: tambah § "Provider credentials (Spec 005)" dengan rantai
  fallback + analisis STRIDE.
- `crates/hermes-core/tests/sqlite_parity_integration.rs`: test koncurrency
  di-robust-kan (helper `save_turn_retry_busy`).

## Pengujian

- `cargo test --workspace` → 117/117 (naik dari 114; +3 registry), stabil pada
  2 run penuh.
- `clippy --workspace --all-targets -D warnings` → bersih.
