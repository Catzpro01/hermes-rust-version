# 01: Config `mcp_servers` + model + redaksi + STRIDE

**Status:** done. Commit `e69c4dd`; 254/254 hijau saat itu (akhirnya 273/273),
clippy bersih.

**What to build:** Bagian config & model untuk MCP, plus model ancaman STRIDE
untuk surface eksekusi baru (menjalankan child process MCP). Belum spawn
process / belum transport; tiket ini murni data model + validasi + dokumentasi
keamanan yang testable tanpa eksekusi.

**Blocked by:** —

## Kondisi sekarang (terverifikasi)

- `HermesConfig` (`crates/hermes-core/src/config/schema.rs`) memakai
  `#[serde(default)]` per-field; field baru tak mengganggu config lama.
  `SecretString` menyediakan redaksi `Debug`.
- Tidak ada model MCP sama sekali.

## Desain config

```yaml
mcp_servers:
  github:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-github"]
    env:
      GITHUB_PERSONAL_ACCESS_TOKEN: "..."
    # opsional: lewat gate konfirmasi Spec 002 utk tiap tool server ini
    confirm: false
```

Rust model:

```rust
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)] pub args: Vec<String>,
    #[serde(default)] pub env: HashMap<String, String>, // nilai bisa secret
    #[serde(default)] pub confirm: bool,
}
// di HermesConfig: #[serde(default)] pub mcp_servers: HashMap<String, McpServerConfig>
```

## Kriteria

- [ ] `mcp_servers` default kosong → tidak ada efek (backward compatible).
- [ ] Parse YAML utk skema di atas; field `command` wajib non-empty (error
      parse bila kosong), `args`/`env`/`confirm` default aman.
- [ ] `Debug`/render tidak pernah membocorkan nilai `env` (nama server & kunci
      env boleh tampil, nilai diredaksi) — reuse pola `SecretString`.
- [ ] Validasi: nama server unik (HashMap sudah menjamin); `command` non-empty;
      (opsional) peringatan bila `env` berisi var yg menyerupai token.
- [ ] Tidak spawn / tidak jalankan apa pun di tiket ini.
- [ ] STRIDE section ditulis (lihat bawah) & test unit lulus; clippy bersih.

## STRIDE (execution surface baru)

- **Spoofing:** `command`/`args`/`env` dari config user (input tepercaya,
  setara provider). Tidak ada nilai secret yang di-log/display.
- **Tampering:** config read-only; Hermes tidak menulis config.
- **Repudiation / non-repudiation:** (n/a eksekusi child tak menghasilkan
  jejak ke db di tiket ini; jejak tool muncul saat eksekusi di tiket 04 via
  `ToolCallRecord`).
- **Information disclosure:** nilai `env` (token) diredaksi pada semua jalur
  output/error/log; hanya nama server & nama kunci env yang boleh tampil.
- **Denial of service:** spawn child baru hanya saat startup & sesuai jumlah
  server di config; tidak ada loop spawn. (Batasan timeout eksekusi tiap tool
  di tiket 04.)
- **Elevation of privilege:** tool MCP adalah kapabilitas server yang
  user-config-kan. `Denied` (Spec 002) tak di-bypass; `confirm: true` bisa
  menuntut konfirmasi. Dokumentasi: config MCP server = perintah tepercaya
  yang bisa menjalankan apa pun → pengguna harus tahu server yang dipasang.

## Dependency

— (config murni).
