# T03 — Info line (Spec 017)

Status: DONE — MERGED (PR #1, `07093dc`)

## Keputusan (per /ask-matt, "jalankan")

Spec §B [KOREKSI]: di Python v0.21.0 **tidak ada** info line `● {model} ·
N tools · provider: …`, dan §E: **tidak ada** prefix `✦ Tip:`. Yang ada
setelah banner hanya string branding `welcome` skin default (§9):
`Welcome to Hermes Agent! Type your message or /help for commands.`

Model line (A.2 item 1) dan summary line (A.2 item 8) sudah ada di dalam
banner (T02). T03 = **bersihkan output pasca-banner** agar parity, bukan
menambah baris baru.

## Perubahan

- `tui/welcome.rs`: konstanta `WELCOME` (verbatim §9); `summary_line(tools,
  skills, mcp_connected)` sesuai rule `' · '.join(summary_parts)` — bagian
  `{k} MCP servers` hanya bila k > 0 (referensi Python T02 dengan server
  configured-not-connected tidak memuatnya, jadi 9 referensi byte tetap
  identik).
- `repl.rs`: mode TTY → banner + `WELCOME` saja. Dihapus dari TTY:
  `✦ Tip: BROWSER_CDP_URL…` (hardcoded, non-parity), `Hermes-RS session …`,
  `Commands: …`, `[context …]`. Mode piped (non-TTY, tanpa banner) tetap
  mencetak header lama byte-stable (dipakai skrip/E2E; invariant
  ANSI-free).
- Tes: unit `summary_line_matches_python_join_rule`,
  `welcome_copy_is_verbatim_default_skin`; PTY E2E
  `banner_wide_tty_shows_logo_gold_title_bronze_border` menegaskan
  WELCOME muncul dan `✦ Tip:`/`Hermes-RS session`/`Commands:`/`provider:`
  tidak muncul di TTY.

## Catatan untuk T04

`✦ Tip:` dihapus di sini. Jika Matt memutuskan port TIPS sebagai fitur
baru, tempatnya setelah `WELCOME` dengan format yang disepakati (bukan
`✦ Tip:`).
