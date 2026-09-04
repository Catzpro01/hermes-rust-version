# 01: Provider registry dan resolusi berbasis config

**What to build:** Sebuah registry yang membaca `providers` dari `config.yaml` dan menghasilkan `Box<dyn Provider>` untuk nama provider yang diminta, menggantikan `match` hard-coded di `main.rs`.

**Blocked by:** tidak ada — Spec 001 sudah mem-parse `providers`.

**Status:** done — commit di VM, 98/98 test hijau, `clippy -D warnings` bersih.

## Catatan implementasi

- `ProviderRegistry` menyimpan **factory**, bukan provider jadi. `from_config`
  tidak melakukan I/O dan tidak membaca environment, jadi satu provider yang
  salah konfigurasi tidak bisa menggagalkan startup atau menjatuhkan provider
  yang sedang aktif.
- **Penyimpangan dari acceptance criteria #1.** Kriteria menyebut
  `HashMap<String, Box<dyn Provider>>`. Itu dihindari dengan sengaja:
  membangun semua provider di muka berarti `from_config` gagal kalau satu saja
  `key_env` kosong, dan itu bertabrakan langsung dengan kriteria #4 di tiket 04
  (provider aktif tidak boleh rusak saat init provider lain gagal).
- `ApiMode` ditambahkan sebagai enum bertag dengan `ChatCompletions` sebagai
  `#[default]`, sesuai catatan backward-compatibility. Routing endpoint-nya
  tetap milik tiket 03.
- **Utang yang sengaja ditinggal:** `--api-url` hanya berlaku pada jalur
  fallback model-level, belum pada provider yang dideklarasikan di
  `providers:`. Menimpa base URL provider terkonfigurasi disatukan dengan
  routing endpoint di tiket 03.
- `model.provider: auto` diperlakukan sebagai "belum dipilih", bukan nama
  provider. Perilaku lama (`--provider openai` + `--api-url`) dipertahankan
  lewat fallback model-level, sehingga `sigint_stream.rs` tidak berubah.

## Kondisi sekarang (terverifikasi)

`crates/hermes-core/src/config/schema.rs` sudah mem-parse semuanya:

```rust
pub struct ProviderConfig {
    pub api: Option<String>,
    pub name: Option<String>,
    pub api_mode: Option<String>,
    pub key_env: Option<String>,
    pub models: HashMap<String, serde_yaml::Value>,
    pub context_length: Option<u64>,
}
```

Tapi `crates/hermes-cli/src/main.rs` **tidak pernah membacanya**. Ia hanya
mencocokkan literal:

```rust
let provider: Box<dyn Provider> = match args.provider.as_str() {
    "fake" => Box::new(FakeProvider),
    "openai" | "custom" => { /* base_url + OPENAI_API_KEY hard-coded */ }
    other => anyhow::bail!("unsupported provider '{other}' (use fake, openai, or custom)"),
};
```

Artinya provider apa pun yang didefinisikan di `config.yaml` saat ini
tidak bisa dipakai.

## Kriteria

- [x] Provider dapat dipilih lewat nama kunci di `providers`, bukan daftar literal.
- [x] `fake` tetap tersedia tanpa `config.yaml` (dipakai test dan slice offline).
- [x] Nama provider yang tidak ada menghasilkan error yang menyebut nama-nama yang tersedia.
- [x] `--provider` CLI dan `model.provider` dari config punya precedence yang terdefinisi dan teruji.
- [x] Registry mengembalikan `Box<dyn Provider>` tanpa mengubah trait `Provider`.
- [x] Conversation runner tidak berubah sama sekali.

## Catatan

`ProviderConfig.models` bertipe `HashMap<String, serde_yaml::Value>` — nilai
tidak terstruktur. Kalau spec ini butuh field model tertentu (mis.
`context_length` per model), putuskan di sini apakah mau diketik ketat atau
dibiarkan longgar, dan catat keputusannya sebagai ADR.
