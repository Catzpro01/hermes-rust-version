# Spec 011 — Model Context Protocol (MCP) Client

Vertical slice: menambahkan **MCP Client** sehingga Hermes-RS bisa memakai
ribuan server MCP yang sudah ada (GitHub, PostgreSQL, Brave Search, Slack,
dll) secara transparan lewat `ToolRegistry` yang sudah dibangun di Spec 002/009.
Kita **tidak** menulis tool satu per satu — kita `tools/list` dari child
process MCP server, membungkus tiap tool yang ditemukan menjadi implementasi
`Tool` trait, dan mendaftarkannya ke registry saat startup.

Ini adalah **execution surface baru** (menjalankan child process MCP server +
tool yang dieksekusikan ke child) → wajib model ancaman STRIDE (invariant
ROADMAP: "New execution or network surface requires a STRIDE threat model").

## Motivasi (terverifikasi dari kode)

- `Tool` trait (`crates/hermes-core/src/tools/mod.rs`): `name()`, `description()`,
  `async fn execute(&self, call: &ToolCall, cancel) -> Result<ToolResponse, ToolError>`.
  `ToolRegistry` memegang `HashMap<String, Box<dyn Tool>>`, `register()`, `get()`,
  `execute()` yang memetakan `ToolError::Unknown` bila tool tak ada.
- `ToolCall { id: Option<String>, name: String, arguments: String }` — `arguments`
  adalah teks (untuk MCP harus berupa objek JSON, karena MCP `tools/call` butuh
  `arguments` bertipe objek).
- Registrasi tool saat ini di `crates/hermes-cli/src/repl.rs` (≈ baris 113):
  `ToolRegistry::new()` lalu `register(ReadFileTool/ListDirTool/ShellReadonlyTool/WriteFileTool)`.
  Startup REPL adalah titik injeksi MCP tool.
- Config (`crates/hermes-core/src/config/schema.rs`): `HermesConfig` punya
  banyak field `#[serde(default)]`; menambah `mcp_servers` baru aman & forward-compatible
  (field tak dikenal di-ignore). Ada `SecretString` utk redaksi.
- Dependency siap: `tokio` sudah enable feature `process`; `serde_json` ada;
  `async-trait`, `futures`, `tokio-util` (CancellationToken) ada. **Tidak perlu**
  dependency berat (wasmtime/extism) — transport MCP cukup JSON-RPC 2.0
  newline-delimited (NDJSON) over stdio.
- Transport MCP standar: JSON-RPC 2.0, framing = 1 pesan JSON per baris (LSP-style,
  `Content-Length` tidak dipakai; MCP stdio transport memakai newline-delimited).

## Keputusan scope awal (untuk direview @matt)

- **Off by default / backward compatible.** `mcp_servers:` default kosong → tidak
  ada child process, tidak ada tool tambahan; startup & mode reaktif tidak
  berubah (zero regression). Tool MCP baru muncul hanya bila user menambah
  server di `config.yaml`.
- **Config = input terpercaya.** Sama seperti provider, `command`/`args`/`env`
  MCP server berasal dari config user (dianggap tepercaya). Namun ini tetap
  **execution surface baru** (spawn arbitrary command) → dianotasikan eksplisit
  di STRIDE & SECURITY; nilai `env` yang mengandung secret diredaksi pada semua
  jalur output/log.
- **Tool MCP tidak diklasifikasikan read/write.** MCP server tidak selalu memberi
  isReadOnly. Opsi konfirmasi per-server (`confirm: true`) ditambahkan bila user
  ingin tool dari server tsb lewat gate konfirmasi Spec 002; default mengikuti
  perilaku ekosistem MCP (server yang di-config dianggap diizinkan).
- **Transport seam untuk testability.** Implementasi protokol JSON-RPC dibangun
  di atas trait transport kecil (`McpTransport`: tulis/baca frame NDJSON) dengan
  dua impl: `StdioTransport` (child process sungguhan, dipakai produksi) dan
  transport in-memory/pipe (dipakai unit test deterministik tanpa butuh `npx`).
- **Tidak ada network yang dibuka Hermes sendiri.** Hermes-RS hanya stdio ke
  child process; koneksi jaringan (jika ada) dilakukan oleh server MCP sendiri
  sesuai config-nya.

## Tiket

| # | Tiket | Blocked by |
|---|---|---|
| 01 | [Config `mcp_servers` + model + redaksi + STRIDE](issues/01-mcp-config-model-stride.md) | — |
| 02 | [JSON-RPC 2.0 stdio transport + handshake `initialize`](issues/02-jsonrpc-stdio-transport.md) | 01 |
| 03 | [Spawn child + discovery `tools/list` → `Tool` wrappers + daftar ke `ToolRegistry`](issues/03-discovery-tool-registration.md) | 02 |
| 04 | [Eksekusi `tools/call` + mapping status + cancell/timeout + graceful shutdown](issues/04-execution-tools-call-shutdown.md) | 03 |
| 05 | [Parity, docs, E2E closure proof](issues/05-parity-docs-closure.md) | 01–04 |

01 memodelkan config & security boundary; 02 fondasi transport JSON-RPC;
03 discovery & bungkus ke Tool trait; 04 eksekusi + lifecycle; 05 penutupan.

## Invariant yang tetap berlaku

Semua invariant `docs/ROADMAP.md` tetap: `state.db` canonical; SIGINT exit 130;
credential terredaksi pada semua jalur; turn yang dibatalkan tidak dipersist
parsial; tak ada fake `User` turn; execution surface baru wajib STRIDE.
Tool MCP yang terdaftar akan terlihat & dieksekusi lewat agentic loop Spec
002/009 persis seperti tool bawaan; `ToolError::Denied` tetap tak di-bypass.
