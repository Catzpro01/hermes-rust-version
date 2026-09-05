# Spec 014 — CLI Subcommands Parity

Slice vertikal: memetakan seluruh **subcommand CLI Hermes Python**
(`hermes model`, `hermes sessions`, …) ke Hermes-RS sebagai clap
subcommand, sehingga fitur bisa dipanggil langsung dari OS shell **tanpa
masuk REPL chat**. Sumber data sudah ada dan siap dipakai:

| Subcommand | Sumber data (sudah ada) | Tiket |
|---|---|---|
| `hermes model` | ProviderRegistry (Spec 005) + config.yaml | 02 |
| `hermes sessions` / `inspect <id>` | SessionStore (Spec 001/003) | 03 |
| `hermes messages <id>` / `tool-calls <id>` | SessionStore | 04 |
| `hermes search <query>` | `search_messages()` (Spec 004) | 05 |
| `hermes info` / `hermes mcp [list\|restart <name>]` | HermesTheme/status_bar (013) / McpServerRegistry (011) | 06 |
| `--version` / `--help` | format output parity | 07 |

## Prinsip

- **Zero regression:** tanpa subcommand → REPL persis seperti HEAD
  (default behavior).
- Flag global (`--provider`, `--api-url`, `--hermes-home`, `--tui`)
  berfungsi dengan maupun tanpa subcommand (posisi bebas).
- Output subcommand melewati sanitasi + redaksi yang sama dengan REPL
  (hanya di CLI stdout boundary; `state.db` kanonik tak tersentuh).
- Subcommand read-only tidak menulis state apa pun.
- Setiap merge: test hijau + clippy `-D warnings` bersih (zero debt).

## Tiket

| # | Tiket | Status |
|---|---|---|
| 01 | [Subcommand parser foundation](issues/01-subcommand-parser-foundation.md) | DONE (commit ini, review Matt pending) |
| 02 | [hermes model](issues/02-model-subcommand.md) | DONE (commit ini, review Matt pending) |
| 03 | `hermes sessions` + `inspect <id>` | Not started |
| 04 | `hermes messages <id>` + `tool-calls <id>` | Not started |
| 05 | `hermes search <query>` | Not started |
| 06 | `hermes info` + `hermes mcp` | Not started |
| 07 | `--version` + `--help` parity | Not started |
| 08 | Parity, docs & closure | Not started |

## Invariant yang tetap berlaku

Semua invariant `docs/ROADMAP.md`: `state.db` canonical; SIGINT exit 130;
credential terredaksi; instalasi Python Hermes **tidak disentuh** (hanya
dibaca sebagai referensi format); subcommand tidak menambah surface
eksekusi.
