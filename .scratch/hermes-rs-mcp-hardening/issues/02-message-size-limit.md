# 011b-02: Message size limit (SECURITY)

**Status:** done (verified on VM, 281/281 green, clippy clean).

Batas ukuran pesan JSON-RPC untuk mencegah MCP server mengirim response raksasa
(DoS). Default 10 MB; response > limit → `ToolError::Failed`/Error + warning.

- Const `MAX_MESSAGE_BYTES = 10 * 1024 * 1024`.
- Di `read_line`/parse inbound: bila line melebihi batas → error jelas.
- Test: line besar ditolak; di bawah batas diterima.
