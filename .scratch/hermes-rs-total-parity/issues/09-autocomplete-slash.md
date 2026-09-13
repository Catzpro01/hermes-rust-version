# T08 — Autocomplete /slash + ghost text (parity perilaku prompt_toolkit, Spec 017)

Status: DONE — MERGED (PR #3, `8d66bbb`) — verdict /ask-matt 2026-09-13:
cap 60 char, skill discovery best-effort, TUI-only placeholder — disetujui

## Scope (spec §D.2 / §J.5 — "paritas = perilaku, bukan crate")

Port perilaku `SlashCommandCompleter` + `SlashCommandAutoSuggest`
(commands.py L1632-1830) di atas rustyline yang sudah dipakai REPL:

- **Katalog**: `COMMAND_REGISTRY` 101 entri di-port **verbatim** dari
  `docs/hermes-ui-spec/017/verbatim/commands_registry.txt` (field:
  name/description/category/aliases/args_hint/subcommands/cli_only/
  gateway_only; field dispatcher gateway di-skip — tidak berperan di
  completion). Tes `catalog_matches_verbatim_file` re-parse file verbatim
  dan membandingkan semua field per entri.
  - `indicator` = satu-satunya entri ber-kwarg non-literal di source
    (`args_hint=f'...'`, `subcommands=INDICATOR_STYLES`); list style
    diambil dari katalog tips verbatim (`kaomoji|emoji|unicode|ascii`)
    dan dikecualikan dari perbandingan field di tes.
- **Completion command + alias**: prefix match case-insensitive;
  `gateway_only` (9 entri) tidak ditawarkan di CLI; alias tampil
  `alias for /name`.
- **Subcommand**: token ke-2 dari command yang punya `subcommands=`
  (mis. `/skills s…`, `/reasoning h…`).
- **Skill**: drop-in dari `<hermes-home>/skills/` — tiap subdirektori
  non-hidden = skill (nama = nama direktori); `short_desc` dari
  `description:` frontmatter `SKILL.md`. Token dinormalisasi:
  underscore ≡ hyphen (case-insensitive).
- **Stacked skills** (L1741): setelah `/skill-a `, baris yang seluruh
  tokennya skill → tawarkan skill lain (yang sudah dipakai di-exclude),
  display `⚡ {short_desc}` (cap 60 char — limit eksak upstream
  [BELUM TERVERIFIKASI]).
- **Path completion** (L1772-1806): word = path jika mulai `./`, `../`,
  `~/`, `/` atau mengandung `/` — kecuali `://` (URL). Direktori diberi
  trailing `/`; `~` diekspansi ke home user (bukan HERMES_HOME).
- **Trailing-space trick** (L1752): replacement = `{cmd} `; **KECUALI**
  `_PICKER_COMMANDS` = `model`, `personality`, `skin` (tanpa spasi agar
  Enter langsung membuka picker).
- **Ghost text** (`SlashCommandAutoSuggest`): rustyline `Hinter` — sisa
  completion unik, atau shared-prefix jika banyak kandidat (mis.
  `/reasoning h` → ghost `i` dari `high`∩`hide`; `/to` → **tanpa**
  ghost karena `/tools`, `/toolsets`, `/topup`, `/tool-calls` tidak
  punya prefix bersama). Tab menerimanya lewat menu completion.
- **Ekstensi Hermes-RS** (14 command ada di dispatch REPL tapi tidak ada
  di registry v0.21.0 — `RS_EXTENSIONS`, di-mark eksplisit):
  `tool-calls`, `inspect`, `messages`, `search`, `info`, `provider`,
  `sandbox`, `mcp`, `pinned`, `pin`, `unpin`, `reflect`, `mascot`,
  `petdex`.

## T04 opsi 3 — composer placeholder (diputuskan di sini)

`COMPOSER_PLACEHOLDERS` (11 string verbatim, tips.py L495) dipakai sebagai
**ghost text prompt kosong di TUI**: `App` menyimpan placeholder acak
(`random_composer_placeholder()`, mirip `random.choice`), di-render dim
italic (style `placeholder` TUI §8) hanya saat input kosong, dan di-roll
ulang setiap kali input dikosongkan (submit / Esc / backspace-to-empty /
history-forward). Tidak pernah masuk buffer input.

## Yang dikerjakan

- Baru: `crates/hermes-cli/src/completion.rs` (`CommandDef`,
  `COMMAND_REGISTRY`, `RS_EXTENSIONS`, `Skill` + `discover_skills`,
  `HermesCompleter` dengan `Completer`+`Hinter`+`Highlighter`+
  `Validator`+`Helper`).
- `repl.rs`: completer lama (static 63 entri, tanpa hint) dihapus;
  editor pakai `HermesCompleter::new(home)` (skills dipindai sekali saat
  start REPL). Token path relatif (`./`, `../`) di-resolve terhadap
  `cwd` yang diambil saat start (field `HermesCompleter`; tes memakai
  `with_home(hermes, home, cwd)` agar tidak menyentuh lingkungan).
- `tui/app.rs`: field `placeholder` + `App::new()` (Default tetap via
  `new()`), `roll_placeholder()` di semua jalur pengosongan,
  `render_input` menampilkan placeholder (menggantikan hint statis
  "Type a message …" — keybinding tetap didokumentasikan di
  HERMES_UI_SPEC/TUI §8).
- Tes: unit completion (verbatim cross-check 101 entri, picker
  no-space, alias, subcommand, stacked skills ± normalisasi, path ±
  tilde/URL, ghost text) + TUI (placeholder ter-render saat kosong,
  tetap anggota katalog setelah clear) + e2e TUI diperbarui.

## Keputusan Matt (verdict /ask-matt, 2026-09-13)

1. **Limit `⚡ {short_desc}` = 60 char — disetujui** (pertahankan).
2. **Skill discovery — disetujui sebagai best-effort** (subdirektori
   non-hidden `<hermes-home>/skills/` + `short_desc` dari frontmatter
   `SKILL.md`); disetel ulang saat skill loader penuh di-port.
3. **T04 opsi 3 = TUI-only — disetujui** (bukan REPL rustyline —
   di REPL Python v0.21.0 placeholder adalah dead code, parity = tidak
   ditampilkan di sana).

Catatan: fix race display-event yang sempat menyusul di branch ini
(terdeteksi via e2e flaky di CI PR) **dipindah ke PR terpisah** sesuai
verdict — lihat `.scratch/hermes-rs-ui-parity/issues/07-race-drain-turn-events.md`.

## Edge/di luar scope

- Palette Ctrl+P (registry ada, implementasi upstream tidak ditemukan —
  [BELUM TERVERIFIKASI], bukan T08).
- Shell completion `hermes completion bash|zsh|fish` (D.3) — fitur
  shell, bukan popup REPL.
- Skills dipindai sekali saat start; `/reload-skills` (belum ada di
  dispatch Rust) nanti harus me-reset helper.
