# 013-04: Streaming Response Box, Reasoning Box & Spinner
**Status:** DONE — menunggu review Matt (commit ini).

## Cakupan (Ticket 04)
- **`tui/kawaii.rs` (generated)** — konstanta spinner & kawaii **byte-exact
  via AST** dari Python canonical truth: `DOTS` (10 frame braille
  `SPINNERS["dots"]`), `KAWAII_WAITING` (10), `KAWAII_THINKING` (15),
  `THINKING_VERBS` (15) dari `agent/display.py`, plus `TICK_MS = 120`
  (divalidasi generator terhadap `time.sleep(0.12)`) dan set tag reasoning
  `REASONING_OPEN_TAGS`/`REASONING_CLOSE_TAGS` (6+6, verbatim tuple
  `_OPEN_TAGS`/`_CLOSE_TAGS` di `cli.py:8370-71`). Generator
  `gen_kawaii.py` gagal keras bila struktur tidak cocok; test ter-generate
  mem-pin seluruh nilai (termasuk face `♪(´ε` )` yang benar-benar
  mengandung backtick).
- **`streaming.rs` (baru)** — mesin display streaming, semua pure &
  testable terhadap `Vec<u8>`:
  - `ReasoningSplitter` — state-machine byte-level yang mengupas tag
    reasoning dari stream (tag set ASCII → aman byte-exact; tail prefix
    tag di-hold antar chunk sehingga tag yang terpecah antar chunk tidak
    bocor; tag terbelah di ujung stream di-flush sesuai mode).
  - `ToolCallFilter` — pengupasan markup `
    tool_call>` + 5 root saudara) dari teks model — paritas dengan
    stream consumer Python (regex IGNORECASE, block terpecah di-hold,
    orphan close tag di-strip, unterminated opener dibuang di akhir;
    root `function` hanya dengan gate `name=` + boundary seperti Python).
  - `StreamRenderer` — kotak respons (header gold bold `#FFD700` di chunk
    pertama, body `banner_text #FFF8DC` tanpa indent, footer selalu di
    baris sendiri — paritas `_cprint`), kotak reasoning `┌─ Reasoning ─┐`
    dim+italic (selalu ditutup di finish), baris tool TTY
    `  ┊ ◇ {name}` / `  ┊ ✅|❌ {name}` (dim) vs piped `  [tool] {name}`;
    baris pertama output TTY menghapus baris spinner (`\r\x1b[2K\r`);
    **setiap chunk** lewat `sanitize_untrusted_output` + `redact_credentials`
    (invariant #4; split-escape aman — sanitizer membuang escape tak
    lengkap di ekor). Mode piped tanpa ANSI sama sekali.
  - `SpinnerState` — format baris verbatim spec §7
    `  {frame} {message} ({elapsed:.1f}s)`: Thinking = `DOTS[tick%10]` +
    `THINKING_VERBS[tick/10 %15]`; Tool = `KAWAII_THINKING[tick/5 %15]` +
    `tool: {name}`. Clock di-inject (`line_at`) → test deterministik.
