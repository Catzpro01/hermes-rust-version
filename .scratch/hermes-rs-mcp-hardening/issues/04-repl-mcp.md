# 011b-04: REPL /mcp commands

**Status:** done (verified on VM, 281/281 green, clippy clean).

- `/mcp` (atau `/mcp list`) → tampilkan server name, status, tool count.
- `/mcp restart <name>` → restart satu server + re-discover tools.
- `/mcp restart unknown` → error jelas.

Butuh menyimpan deskriptor tool per server di REPL (utk re-register) & status.
