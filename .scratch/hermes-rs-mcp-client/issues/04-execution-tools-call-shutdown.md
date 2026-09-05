# 04: Eksekusi `tools/call` + mapping status + cancell/timeout + graceful shutdown

**What to build:** Jalur eksekusi penuh MCP tool (`tools/call`) lewat
`McpTool::execute`, dengan pemetaan hasil/error ke tipe Hermes
(`ToolResponse`/`ToolError`/`ToolExecutionStatus`), hormati `CancellationToken`,
batasi timeout per panggilan, dan matikan child process secara graceful
(`kill_on_drop`) saat REPL/runner selesai.

**Blocked by:** 03

## Desain

- **`McpTool::execute`** (melengkapi stub tiket 03):
  1. `call.arguments` (String) di-parse sebagai objek JSON. Bila bukan objek →
     `ToolError::Failed("mcp arguments must be a JSON object")`.
  2. Kirim request `tools/call` params `{ name, arguments }` via `McpClient`
     (tiket 02). `cancel` diteruskan: bila dibatalkan saat menunggu respons →
     batal (tidak membiarkan response menggantung), map `ToolExecutionStatus::Cancelled`.
  3. Timeout per panggilan (konstanta, mis. default 60 s; selaras pola timeout
     Spec 006) → `ToolError::Timeout`.
  4. Hasil `tools/call`:
     - `result.content` (array konten MCP: text/image/resource) → di-flatten
       ke string (text) utk `ToolResponse.content`. `isError: true` pada result
       → `ToolError::Failed(...)` (retryable? — keputusan: server `isError`
       adalah kegagalan eksekusi tool, dipetakan ke status `Error` sehingga
       recovery Spec 009 (Ticket 04) memperlakukannya retryable).
     - JSON-RPC `error` (mis. method not found, tool tak ada) → `ToolError::Failed`.
     - `structuredContent` bila ada & tak ada text → representasikan JSON.
  5. `ToolResponse { id, name, content, success: true }` bila sukses.
- **`Denied`/konfirmasi:** `confirm: true` (config tiket 01) → tool server tsb
  lewat gate konfirmasi Spec 002 (`Confirmation`), bila user menolak →
  `ToolError::Denied` (tak di-bypass). Default `confirm: false` → jalankan.
- **Lifecycle / shutdown:** `McpManager` + tiap session mengimplement
  `Drop`/`shutdown()`: kirim `notifications/cancelled`/perlu? minimal kirim
  request `shutdown` (bila didukung) lalu tutup stdin & `kill_on_drop(true)` pd
  `Command` sehingga child ikut mati bila Hermes keluar. Jangan tinggalkan
  zombie child saat SIGINT/exit REPL.

## Kriteria

- [ ] `tools/call` round-trip sukses: hasil konten text di-flatten ke
      `ToolResponse.content`; `success: true` + id/name benar.
- [ ] `call.arguments` bukan objek JSON → error jelas (Failed), tidak eksekusi.
- [ ] `isError: true` / JSON-RPC error → dipetakan (Failed/Error); `Denied`
      via `confirm` gate → `ToolError::Denied` (tak retry).
- [ ] Cancellation: `cancel` sebelum/saat menunggu → batal bersih, tak ada
      response menggantung; status `Cancelled`.
- [ ] Timeout per-panggilan → `ToolError::Timeout`.
- [ ] Graceful shutdown: tutup stdin → (optional shutdown request) → child
      di-kill pada drop; tidak ada child tersisa saat exit (test cek PID mati).
- [ ] Registrasi & eksekusi berjalan lewat agentic loop (unit + integration
      dengan helper server); clippy bersih.
- [ ] `ToolCallRecord` (state.db, bila store_ctx) merekam tool MCP dgn status
      benar — sama seperti tool bawaan (Spec 002/009 path).

## Catatan keamanan

Eksekusi tool MCP = permintaan ke child yang user-config-kan; Hermes sendiri
tidak menjalankan perintah dari tool tsb di shell-nya, tapi server bisa
melakukan apa pun sesuai kapabilitasnya. Karena itu `confirm: true` tersedia,
dan dokumentasi SECURITY menekankan bahwa menambahkan server MCP = menambah
kapabilitas eksekusi terpercaya (setara menambah tool). `Denied` tetap tak
di-bypass; timeout mencegah child hang menggantung agent selamanya.

## Dependency

03 (dan 01 utk `confirm`, 02 utk transport/request).