- **`repl.rs`** — satu task mengowni seluruh turn (future `chat_agentic` +
  channel observer + tick 120 ms + SIGINT) via `tokio::select!` → output
  stdout terurut tanpa race. Observer per-turn (`set_observer`, sender lama
  di-drop = no-op aman). Fallback provider non-streaming: teks final
  di-render lewat pipeline yang sama (`emit_final`) — tidak ada duplikasi
  bila chunk sudah mengalir (dedup via `any_text()`). SIGINT mid-stream:
  tutup kotak, hapus baris spinner, `discard_pending_user`, return
  "interrupted" → exit 130 + partial turn dibuang (invariant #2). Prompt
  konfirmasi `[y/N]` kini didahului face `KAWAII_WAITING` (rotasi) dan
  flag `confirm_active` menjeda + mengosongkan baris spinner selama
  approval (face = display-only, tidak masuk teks canonical/prompt).
- **`tui/welcome.rs`** — helper SGR baru `sgr_bold_gold`,
  `sgr_dim_italic`, `sgr_banner_text`, `SGR_RESET` (dipakai T03 frame +
  T04 streaming). Fungsi frame T03 (`response_frame*`) **dihapus** —
  `StreamRenderer` adalah satu-satunya jalur kotak sekarang; hitung-lebar
  tetap di-pin (test `renderer_width_math_matches_python` + konstanta
  `RESPONSE_LABEL_WIDTH` bersama test branding-nya).
- **`tests/streaming_e2e.rs`** — 3 E2E piped: (1) jawaban ter-stream
  masuk kotak `⚕ Hermes` tepat sekali (bukan dua — dedup final text);
  (2) aktivitas tool = baris `  [tool]` polos, markup `<tool_call…>` fake
  provider **tidak** mencapai stdout; (3) blok reasoning di-stream →
  kotak `┌─ Reasoning ─┐` polos, tag tidak bocor, urutan box sesuai
  paritas Python (kotak respons reopened setelah reasoning).

## Keputusan & catatan transparan
- **Spinner digambar inline di loop turn (bukan task terpisah)** —
  satu-satunya pemegang stdout selama turn → urutan baris terjamin,
  tanpa race lock. Tick 120 ms lewat `tokio::time::sleep_until` di dalam
  `select!` (arm berguard: hanya saat spinner aktif & bukan saat prompt
  approval). Event observer di-drain (`try_recv`) setiap iterasi sebelum
  wait — `emit` di core non-blocking (`try_send`, channel penuh = event
  display boleh hilang; turn tidak boleh deadlock).
- **`<tool_call …>` di-strip di boundary** — `AgentEvent::Chunk` (core
  mod.rs:593) memancarkan SELURUH chunk mentah termasuk markup tool-call;
  tanpa filter, markup bocor ke kotak. Python melakukan strip yang sama
  live di stream consumer (dokumentasi `_strip_think_blocks`: "Must stay
  in sync with the stream consumer"). Ini bagian invariant #4, bukan
  tambahan scope.
- **Tag reasoning = 6+6 verbatim (tanpa backtick)** — session summary
  awal salah menyebut keluarga backtick; tuple Python (cli.py:8370-71)
  hanya 6 tag sudut. Generator meng-extract persis apa yang ada; test
  backtick dihapus, pin generator mengunci set 6+6.
- **Body stream diwarnai `banner_text #FFF8DC`** — persis spec §5.1
  (`_tc` truecolor); di T03 body frame sengaja tak diwarnai karena body
  dicetak sekali, kini body = teks tidak-terpercaya yang di-scrub per
  chunk → warna di-render bersama scrub (tetap display-only).
- **Lebar kotak piped = 60** (konvensi T03; lebar scrollback Python tak
  tersedia di non-TTY). TTY = lebar terminal aktual (`terminal_width()`).
- **Lebar label reasoning = 11 cell** (` Reasoning ` = 1+9+1) — berbeda
  dengan response label 10; `r_fill = w - 2 - 11` (rumus Python §5.2).
- **Fungsi frame T03 dihapus, bukan di-`expect(dead_code)`** — jalur
  kotak kini tunggal (streaming); coverage hitung-lebar dipindahkan ke
  test streaming (zero dead code).
- **fmt:** AGENTS.md dijalankan pada file baru (streaming/kawaii/
  streaming_e2e) + `welcome.rs` (CLEAN di HEAD). `repl.rs`/`mod.rs`/
  `main.rs` **tidak** di-fmt utuh (baseline dirty — 15+ hunk pre-existing
  di repl.rs sejak HEAD); baris-baris baru T04 di-repl.rs telah dibuat
  fmt-clean manual (diverifikasi `rustfmt --check`: hunk tersisa hanya
  pre-existing). Diskresi sama dengan Ticket 03.
- **Whitespace sebelum opener tool-call dipertahankan** (Python hanya
  memakan `\s*` SESUDAH block) — paritas, test mengunci.

## Kriteria (checklist tiket)
- [x] Kotak respons §5.1: header `╭─ ⚕ Hermes ─╮` gold bold buka di chunk
      pertama, body `banner_text #FFF8DC` tanpa indent, footer `╰─…╯`
      persis di Done/finish
- [x] Kotak reasoning §5.2: 6 tag reasoning → box `┌─ Reasoning ─┐…└───┘`
      dim+italic, selalu sebelum kotak respons berikutnya, selalu ditutup
      (termasuk tag tak tertutup)
- [x] Spinner §7: dots `['⠋','⠙','⠹','⠸','⠼','⠴','⠦','⠧','⠇','⠏']`
      120 ms, message `{verb}` rotating, format `  {frame} {message}
      ({elapsed:.1f}s)`; KAWAII_WAITING/KAWAII_THINKING/THINKING_VERBS
      verbatim display.py; tool → face kawaii + `tool: {name}`
- [x] Non-TTY: `[tool] {message}` polos, output piped bebas-ANSI
      (E2E assert `!contains(\x1b)`)
- [x] Setiap chunk lewat boundary scrub (ANSI sanitizer + credential
      redaction) — unit test split-escape + redaction + E2E markup
      tool-call tak bocor
- [x] SIGINT mid-stream → exit 130, partial turn dibuang (invariant #2;
      jalur return "interrupted" yang sama dengan T03)
- [x] `cargo test --workspace` ≥ 339 → **379/379 hijau**
- [x] clippy `--workspace --all-targets -- -D warnings` → RC 0

## Bukti
- `cargo test --workspace` → TEST_RC=0, 379 passed / 0 failed
  (baseline T03 = 339; +40 test baru: 36 streaming unit + 3 kawaii pin +
  3 streaming E2E, dikurangi 2 test frame T03 yang digantikan)
  — log VM: `/tmp/t04_final_test.log`
- `cargo clippy --workspace --all-targets -- -D warnings` → CLIPPY_RC=0
  — log VM: `/tmp/t04_final_clippy.log`
- Record red→green: compile merah pertama (9 error: akses private
  module, `write!` ke `Stdout`, slice borrow) → hijau; 8 unit test merah
  (footer nempel teks / case-sensitivity tool filter / ekspektasi test
  yang salah soal whitespace & backtick) → hijau; 1 E2E merah
  (asumsi urutan box vs prefix `echo: ` fake provider) → hijau.
- Kawaii & tag: generator meng-assert struktur display.py + cli.py;
  output generator memuat face `♪(´ε` )` (backtick asli) dan 6+6 tag.
