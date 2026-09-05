# 011b-01: Default confirmation policy (SECURITY)

**Status:** done (verified on VM, 281/281 green, clippy clean).

Ubah default tool MCP agar **wajib konfirmasi** kecuali user eksplisit
`confirm: false` di config. Mencegah MCP server jahat mengeksekusi tanpa izin.

- `McpServerConfig.confirm` serde-default → `true` (secure-by-default).
- Eksplisit `confirm: false` tetap mengizinkan auto-run (opt-out jelas).
- Update doc komentar + test yang membangun config eksplisit.
