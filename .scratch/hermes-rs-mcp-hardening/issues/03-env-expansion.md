# 011b-03: Environment variable expansion

**Status:** done (verified on VM, 281/281 green, clippy clean).

Parse `${VAR_NAME}` pada nilai `env` MCP config dan ekspansi dari process
environment saat spawn. VAR tak ada → error jelas (sebut nama var, bukan nilai).
Nilai expanded TIDAK di-log.

- Helper `expand_env_value(s)` → ganti `${NAME}` (atau `$NAME`).
- Dipanggil saat `McpServer::spawn` (sebelum `envs()`).
- Test: `${HOME}` expands; `${NONEXISTENT}` error; tanpa placeholder dibiarkan.
