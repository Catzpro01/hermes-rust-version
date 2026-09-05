# 012-03: Status panels + real agentic wiring via core observer
**Status:** DONE (commit pending, review Matt).

## Ringkasan
Opsi A (Core Observer) disetujui Matt. Live data agentic kini dialirkan ke TUI
melalui observer UI-agnostik di hermes-core (AgentEvent), bukan duplikasi loop.

## Observability core (Spec 012 / Opsi A)
- `crates/hermes-core/src/conversation/events.rs` (baru): enum `AgentEvent`
  UI-agnostik — `Chunk`, `ToolStarted`, `ToolDone{name,status,result}`,
  `Iteration{current,max}`, `StatusChanged{goal_status,plan_active,reflection_on}`,
  `TokenTick`, `Done`, `Error`. hermes-core tetap bebas UI.
- `ConversationRunner`: field `observer: Option<mpsc::Sender<AgentEvent>>`
  (default None → zero overhead / zero regression), `set_observer()`, `emit()`
  = `try_send` (non-blocking, drop-on-full — Warning B). Emit di 6+ titik
  `chat_agentic`: StatusChanged+TokenTick awal, Iteration tiap loop, Chunk per
  streaming, ToolStarted sebelum exec, ToolDone sesudah exec, Done+TokenTick
  final.

## TUI worker (real agentic, no second loop)
- `tui/worker.rs`: `run_agent` membangun runtime nyata (SessionStore + session
  resume/create + ConversationRunner atas provider aktual + safe tool set
  ReadFile/ListDir/ShellReadonly/WriteFile). `chat_agentic` DIJALANKAN satu-satu
  (shared dengan REPL) dengan observer 256; forwarder task memetakan tiap
  AgentEvent → TuiEvent via `agent_event_to_tui` (sanitize + redact DI SINI,
  boundary CLI), push ke EventQueue. Turn lalu disimpan ke state.db.
- Concurrency: SessionStore tidak `Send` (rusqlite) → worker dijalankan inline
  (seperti REPL) dan `tokio::select!` vs renderer blocking; cmd_tx ditahan agar
  worker wind-down saat renderer quit. Confirmation TUI default DENY (aman;
  write/shell interaktif + input/history = tiket lanjutan).
- `tui/event.rs`: tambah varian `StatusMeta{goal_status,plan_active,reflection_on}`.
- `main.rs`: provider dibangun sekali; cabang `--tui` → `tui::run_tui(home,
  provider, provider_name, config)`. `repl::resolve_context` jadi `pub(crate)`
  agar limit konteks dipakai TUI (tanpa duplikasi logika).
- `tui/app.rs` + `layout.rs`: header 2 baris — baris1 session/provider/tokens/
  iterasi; baris2 goal/plan/reflection (StatusMeta). HEADER_HEIGHT 3→4.

## Kriteria
- [x] AgentEvent di hermes-core (8 varian, UI-agnostik).
- [x] ConversationRunner::set_observer + emit() helper (try_send non-blocking).
- [x] Emit di 6+ titik chat_agentic.
- [x] Observer default None → zero overhead/zero regression.
- [x] TUI worker map AgentEvent→TuiEvent dengan sanitasi di boundary CLI.
- [x] Unit test: observer menerima urutan event yang benar (2 core test);
      mapping+redaksi pure (4 cli test).
- [x] Status header menampilkan goal/plan/reflection/token/session (TokenTick +
      StatusMeta). Token limit dari resolve_context (ctx.limit).

## Bukti verifikasi (VM)
`cargo test --workspace` = **306/306 green** (300 + 6 baru).
`cargo clippy --workspace --all-targets -- -D warnings` bersih.
`cargo check` sukses. Repl & jalur non-TUI regresi nol (semua test hijau).
