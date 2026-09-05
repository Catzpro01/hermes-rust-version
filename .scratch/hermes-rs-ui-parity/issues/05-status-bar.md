# 013-05: Status Bar Parity
**Status:** DONE — menunggu review Matt (commit ini).

## Cakupan (Ticket 05)
- **`status_bar.rs` (baru)** — baris status bar satu-baris (§8) sebagai
  renderer pure (pola `streaming.rs` T04), semua logic di-verbatim dari
  Python `cli.py`:
  - `build_fragments(width, data)` — urutan widget & tier persis
    `_get_status_bar_fragments`: prefix ` ⚕ ` (Bar) + model (Strong,
    `#FFD700` bold), segmen dipisah ` · ` (tier <76) atau ` │ ` (tier ≥76)
    dalam Dim `#8B8682`, pad kanan 1 sel.
    - **Tier <52 (compact):** model · duration · goal · focus · yolo
      (tanpa context %).
    - **Tier 52–75 (medium):** + context `%` (tanpa bar), `🗜️ N`, `▶ N`,
      `⚙ N`, `⛓ N`, goal, duration, focus, yolo.
    - **Tier ≥76 (full):** + context detail `{used}/{total}` (dim) dan
      block bar `[████░░░░░░]` (lebar 10, `█`/`░`) + `pct` berwarna tier,
      semua badge, durasi.
  - Threshold warna gauge verbatim `_status_bar_context_style`:
    `None→Dim`, `<50 Good #8FBC8F`, `≥50 Warn #FFD700`, `>80 Bad #FF8C00`,
    `≥95 Critical #FF6B6B` (semua bold). `context_bar` memakai
    **round-half-to-even** (banker's) persis `round()` Python — test pin
    `25%→2 blok` dan `95%→10 blok` (round(9.5)=10).
  - Helper format verbatim: `format_duration_compact` (`42s`/`5m`/
    `2h 5m`/`1.5d`), `format_token_count_compact` (`1.23K`/`12K`/`1.5M`/
    `123M`, trailing-zero strip), `format_context_length` (`128K`/`1M`/
    `1.5M` — rule "bulat bila diff <0.05"), `model_short` (segment slash
    terakhir, strip `.gguf`, >26 char → 23 + `...`), `goal_segment`
    (`⊙ goal [used/max]`).
  - `trim_text` (`_trim_status_bar_text`) — trim sadar display-width
    (emoji `🗜️` = 2 sel, VS16 zero-width) + `...` di ekor; bila total
    fragment > lebar terminal, seluruh baris degradasi ke teks polos
    ter-trim (single fragment `status-bar`) — baris **tidak pernah wrap**.
  - `right_align_title` (`_right_align_status_title_fragments`) — badge
    sesi ` ─ {title}` di ujung kanan (bg gold `#FFD700`, teks navy,
    bold), konten kiri dipad dim; disuppres <24 kolom.
  - `render_line(width, depth, data)` — SGR per style class dengan bg navy
    `#1a1a2e` penuh baris (truecolor / 256-approx per `ColorDepth` T02;
    Basic = tanpa ANSI), selalu tepat `width` display cell (padded).
- **`repl.rs`** — wiring ke runtime yang sudah ada: baris status di-render
  ulang **sebelum setiap prompt** (TTY-only — piped tetap byte-stable &
  bebas-ANSI, E2E tak tersentuh): `model` = provider aktif (ikut `/provider`
  switch), `context_tokens` = `runner.estimated_tokens()`,
  `context_limit` = `ctx.limit`, `goal_active` = status goal `InProgress`,
  `duration` dari `session_start` (baru).
- **`Cargo.toml`** — `unicode-width 0.2` (sudah ada di lock via ratatui;
  untuk ukur display cell emoji).

## Keputusan & catatan transparan
- **Penempatan: di atas prompt, bukan fullscreen** — Python status bar =
  bottom-chrome `prompt_toolkit` (`wrap_lines=False`). REPL hermes-rs
  berbasis rustyline (bukan full-screen); paritas terdekat = baris penuh
  di-render ulang sebelum tiap prompt (selalu "chrome bawah" terakhir di
  atas composer). TUI ratatui (`--tui`) punya top status bar sendiri
  (Spec 012) — tidak disentuh tiket ini.
- **Data yang belum ada di hermes-rs di-hide, bukan di-templat kosong**
  (paritas dengan Python: segmen hanya muncul bila datanya ada):
  - `compressions` = 0 → `🗜️` tersembunyi. **Core belum punya counter
    kompresi** (hanya flag enabled/target di config) — struktur
    `StatusBarData` sudah siap menerima datanya; menambah counter ke
    hermes-core = perubahan core, diluar scope tiket UI ini (dicatat
    sebagai follow-up opsional).
  - `bg_tasks/bg_processes/bg_subagents` = 0 (fitur background tak ada di
    REPL hermes-rs) → badge tersembunyi.
  - `yolo` = false (fitur YOLO belum ada di hermes-rs) → `⚠ YOLO`
    tersembunyi; path render-nya di-test unit.
  - `focus_label`/`session_title`/cache-hit/latency/tps = tidak ada di
    runtime → kosong; logic-nya tetap di-render & di-test (badge judul
    sesi termasuk).
  - `goal_turns_used/max` = 0 → segment `⊙ goal` polos (persis path Python
    `max_turns == 0`); goal aktif = status `InProgress` (goal Blocked/
    Achieved tidak menempati bar — paritas "active-goal-only").
  - `context_percent` bulat ke bawah via integer division + clamp 100;
    tanpa limit → label `--` dim + bar kosong (paritas `percent=None`).
- **`render_line` selalu tepat `width` cell** — invariant anti-wrap
  (§8 `wrap_lines=False`): pad spasi (dengan bg navy) bila muat, trim +
  `...` bila tidak. Diuji untuk 3 tier + overflow.
- **fmt:** `status_bar.rs` fmt-clean (rustfmt 1.98.1). Hunk T05 di
  `repl.rs` dibuat fmt-clean (diverifikasi `rustfmt --check`: semua hunk
  tersisa di repl.rs pre-existing sejak baseline — hanya bergeser oleh
  ~29 baris yang ditambah); `main.rs`/`Cargo.toml` = 1 baris. Diskresi
  sama dengan T03/T04.

## Kriteria (checklist tiket)
- [x] Skema warna verbatim: bg navy `#1a1a2e`, teks `#C0C0C0`, prefix
      ` ⚕ ` + model `#FFD700` bold, separator ` · ` `#8B8682`
- [x] Context gauge bertingkat `#8FBC8F → #FFD700 → #FF8C00 → #FF6B6B`
      (threshold 49/50/80/81/94/95 di-pin test)
- [x] Badges: `🗜️ N`, `▶ N`/`⚙ N`/`⛓ N`, `⚠ YOLO` (render + hide-when-zero
      di-test; data runtime yang belum ada di-hide per paritas Python)
- [x] 3 tier lebar: <52 compact / <76 medium / ≥76 full (separator
      ` · ` vs ` │ `), termasuk degradasi trim anti-wrap
- [x] Terhubung runtime: token estimate, context limit, goal, provider
      aktif, durasi sesi (di-render sebelum tiap prompt, TTY-only)
- [x] Tes unit layout (22 test: tier, threshold, bar, format helper,
      trim, title badge, SGR truecolor/256/basic, anti-wrap)
- [x] `cargo test --workspace` → **400/400** (≥379 baseline T04)
- [x] clippy `--workspace --all-targets -- -D warnings` → RC 0

## Bukti
- `cargo test --workspace` → TEST_RC=0, 400 passed / 0 failed —
  log VM `/tmp/t05_final_test.log`
- `cargo clippy --workspace --all-targets -- -D warnings` → CLIPPY_RC=0 —
  log VM `/tmp/t05_clippy.log`
- Record red→green: compile merah pertama (5 error: `:.1f` trait salah,
  u16/u64 cast, trait width tak di-import) → hijau; 8 unit test merah
  (banker's rounding salah, ekspektasi `2.1M` vs `2M` salah di sisi test,
  urutan goal/durasi salah di test, `strip_ansi` per-byte pecah di char
  multibyte, `round(9.5)=10` bukan 9) → hijau 22/22.
- Ground truth: `cli.py:7678` `_get_status_bar_fragments`, `:6364`
  `_status_bar_context_style`, `:6480` `_build_context_bar`,
  `:7496` `_status_bar_goal_segment`, `:113` `format_duration_compact`,
  `usage_pricing.py:1584` `format_token_count_compact`,
  `banner.py` `_format_context_length`, style block verbatim spec §8.
