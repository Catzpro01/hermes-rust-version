# 02: JSON-RPC 2.0 stdio transport + handshake `initialize`

**What to build:** Inti transport protokol MCP — klien JSON-RPC 2.0 yang
berbicara **newline-delimited JSON (NDJSON) over stdio** ke child process, plus
handshake `initialize` → `notifications/initialized`. Belum discovery/eksekusi
tool; fokus korek API transport + protokol yang deterministik & teruji.

**Blocked by:** 01

## Desain

- Modul baru `crates/hermes-core/src/mcp/` dengan:
  - `client.rs` — `McpClient` (protocol logic, provider-neutral).
  - `transport.rs` — trait `McpTransport` (tulis/baca frame NDJSON) +
    dua impl: `StdioTransport` (tokio `Command` child, stdin/stdout) dan
    transport in-memory/pipes utk unit test.
  - `jsonrpc.rs` — struct tipe JSON-RPC 2.0 (`Request`, `Response`, `Notification`,
    `Error`) dgn `serde`, plus helper id generator.
  - `error.rs` — `McpError` (parse, timeout, transport closed, protocol error,
    id mismatch).

- **Transport seam:** `McpTransport { fn send(&self, line) ; fn next_line() }`
  dibungkus agar unit test memberi payload canned tanpa spawn `npx`.

- **Handshake:**
  1. Kirim `initialize` request (params: `protocolVersion`, `capabilities`,
     `clientInfo`).
  2. Terima `initialize` **response** (protocolVersion server, capabilities,
     serverInfo).
  3. Kirim `notifications/initialized` (notification, tanpa id).
  4. `McpClient` siap → metode `request(method, params)` & `notify(method,
     params)` publik.

- Protocol version MCP: pin `"2024-11-05"` (latest stable saat ini) dgn
  konstanta; kalau server balas versi lebih baru tetap lanjut (server
  menentukan).

## Kriteria

- [ ] JSON-RPC 2.0 encode/decode benar (id bisa number/string; `result` /
      `error` mutually exclusive; `params`/`result` bisa objek).
- [ ] NDJSON framing: tiap pesan satu baris JSON (no `Content-Length`); baca
      per-baris dari child stdout.
- [ ] Handshake sukses terhadap payload canned (unit test via transport
      in-memory): kirim `initialize` dgn benar, parse response, kirim
      `notifications/initialized`.
- [ ] `McpError` utk: response id tak cocok dengan request yang outstanding,
      `error` code JSON-RPC, transport EOF/closed sebelum respons, timeout.
- [ ] Request concurrent: id unik per request; respons dicocokkan ke id (mapan
      request-id → oneshot).
- [ ] Tidak melakukan discovery/eksekusi tool di tiket ini.
- [ ] Unit tests (in-memory transport) + clippy bersih.

## Catatan keamanan

Transport hanya membuka **stdio ke child yang user-config-kan** (tidak ada
socket/network dari Hermes). Nilai yang ditulis/dibaca adalah JSON-RPC tool
payload; tidak ada secret Hermes yang dikirim ke child kecuali `env` yang
sengaja user-set di config (tiket 01).

## Dependency

01 (model config siap, walau tiket ini bisa memakai tipe JSON-RPC murni tanpa
config — blokir tetap utk menjaga urutan dok).
