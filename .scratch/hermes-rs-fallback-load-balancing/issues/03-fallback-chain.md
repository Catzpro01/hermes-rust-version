# 03: Rantai fallback antar provider

**What to build:** Ketika provider aktif gagal (setelah retry tiket 02 habis),
Hermes-RS otomatis mencoba provider berikutnya dalam rantai fallback yang
terkonfigurasi — tanpa mengubah `ConversationRunner`/`repl` (transparan sebagai
satu `Provider`).

**Blocked by:** 01, 02.

**Status:** done — commit di VM, 147/147 test hijau (`cargo test --workspace`),
`clippy --workspace --all-targets -D warnings` bersih.

## Kondisi sekarang (terverifikasi)

`ConversationRunner<P: Provider>` memegang satu `provider: P` (spec 005).
`ProviderRegistry::select` memilih satu provider dari precedence
CLI → `model.provider` → `fake`. Tidak ada konsep "daftar provider dicoba
berurutan". `HttpProvider` memakai `key_env`/`model.api_key` sendiri dan punya
retry pra-stream (tiket 02).

## Konsep (dikunci)

Fallback dibangun sebagai **wrapper `Provider`** baru: `FallbackProvider`
memegang daftar hop berurutan `(nama, Box<dyn Provider>)` dan men-delegasi ke
masing-masing. Setiap hop tetap memakai **credentialnya sendiri** (isolasi per
hop — key provider A tidak pernah sampai ke endpoint B). Saat satu hop gagal,
turn dicoba ulang **dari awal** pada hop berikutnya dengan `turns` yang sama
(tidak ada partial output carry). `ConversationRunner` & REPL melihat satu
`Box<dyn Provider>`.

Rantai diambil dari **`model.fallback_chain: [b, c]`** (level model, global —
keputusan Matt; ini strategi level sesi). Hanya aktif bila provider aktif adalah
entri `providers:` terdaftar. Manual `/provider <name>` tetap lewat
`registry.select` (single-provider), jadi pilihan eksplisit pengguna **bypasses**
fallback.

## Kriteria

- [x] `config.yaml` mendukung `model.fallback_chain: [b, c]` (`ModelConfig`),
      default kosong = tanpa fallback otomatis.
- [x] Nama hop yang tidak dikenal **ditolak ketat saat startup** (unknown name
      → `RegistryError::UnknownProvider` + daftar yang tersedia), bukan dilewati
      diam-diam.
- [x] `FallbackProvider` mengimplementasikan `Provider`, transparan bagi runner.
- [x] Setiap provider dalam rantai memakai credentialnya sendiri (teruji: key A
      tak pernah sampai ke B & sebaliknya).
- [x] Hop fallback yang gagal **diinisialisasi** (mis. `key_env` kosong) dilewati
      dengan bersih; rantai / provider aktif yang sehat tetap dipakai — tapi
      namanya harus terdeklarasi (strict di atas).
- [x] Jika seluruh rantai gagal → error agregat `ProviderError::Fallback` yang
      menyebut **nama provider** yang benar-benar dicoba (urutan hop).
- [x] Fallback mencoba hop berikutnya dengan **turns yang sama dari awal**.
- [x] `Cancelled` pada hop mana pun **keluar segera**, tidak pernah jatuh ke hop
      berikutnya (token sudah cancel sebelum hop → tidak ada hop yang disentuh).
- [x] Semua anggota rantai memakai `SecretString`/redaction yang sama (lewat
      `HttpProvider` masing-masing).

## STRIDE

- **Spoofing/credential confusion:** hop dibangun lewat registry yang sama,
  jadi tiap hop tetap melewati `resolve_api_key` (pin `key_env` ketat; tak ada
  cross-leak). Teruji dua server wiremock dengan header `authorization` masing-
  masing. Error agregat hanya menyebut nama, tak pernah nilai key.
- **Information disclosure:** `ProviderError::Fallback` memuat daftar nama hop,
  tanpa body error/credential. Redaction tetap di lapisan `HttpProvider`.
- **DoS:** retry tiap hop dibatasi policy (tiket 02, default 3); rantai terbatas
  jumlah hop dari config. Tidak ada retry loop tak terbatas lintas-hop.
