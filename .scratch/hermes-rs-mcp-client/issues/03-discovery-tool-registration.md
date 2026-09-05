# 03: Spawn child + discovery `tools/list` → `Tool` wrappers → `ToolRegistry`

**What to build:** Menghidupkan child process MCP server (dari config tiket 01),
menjalankan handshake (tiket 02), meminta `tools/list`, membungkus tiap tool
yang ditemukan menjadi implementasi `Tool` trait Hermes, dan mendaftarkannya ke
`ToolRegistry` saat startup REPL.

**Blocked by:** 02

## Desain

- `McpManager` (baru, di `hermes-core/src/mcp/manager.rs` atau `mcp/mod.rs`):
  memegang satu `McpClient` + `McpSession` per server dari `mcp_servers`.
- **Discovery:** `tools/list` → daftar `McpToolDescriptor { name, description,
  inputSchema, annotations? }`. Map ke `Tool` wrapper.
- **Wrapper `McpTool`:** implementasi `Tool` trait (Spec 002):
  - `name()`: **namespaced** `"{server}__{tool}"` utk hindari tabrakan dengan
    tool bawaan (`read_file`, `shell`, dll) dan antar-server. Karakter `__`
    dipilih agar valid sebagai nama tool & jelas asalnya.
  - `description()`: dari deskripsi server tool.
  - `execute(call, cancel)`: terjemahkan ke `tools/call` (tiket 04) — di tiket
    ini cukup stub yang mencatat bahwa eksekusi ditangani tiket 04, ATAU langsung
    eksekusi penuh bila 04 dikerjakan berurutan. (Lihat catatan order.)
  - Pegang `Arc<McpSession>` (interior mutability: `Tool::execute(&self)`
    immutable, jadi session harus `Arc` + `&self` method yang `async`).
- **Registrasi:** di `repl.rs` setelah tool bawaan, panggil
  `mcp_manager.register_all(&mut tool_registry)`; hanya server yang config ada
  → default none = nol tool MCP.
- **Lazy vs eager spawn:** spawn child **saat startup REPL** utk server yang
  ada di config (sesuai rekomendasi Matt), tapi setiap server dibuka di tokio
  task; bila satu server gagal handshake → laporkan per-server (jangan
  gagalkan seluruh startup), tool server itu tak didaftarkan.

## Kriteria

- [ ] Spawn child MCP (command/args/env dari config) utk tiap server; stdin/
      stdout/stdout dihubungkan; `stderr` diteruskan ke log/trace (diredaksi).
- [ ] Handshake sukses (tiket 02) sebelum `tools/list`.
- [ ] `tools/list` diparse → `McpToolDescriptor` list.
- [ ] Tiap descriptor dibungkus `McpTool` dgn nama `"{server}__{tool}"`;
      registrasi ke `ToolRegistry`.
- [ ] Server gagal init/discovery → tool server tsb tidak didaftarkan, pesan
      error per-server jelas (startup lain tetap jalan). Default kosong → nol
      spawn (zero regression).
- [ ] Duplikat nama setelah namespace → konflik dideteksi & dilaporkan (tak
      diam-diam menimpa).
- [ ] Unit test discovery terhadap payload `tools/list` canned; integration
      (opsional helper) bila helper siap. clippy bersih.

## Catatan order / scope

Discovery & eksekusi (`tools/call`) saling dekat. Agar tiap tiket punya proof
mandiri: tiket 03 membuktikan **spawn + handshake + list + registrasi** (tool
ada di registry, nama & deskripsi benar); eksekusi penuh `tools/call` + mapping
status + lifecycle shutdown dituntaskan di tiket 04. Bila implementasi lebih
enak digabung, 03 menyediakan `execute` yang men-delegate ke session tapi
tiket 04 yang menyelesaikan pemetaan error/status + tes end-to-end eksekusi.

## Dependency

02.
