# Spec 012 — TUI Dashboard (Ratatui)

Vertical slice: menambahkan **tampilan terminal visual** (Ratatui + crossterm,
di-compile ke binary — tanpa dependency runtime eksternal) yang memanfaatkan
data yang sudah ada: token meter (Spec 008), tool log (Spec 002/011), status
provider/goal/plan/reflection (Spec 009), session list (Spec 003). Hermes-RS
tetap bisa dipakai sebagai REPL readline biasa; TUI adalah **view opt-in** yang
tidak mengubah mode reaktif / non-interactive (zero regression).

## Motivasi (terverifikasi dari kode)

- Entry CLI `crates/hermes-cli/src/main.rs` → `repl::run_repl(...)`. Semua state
  hidup di `ConversationRunner` (turns, estimated_tokens, context_limit, pinned,
  goal, plan, reflection) yang dipegang `run_repl`. Streaming event dari provider
  (`EventStream`) sedang dikumpulkan jadi teks; tool calls diproses per iterasi.
- `ToolRegistry` + `state.db` (`tool_calls`) menyimpan tool log. MCP status ada
  di `repl.rs` (`mcp_handles`). Session list di `SessionStore`.
- Tidak ada render state machine saat ini; output adalah `println!`/`eprintln!`
  langsung dari loop readline. Untuk TUI kita butuh **sinkronisasi** antara
  worker agentic & renderer (channel event), bukan menulis langsung ke stdout.
- `ratatui` + `crossterm` perlu ditambah ke `crates/hermes-cli/Cargo.toml`
  (bukan hermes-core; UI murni layer CLI). Ini dependency dev/binary, tidak
  menambah surface eksekusi/network.

## Keputusan scope awal (untuk direview @matt)

- **TUI opt-in & non-invasive.** Default tetap REPL readline. TUI diaktifkan via
  flag `--tui` (atau `/tui on` saat runtime bila feasible). Keberadaan TUI tidak
  mengubah perilaku agentic loop / persisten / sanitasi.
- **Arsitektur worker + renderer.** `run_repl` tetap pemilik runner. Untuk TUI,
  agentic turn dijalankan di task worker yang mengirim `TuiEvent` (status,
  chunk token, tool call, iterasi, dsb.) ke channel; thread renderer Ratatui
  menampilkan multi-panel & mengirim perintah user balik. Ini menjaga semua
  sanitasi/redaksi tetap di lapisan yang sama (output boundary).
- **Sanitasi tetap.** Semua teks yang ditampilkan (baik dari model maupun tool)
  lewat `sanitize_untrusted_output` + redaksi kredensial, konsisten dengan REPL.
  TUI tidak membuat jalur output baru yang tak di-sanitasi.
- **Reuse data yang ada**, tidak menambah model baru: panel memakai aksesor yang
  sudah publik di `ConversationRunner` + `store` + `mcp_handles`.

## Tiket

| # | Tiket | Blocked by |
|---|---|---|
| 01 | [TUI event model + dep + CLI flag opt-in](issues/01-tui-event-model-flag.md) | — |
| 02 | [Renderer shell: Ratatui loop + multi-panel layout + crossterm events](issues/02-renderer-shell-layout.md) | 01 |
| 03 | [Status panels: token meter, provider, goal/plan/reflection, sessions](issues/03-status-panels.md) | 02 |
| 04 | [Tool log panel + streaming transcript panel + redaksi](issues/04-tool-log-transcript.md) | 02 |
| 05 | [Parity, docs, E2E closure proof](issues/05-parity-docs-closure.md) | 01–04 |

01 memodelkan event & boundary; 02 fondasi renderer/layout; 03 panel status;
04 panel tool log & transkrip; 05 penutupan.

## Invariant yang tetap berlaku

Semua invariant `docs/ROADMAP.md` tetap: `state.db` canonical; SIGINT exit 130;
credential terredaksi; turn yang dibatalkan tak dipersist parsial; tak ada fake
`User` turn. TUI **tidak** menambah surface eksekusi; hanya menampilkan ulang
state yang sudah ada. Sanitasi di render boundary tetap.