- Tidak ada surface eksekusi baru.

## Pengujian (di atas 132 → 147, +15)

- Unit `provider/fallback.rs` (+7): pakai primary & hop kedua tak disentuh;
  pindah ke hop berikutnya saat primary gagal; pindah bahkan pada error permanen
  non-retryable (fallback lintas endpoint beda); semua gagal → agregat menamai
  urutan hop; `Cancelled` hop → berhenti tanpa jatuh; token sudah-cancel → tak
  ada hop disentuh; `provider_names()` urut.
- Unit `provider/registry.rs` (+5): `select_with_fallback` offline→fake tunggal;
  nama fallback tak dikenal ditolak; rantai >1 hop ter-bangun saat keduanya
  sehat; fallback yang salah-init dilewati (tetap single, tidak gagalkan
  startup); precedence CLI menang.
- Unit `provider/mod.rs`: `ProviderError::Fallback` tidak `is_retryable`.
- Integrasi `tests/provider_fallback_integration.rs` (wiremock, +3):
  1. A persistent 500 → sukses via B; B's "hello-from-b" dikonsumsi; isolasi key
     diverifikasi di kedua server (A hanya lihat a-key, B hanya b-key).
  2. Alur `ConversationRunner` + `SessionStore`: jawaban B ("stored-from-b")
     adalah satu-satunya teks assistant yang tersimpan di `state.db` (dibaca
     ulang dari disk), dan teks provider gagal tidak ikut tersimpan.
  3. Semua hop down → `ProviderError::Fallback { tried: ["a","b"] }`.

## Perubahan

- `crates/hermes-core/src/config/schema.rs`: field `model.fallback_chain:
  Vec<String>` (struct `ModelConfig` + deserializer `ModelMap`), default kosong.
- `crates/hermes-core/src/provider/mod.rs`: varian `ProviderError::Fallback
  { tried }` (non-retryable), `pub mod fallback;` + re-export `FallbackProvider`.
- `crates/hermes-core/src/provider/fallback.rs` (baru): `FallbackProvider`
  (hops + `first_available`), semantik hop-retry / cancel / agregat.
- `crates/hermes-core/src/provider/http.rs`: override trait `chat_with_cancel`
  agar `Box<dyn Provider>` benar-benar menghormati `CancellationToken` (sebelum
  ini dispatch dinamis jatuh ke default `self.chat()` dengan token baru —
  cancel diabaikan) sambil tetap tool-aware. Ini prasyarat agar cancellation
  bisa menembus hop fallback HTTP secara nyata.
- `crates/hermes-core/src/provider/registry.rs`: `select_with_fallback(...)`
  (startup): aktif via precedence sama, rantai = aktif + `fallback_chain`,
  strict reject nama tak dikenal, drop hop yang gagal-init, bungkus bila >1 hop.
  `select` tetap utuh untuk `/provider` manual.
- `crates/hermes-cli/src/main.rs`: startup memakai `select_with_fallback`;
  REPL `/provider` tetap `select` (bypass fallback).
- `crates/hermes-core/tests/provider_fallback_integration.rs` (baru).

## Catatan desain

- Semantik hop: retry tiap hop = policy-nya sendiri (tiket 02). Setelah itu,
  error apa pun (retryable habis atau permanen) → lanjut hop berikutnya, karena
  fallback lintas endpoint berbeda bisa menolong bahkan pada error permanen yang
  lokal ke endpoint A. `Cancelled` selalu berhenti total.
- Bila hanya aktif yang tersisa (tidak ada hop fallback sehat), dikembalikan
  provider tunggal tanpa wrapper — sesi single-provider tanpa indirection.
- Fallback hanya bermakna antar `providers:` terdaftar; bila aktif lewat
  model-level fallback, rantai diabaikan (perilaku sama dengan `select`).
- Belum ada knob `RetryPolicy` di config untuk hop fallback (hop memakai default
  injectable). Calon lanjutan bila dibutuhkan.
- Retry/fallback tetap **pra-stream**; gagal di tengah stream (partial turn)
  tetap di luar scope (lihat README Spec 006).

## Dependency

01, 02.
