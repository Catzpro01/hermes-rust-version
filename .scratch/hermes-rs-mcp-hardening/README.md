# Spec 011b — MCP Hardening & REPL Commands (follow-up)

**What to build:** Menutup 4 kesenjangan yang dikoreksi dari review Spec 011
(default-confirm policy, message size limit, env expansion, `/mcp` REPL
commands). Prioritas: dua yang pertama adalah security.

**Status:** in-progress.

## Tiket

| # | Tiket | Blocker | Prioritas |
|---|---|---|---|
| 01 | [Default confirmation policy](issues/01-default-confirm.md) | — | 🔴 Security |
| 02 | [Message size limit](issues/02-message-size-limit.md) | — | 🔴 Security |
| 03 | [Environment variable expansion](issues/03-env-expansion.md) | — | 🟡 |
| 04 | [REPL /mcp commands](issues/04-repl-mcp.md) | 01-03 | 🟡 |

Dependency: Spec 011 (done). Setelah selesai: koreksi `Spec011-review-report.md`,
update ROADMAP (Spec 011b done), lanjut Spec 012.
