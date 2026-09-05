# 013-03: Banner, Prompt, & Strings
**Status:** DONE (commit ini, review Matt pending).

## Cakupan final (Ticket 03)
- **`tui/art.rs` (generated)** — `HERMES_AGENT_LOGO` (6 baris, tier warna
  `bold #FFD700` ×2 / `#FFBF00` ×2 / `#CD7F32` ×2) dan `HERMES_CADUCEUS`
  (15 baris braille, gradien `#CD7F32`→`#FFBF00`→`#FFD700`→`#FFBF00`→
  `#CD7F32`→`#B8860B`) di-extract **byte-exact via AST** dari
  `~/.hermes/hermes-agent/hermes_cli/banner.py` (canonical truth). Generator
  `gen_art.py` di-Vm meng-assert struktur (jumlah baris, urutan tier) dan
  gagal keras bila tidak cocok; test yang ikut ter-generate mem-pin byte
  persis tiap baris (text + byte length + fg per baris).
- **`tui/welcome.rs`** — brand strings verbatim (§9): `PROMPT_SYMBOL "❯ "`
  (renderer menambah spasi), `RESPONSE_LABEL " ⚕ Hermes "`, `TOOL_PREFIX "┊"`,
  `SEPARATOR` (`─`×40), `WELCOME`/`WELCOME_TITLE`/`WELCOME_HINT`,
  `BANNER_TITLE "Welcome to Hermes-RS!"`, `GOODBYE "Goodbye! ⚕"`,
  `HELP_HEADER "(^_^)? Available Commands"`. Layout banner: panel ber-border
  `banner_border #CD7F32`, judul gold bold `#FFD700`, grid 2 kolom (caduceus
  di kiri ter-center, welcome copy di kanan) — dirender ke `Buffer` ratatui
  (pure, testable tanpa TTY) lalu di-flush ANSI oleh `write_buffer_ansi`
  (truecolor / 256 per `ColorDepth`, SGR state tracking, baris dipangkas di
  sel non-blank terakhir). `LOGO_MIN_WIDTH = 95` (aturan Python verbatim).
- **`repl.rs`** — banner startup **TTY-only** (piped E2E tetap byte-stable &
  bebas-ANSI); prompt `hermes> ` → `❯ ` (TTY rustyline + non-TTY); `/help`
  baru (header kawaii + separator + daftar 18 command); response frame
  `╭─ ⚕ Hermes ─…╮…╰─…╯` (spec §5.1) — gold bold di TTY, plain piped, body
  (output model) **tidak diwarnai**; goodbye `Goodbye! ⚕` hanya di clean exit
  (`/exit` + EOF) — SIGINT tetap exit-130 polos (invariant #2).
- **`render.rs`** — tool prefix `┊` (`┊ tool_call <name>`).
- **`tui/app.rs`** — composer TUI: `> ` → `❯ `.
- **`tests/branding_e2e.rs`** — 4 E2E piped (header/separator /help; goodbye;
  response frame `⚕ Hermes` + `echo: hello` di dalam frame; prompt `❯ ` +
  banner tidak bocor di piped; semua assert bebas `\x1b`).
- **`tests/smoke.rs`** — 1 asersi legacy `contains("hermes>")` diperbarui ke
  brand prompt `❯ ` (dampak langsung dari swap prompt; test lain tak tersentuh).

## Keputusan & catatan transparan
- **Adaptasi satu-satunya dari Python:** nama produk di welcome ("Hermes-RS",
  sesuai tiket: title `#FFD700 "Welcome to Hermes-RS!"`); semua simbol/label
  verbatim — divalidasi **live** dari `skin_engine._BUILTIN_SKINS["default"]`
  di VM (welcome/goodbye/response_label/prompt_symbol/help_header cocok
  persis; `tool_prefix` = `None` di dict branding, `┊` adalah literal di
  cli.py sesuai §2.3 + tiket).
- **Logo ragged dipertahankan verbatim:** baris 1-2 = 101 char, baris 3-6 =
  98 char (trailing space hilang sejak authoring asli; Python mencetak apa
  adanya → paritas byte-exact, bukan "diperbaiki").
- **`WELCOME`** (string gabungan) di-`#[cfg_attr(not(test), expect(dead_code))]`
  — dipakai test; dikonsumsi tiket TUI berikutnya (display welcome di TUI).
- **fmt:** aturan AGENTS.md `cargo fmt` dijalankan pada 3 file baru
  (art/welcome/branding_e2e = fmt-clean). Pohon baseline sudah tidak
  fmt-clean di bawah rustfmt 1.98.1 (ratusan diff di file yang tidak
  disentuh; commit sebelumnya pun tidak fmt-clean) — `cargo fmt --all` penuh
  sengaja **tidak** dijalankan agar diff tiket tetap fokus (diskresikan di sini).
- Response frame piped memakai lebar tetap 60 (lebar kotak scrollback Python
  tidak tersedia di mode non-TTY); TTY memakai lebar terminal aktual.

## Kriteria (checklist Matt)
- [x] art.rs generated dari Python source (byte-exact)
- [x] 6 logo lines, 3 color tiers (gold/accent/bronze)
- [x] 15 caduceus lines, gradient colors
- [x] Brand strings verbatim (❯, ⚕ Hermes, ┊, ─×40, welcome, goodbye, help)
- [x] render_banner() → Buffer with border, title, grid
- [x] write_buffer_ansi() truecolor + 256 fallback
- [x] print_banner() TTY-gated
- [x] response_frame() + response_frame_ansi()
- [x] /help command with Hermes branding
- [x] Goodbye on clean exit only
- [x] Prompt ❯ in REPL + TUI
- [x] tool_prefix ┊ in render.rs
- [x] 4 E2E tests (help, goodbye, response frame, prompt symbol)
- [x] Unit tests (strings, logo, caduceus, banner buffer, ANSI, frame math, narrow-width wrap)
- [x] 323+ tests green, clippy clean
- [x] No ANSI in piped stdout (E2E invariant)

## Follow-up review (siklus 1 — wajib, diterapkan sebelum closure)
- 🟡 **Clipping hint di lebar minimum (60 kolom):** tanpa wrap, baris
   (41 char) terpotong di
   karena kolom kanan hanya ±24 sel. Dibuktikan **red** oleh
   (dump buffer menunjukkan
  pemotongan), lalu **fixed** dengan  pada
  paragraph welcome copy → **green** (hint wrap per-kata:  / ).

## Bukti verifikasi (VM master)
`cargo test --workspace` = **339/339 green** (323 baseline + 16 baru:
art 4, welcome 8, branding_e2e 4).
`cargo clippy --workspace --all-targets -- -D warnings` = bersih (RC=0).
`rustfmt --check` bersih pada art.rs, welcome.rs, branding_e2e.rs.
Instalasi Python Hermes tak tersentuh (test `smoke_python_hermes_untouched` green).
