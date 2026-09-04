# 02: Resolusi key_env per provider

**What to build:** Setiap provider mengambil API key dari variabel lingkungan yang disebut field `key_env`-nya sendiri, bukan dari `OPENAI_API_KEY` yang hard-coded.

**Blocked by:** 01 — Provider registry.

**Status:** todo

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

- [ ] Nama variabel lingkungan diambil dari `key_env` provider yang dipilih.
- [ ] Urutan fallback terdefinisi eksplisit: `key_env` → `model.api_key` → error.
- [ ] `key_env` yang kosong/tidak diset menghasilkan error yang **menyebut nama variabelnya**, bukan nilainya.
- [ ] Key disimpan sebagai `SecretString` sepanjang jalur; tidak pernah jadi `String` polos.
- [ ] Pesan error dan log lolos `redact_credentials` — teruji, bukan diasumsikan.
- [ ] Test tidak membutuhkan credential nyata (pakai nilai dummy dan guard variabel lingkungan per-test).

## STRIDE (wajib per invariant — ini surface credential baru)

- **Spoofing:** `key_env` bisa diarahkan ke variabel milik provider lain.
  Putuskan apakah nama variabel dibatasi pola tertentu.
- **Information disclosure:** error "key not found" adalah vektor bocor paling
  umum. Pastikan nama variabel boleh muncul, nilainya tidak.
- **Repudiation / Elevation:** tidak ada surface eksekusi baru di tiket ini.

Catat hasilnya di `docs/SECURITY.md`, jangan hanya di tiket.
