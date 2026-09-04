# 01: Provider registry dan resolusi berbasis config

**What to build:** Sebuah registry yang membaca `providers` dari `config.yaml` dan menghasilkan `Box<dyn Provider>` untuk nama provider yang diminta, menggantikan `match` hard-coded di `main.rs`.

**Blocked by:** tidak ada — Spec 001 sudah mem-parse `providers`.

**Status:** todo

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

- [ ] Provider dapat dipilih lewat nama kunci di `providers`, bukan daftar literal.
- [ ] `fake` tetap tersedia tanpa `config.yaml` (dipakai test dan slice offline).
- [ ] Nama provider yang tidak ada menghasilkan error yang menyebut nama-nama yang tersedia.
- [ ] `--provider` CLI dan `model.provider` dari config punya precedence yang terdefinisi dan teruji.
- [ ] Registry mengembalikan `Box<dyn Provider>` tanpa mengubah trait `Provider`.
- [ ] Conversation runner tidak berubah sama sekali.

## Catatan

`ProviderConfig.models` bertipe `HashMap<String, serde_yaml::Value>` — nilai
tidak terstruktur. Kalau spec ini butuh field model tertentu (mis.
`context_length` per model), putuskan di sini apakah mau diketik ketat atau
dibiarkan longgar, dan catat keputusannya sebagai ADR.
