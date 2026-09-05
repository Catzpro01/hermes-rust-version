# HERMES UI SPEC — Arkeologi UI Python Hermes untuk Hermes‑RS

**Spec:** Spec 013 — Hermes Python UI Parity
**Ticket:** 01 (Visual Archaeology)
**Sumber:** `~/.hermes/hermes-agent` di VM (`fern@103.171.85.230`), hasil ekskavasi langsung dari source (bukan tebakan).
**Status dokumen:** Draf untuk review Matt. **Belum ada kode Spec 013 yang ditulis.**

> Prinsip Matt: *"Don't guess the UX. Excavate it."* Setiap warna/heksa/banner/prompt/pemisah di bawah ini diambil **verbatim** dari file Python Hermes asli, kecuali bagian yang secara eksplisit ditandai **[BELUM TERVERIFIKASI / perlu konfirmasi]**.

---

## 1. Gambaran Arsitektur UI

Hermes Python adalah **dua permukaan render yang berjalan bersamaan** di dalam satu proses:

1. **`prompt_toolkit` TUI (non‑fullscreen, "fixed bottom chrome").** `Application(full_screen=False)`. Yang ditampilkan tetap adalah bagian bawah terminal: baris spinner/status, *status bar*, *input rule* atas‑bawah (`─`), *image bar*, dan *input area* (`TextArea`). Scrollback di atasnya dipakai untuk transkrip percakapan.
2. **`rich` untuk rendering transkrip/welcome di atas input.** `rich.Console`, `rich.panel.Panel`, `rich.table.Table` dipakai untuk *welcome banner* startup, dan render akhir / background di dalam `rich.panel` (box `HORIZONTALS`). Streaming kata‑per‑kata berjalan lewat `prompt_toolkit.print_formatted_text` / `ANSI` (`_cprint`), bukan lewat rich `Live`.

**Versi dependensi yang relevan** (dari `cli.py` import langsung / pyproject): `rich==14.3.3`, `prompt_toolkit==3.0.52`. Ada catatan kode `#40490` bahwa rich + prompt_toolkit pernah deadlock sehingga pemakaian rich dibatasi (path "Matrix delivery" memakai `rich=false`). Path CLI produktif memakai prompt_toolkit sebagai jalur output utama.

**Konstanta warna streaming inti** (`cli.py`):

```
_ACCENT_ANSI_DEFAULT = "\033[1;38;2;255;215;0m"   # #FFD700 bold (fallback)
_BOLD = "\033[1m"
_RST  = "\033[0m"
_DIM  = "\x1b[2;3m"                                # dim + italic
_STREAM_PAD = ""                                    # tanpa indent kiri
_STREAM_PARTIAL_PREVIEW_LEN = 60
_ACCENT = _SkinAwareAnsi("response_border", "#FFD700", bold=True)
```

`_SkinAwareAnsi` mengecek skin aktif lalu mengembalikan escape true‑color `38;2;R;G;B` dengan opsi bold. Jadi **aksen frame respon = `response_border` = `#FFD700` bold**.

---

## 2. Palet Warna (Skin "default" — sumber utama)

Didefinisikan di `hermes_cli/skin_engine.py`, blok `_BUILTIN_SKINS["default"]["colors"]`. Deskripsi skin: *"Classic Hermes — gold and kawaii"*. Ini adalah palet yang harus **persis** direplikasi.

### 2.1 Kolom warna kanonik (dark mode)

| Kunci | Hex | Peran UI |
|---|---|---|
| `banner_border` | `#CD7F32` | Border panel welcome (perunggu/bronze) |
| `banner_title` | `#FFD700` | Judul panel + label versi (bold) |
| `banner_accent` | `#FFBF00` | Header bagian (Available Tools, MCP, Skills) |
| `banner_dim` | `#B8860B` | Teks redup: separator `·`, label, kategori, cwd |
| `banner_text` | `#FFF8DC` | Teks isi panel (nama tool, nama skill) |
| `ui_accent` | `#FFBF00` | Aksen umum |
| `ui_label` | `#DAA520` | Label umum |
| `ui_ok` | `#4caf50` | Status sukses |
| `ui_error` | `#ef5350` | Kesalahan |
| `ui_warn` | `#ffa726` | Peringatan |
| `prompt` | `#FFF8DC` | Warna prompt |
| `input_rule` | `#CD7F32` | Garis `─` atas/bawah area input |
| `response_border` | `#FFD700` | Frame kotak respons streaming + Panel jawaban |
| `status_bar_bg` | `#1a1a2e` | Latar status bar (navy gelap) |
| `status_bar_text` | `#C0C0C0` | Teks status bar default |
| `status_bar_strong` | `#FFD700` | Model / segmen kuat (bold) |
| `status_bar_dim` | `#8A7A4A` | Separator `·` di status bar |
| `status_bar_good` | `#8FBC8F` | Konteks baik (bold) |
| `status_bar_warn` | `#FFD700` | Peringatan (bold) |
| `status_bar_bad` | `#FF8C00` | Buruk / mendekati batas (bold) |
| `status_bar_critical` | `#FF6B6B` | Kritis (bold) |
| `session_label` | `#DAA520` | Label sesi |
| `session_border` | `#8B8682` | Sesi redup |
| `completion_menu_bg` | `#1a1a2e` | Menu autocomplete latar |
| `completion_menu_current_bg` | `#333355` | Item autocomplete aktif |
| `selection_bg` | `#3a3a55` | Seleksi |
| `shell_dollar` | `#4dabf7` | Prompt shell/dollar |
| `voice_status_bg` | `#1a1a2e` | Latar bar status voice |

Kunci tambahan yang disebut di komentar skin (`ui_thinking` = `#CC9B1F`, `syntax_comment` = `#CC9B1F`) dengan catatan *"falls back to banner_dim"*. **[BELUM TERVERIFIKASI]** apakah kedua kunci ini benar‑benar dipakai di path streaming CLI atau hanya di TUI/editor; lihat §5 (reasoning di streaming klasik ternyata pakai `_DIM`, bukan `#CC9B1F`).

### 2.2 Skin lain (referensi, bukan target utama)
- `ares`: crimson/bronze (simbol prompt `⚔`, response_label `" ⚔ Ares "`).
- `nord`, `dracula`, `github_dark`, dll. (periksa `skin_engine.py`).
- `light_colors`: overlay untuk light mode — menurunkan kecerahan emas agar terbaca di latar terang (mis. `banner_title #C8961E`). Detail tabel `light_colors` juga sudah diekskavasi; dipakai hanya bila terminal terdeteksi terang (`_detect_light_mode`, query OSC‑11 `_query_osc11_background`).

### 2.3 Peta warna per peran dalam alur obrolan

| Elemen | Warna | Sumber |
|---|---|---|
| Prompt simbol `❯` (komposer) | gaya kosong (warisi terminal) | `'prompt': ''` di style TUI |
| Placeholder/hint | `#888888 italic` | style TUI |
| Pesan user (bullet) | `●` **bold** + aksen (`_accent_hex()` = aksen aktif) | `_print_user_message_preview` |
| Frame kotak respons (`╭─╮`) | **bold** `response_border #FFD700` | `_ACCENT` |
| Isi teks respons streaming | true‑color `banner_text #FFF8DC` | `_stream_text_ansi` |
| Kotak reasoning | `_DIM` = dim+italic (warna default terminal) | `_close_reasoning_box` |
| Separator `─` (misc output) | aksen (`_accent_hex()`) | `_print_user_message_preview`, dll. |
| Baris tool | prefix `┊` (dan `_DIM`/`_BOLD` untuk header) | lihat §6 |

---

## 3. Banner Startup (Welcome) — struktur & copy verbatim

Dibangun di `hermes_cli/banner.py` → `build_welcome_banner(console, model, cwd, tools, ...)`.

Urutan cetak:
1. baris kosong (`console.print()`)
2. Jika `terminal_width >= 95`: cetak **`HERMES_AGENT_LOGO`** (ASCII HERMES‑AGENT, 2×8 blok + versi label), lalu baris kosong.
3. Cetak **outer `rich.panel.Panel`** dengan `layout_table = rich.table.Table.grid(padding=(0,2))`, dua kolom: kiri (justify center) = `HERMES_CADUCEUS`; kanan (justify left) = info.

Atribut panel:
- `title` = versi, markup `[bold #FFD700]...[/]` (banner_title). Bila ada git tag release: `[bold #FFD700][link=<url>]Hermes Agent vX ... [/link][/]`.
- `border_style` = `banner_border` `#CD7F32`.
- `padding=(0, 2)`.
- Kolom dipisah dengan `padding=(0,2)` pada grid.

Konten kolom kanan:
- Header `[bold accent]Available Tools[/]` (accent `#FFBF00`).
- Baris per toolset: `[dim]toolset:[/] tool1, tool2, ...` (nama tool `banner_text #FFF8DC`; tool disabled → `[red]`, lazy → `[yellow]`, kelebihan panjang → `[dim]...[/]`).
- `MCP Servers` (jika ada): status `connected` hijau/teks, `connecting` → `[yellow]— connecting[/]`, `failed` → `[red]— failed[/]`, dsb.
- `[bold accent]Available Skills[/]`: baris per kategori `[dim]category:[/] nama1, nama2, ...`.
- Baris ringkasan redup: `[dim]· `N tools · M skills · /help for commands` ·[/]`.
- Label `[bold accent]Runtime:[/]`, `[bold accent]Profile:[/]`, baris `Session:` `#8B8682`.
- CWD baris redup.
- Ada juga snapshot banner (`banner_snapshot_fingerprint`) utk mempercepat startup — tidak memengaruhi tampilan.

**Blok hero / logo (verbatim, karakter persis):**

`HERMES_AGENT_LOGO` (line 4863‑4868 cli.py / banner.py 70‑76):
```
[bold #FFD700]██╗  ██╗███████╗██████╗ ███╗   ███╗███████╗███████╗       █████╗  ██████╗ ███████╗███╗   ██╗████████╗[/]
[bold #FFD700]██║  ██║██╔════╝██╔══██╗████╗ ████║██╔════╝██╔════╝      ██╔══██╗██╔════╝ ██╔════╝████╗  ██║╚══██╔══╝[/]
[#FFBF00]███████║█████╗  ██████╔╝██╔████╔██║█████╗  ███████╗█████╗███████║██║  ███╗█████╗  ██╔██╗ ██║   ██║[/]
[#FFBF00]██╔══██║██╔══╝  ██╔══██╗██║╚██╔╝██║██╔══╝  ╚════██║╚════╝██╔══██║██║   ██║██╔══╝  ██║╚██╗██║   ██║[/]
[#CD7F32]██║  ██║███████╗██║  ██║██║ ╚═╝ ██║███████╗███████║      ██║  ██║╚██████╔╝███████╗██║ ╚████║   ██║[/]
[#CD7F32]╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝╚══════╝      ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═══╝   ╚═╝[/]
```
Warna tiap baris logo: baris 1‑2 `bold #FFD700`, baris 3‑4 `#FFBF00`, baris 5‑6 `#CD7F32`. Ini **figlet HERMES‑AGENT (two‑line)**.

`HERMES_CADUCEUS` (hero di kolom kiri banner) — seni titik/braille bergradasi `#CD7F32`/`#FFBF00`/`#FFD700`/`#B8860B` (~16 baris). Teks penuh diambil verbatim dari `banner.py` baris ~78‑96 (terlampir di file evidence; karakter braille persis di `arch` source).

---

## 4. Label versi & format

`format_banner_version_label()`: `"Hermes Agent v{VERSION} ({RELEASE_DATE})"`, dengan tambahan bila repo git:
- sinkron: `" · upstream {short}"`
- ada commit lokal: `" · upstream {short} · local {short} (+{n} carried commit(s))"`

VERSI YANG TERUKUR (hasil `--version` VM):
```
Hermes Agent v0.21.0 (2026.8.31) · upstream 63279301
Install directory: /home/fern/.hermes/hermes-agent
Install method: git
Python: 3.11.16
OpenAI SDK: 2.24.0
Update available: 4303 commits behind — run 'hermes update'
```

`--version` memakai string `Hermes Agent v…`, **bukan** `hermes v…`. (Verbatim bagian penting.)

---

## 5. Streaming respons (transkrip) — spesifikasi kotak

Semua lewat `_emit_stream_text` / `_flush_stream` / `_stream_delta`. Ini alur "token streaming" yang harus disamakan dengan render Hermes‑RS.

### 5.1 Kotak respons (assistant)
- Saat teks pertama terlihat: cetak header frame:
  ```
  _cprint(f"\n{_ACCENT}╭─{label}{'─' * max(fill - 1, 0)}╮{_RST}")
  ```
  - `label` = branding `response_label` = `" ⚕ Hermes "` (spasi di kedua sisi). Bila `show_timestamps`: label += ` HH:MM`.
  - `fill = w - 2 - width(label)`; `w = self._scrollback_box_width()` (lebar kotak scrollback).
  - Jadi visual: **`╭─ ⚕ Hermes ───────────╮`** (ujung‑ujung rounded, diisi `─`, aksen bold `#FFD700`).
- Isi: tiap baris lengkap dicetak `_cprint(f"{_STREAM_PAD}{_tc}{line}{_RST}")`; `_tc` = escape true‑color `banner_text #FFF8DC` (`\033[38;2;255;248;220m`). Tidak ada indent kiri.
- Di akhir (`_flush_stream`): cetak footer **`╰───╯`** (aksen bold): `_cprint(f"{_ACCENT}╰{'─' * (w - 2)}╯{_RST}")`.
- **Tidak ada border kiri/kanan** selama streaming — hanya bingkai atas+bawah.
- Markdown di‑`strip` bila `final_response_markdown == "strip"`. Tabel diakumulasi lalu di‑realign per blok.
- TTFT: bila paragraf panjang tanpa newline melewati `>=80` char, ekor baris dimirror ke baris spinner status dengan prefiks `… ` (contoh: `self._spinner_text = f"… {preview}"`).

### 5.2 Kotak reasoning (opsional, `show_reasoning`)
- Dibuka pada token reasoning pertama (tag `<REASONING_SCRATCHPAD>`/`<think>`/dll.).
- Header: `_cprint(f"\n{_DIM}┌─{r_label}{'─' * max(r_fill - 1, 0)}┐{_RST}")`, `r_label = " Reasoning "`.
- Isi dicetak `_DIM` (dim+italic). Footer: `_DIM└───┘`.
- Reasoning dirender **redup**, dan selalu muncul **sebelum** kotak respons.

### 5.3 Pesan user di scrollback
```
ChatConsole().print(f"[{_accent_hex()}]{'─' * 40}[/]")     # pemisah aksen
# satu baris:
ChatConsole().print(f"[bold {_accent_hex()}]●[/] [bold]{_escape(text)}[/]")
# multi-baris: via _format_submitted_user_message_preview (format tabel/list)
```
Visual: pemisah `─` × 40 berwarna aksen, lalu **bullet `●` bold‑aksen** + teks **bold**.

> `_accent_hex()` = warna aksen aktif via skin (default `banner_accent #FFBF00` bila tidak ada skin/light‑mode; lihat `_accent_hex` dan `_SkinAwareAnsi`). **[BELUM TERVERIFIKASI nilai pasti `_accent_hex()` di dark canonical]** — lihat catatan 2.1 tentang fallback. Pada skin default, aksen = `#FFBF00`.

---

## 6. Tool call / aktivitas

- Prefix baris tool: `┊` (branding `tool_prefix` default). Contoh verbatim dari kode:
  - `_cprint(f"  ┊ {emoji} preparing {tool_name}…")`
  - `_cprint(f"  {_DIM}┊ ◇ {header}{_RST}")` — header tool/turn redup, marker `┊ ◇`.
  - `[tool]` dipakai di KawaiiSpinner saat non‑TTY: `_write(f"  [tool] {message}")`.
- Emoji indikator yang terlihat di kode: `✅`/`❌` (hasil), `⚠` (peringatan/YOLO), `▶` `⚙` `⛓` (tugas/proses/sub‑agent di status bar), `💭` (thinking), `📎` (image), `🗜️` (kompresi), `⚕` (ikon Hermes), `⚔`/`⚥` (skin lain).
- Peringatan di rich: `[bold red]...[/]` pada path tertentu (mis. unknown toolsets), `[bold red]{message}[/]` (error umum di `_console_print`).

> Katalog lengkap emoji per jenis pesan tool belum 100% dijamin komplet; ditandai **[BELUM TERVERIFIKASI]** bila dipakai sebagai spesifikasi byte‑exact.

---

## 7. Spinner & animasi

`agent/display.py` → `class KawaiiSpinner`.

```python
SPINNERS = {
  'dots':    ['⠋','⠙','⠹','⠸','⠼','⠴','⠦','⠧','⠇','⠏'],
  'bounce':  ['⠁','⠂','⠄','⡀','⢀','⠠','⠐','⠈'],
  'grow':    ['▁','▂','▃','▄','▅','▆','▇','█','▇','▆','▅','▄','▃','▂'],
  'arrows':  ['←','↖','↑','↗','→','↘','↓','↙'],
  'star':    ['✶','✷','✸','✹','✺','✹','✸','✷'],
  'moon':    ['🌑','🌒','🌓','🌔','🌕','🌖','🌗','🌘'],
  'pulse':   ['◜','◠','◝','◞','◡','◟'],
  'brain':   ['🧠','💭','💡','✨','💫','🌟','💡','💭'],
  'sparkle': ['⁺','˚','*','✧','✦','✧','*','˚'],
}
```
- Default `spinner_type='dots'` (braille). Tick `time.sleep(0.12)` ≈ **120 ms/frame**.
- Format baris: `\r  {frame} {message} ({elapsed:.1f}s)` (indent 2 spasi), dengan `wings` dari skin bila ada: `  {left} {frame} {message} {right} ({elapsed:.1f}s)`.
- **Kawaii faces** saat "menunggu/mikir" (dipakai tool_executor sbg ganti frame saat menunggu persetujuan dsb.):
  - `KAWAII_WAITING`: `(｡◕‿◕｡)`, `(◕‿◕✿)`, `٩(◕‿◕｡)۶`, `(✿◠‿◠)`, `( ˘▽˘)っ`, `♪(´ε` )`, `(◕ᴗ◕✿)`, `ヾ(＾∇＾)`, `(≧◡≦)`, `(★ω★)`.
  - `KAWAII_THINKING`: `(｡•́︿•̀｡)`, `(◔_◔)`, `(¬‿¬)`, `( •_•)>⌐■-■`, `(⌐■_■)`, `(´･_･`)`, `◉_◉`, `(°ロ°)`, `( ˘⌣˘)♡`, `ヽ(>∀<☆)☆`, `٩(๑❛ᴗ❛๑)۶`, `(⊙_⊙)`, `(¬_¬)`, `( ͡° ͜ʖ ͡°)`, `ಠ_ಠ`.
  - `THINKING_VERBS`: pondering, contemplating, musing, cogitating, ruminating, deliberating, mulling, reflecting, processing, reasoning, analyzing, computing, synthesizing, formulating, brainstorming.
- Di dalam TUI (StdoutProxy aktif), animasi `\r` **tidak** dijalankan — state spinner dirender sebagai **widget `_spinner_text`** di atas status bar (teks "sedang berpikir"), token flow aktif. Non‑TTY: spinner di‑skip, cukup log `[tool] {message}`.
- Konfigurasi display yang relevan: `display.bell_on_prompt` (default False), `display.spinner_token_flow` (default True), `display.cli_refresh_interval` (default 0).

---

## 8. Layout TUI bottom‑chrome (prompt_toolkit) — urutan widget

Root `Layout(HSplit([...]))` — urutan anak (dari `_build_tui_layout_children`):
```
Window(height=0)
sudo_widget            # panel password (Modal → muncul saat dibutuhkan)
secret_widget          # capture secret
approval_widget        # persetujuan perintah berbahaya
slash_confirm_widget   # konfirmasi perintah
clarify_widget         # pertanyaan klarifikasi pilihan
model_picker_widget    # pemilih model
command_palette_widget # palet perintah
spinner_widget         # baris status "sedang berpikir" (FormattedTextControl)
spacer                 # hint text
_pet_widget            # "pet" (opsional, skin)
_stash_panel_widget    # panel draft tersimpan (Ctrl+S)
status_bar             # ConditionalContainer, tinggi 1, wrap_lines=False
input_rule_top         # Window(char='─', style class:input-rule)  → #CD7F32
image_bar              # badge 📎 (ConditionalContainer)
input_area             # TextArea (komposer utama)
input_rule_bot         # Window(char='─', style class:input-rule)
voice_status_bar       # hanya saat voice mode
completions_menu       # CompletionsMenu(max_height=12)
```

Komponen kunci:
- **input_area**: `prompt_toolkit.widgets.TextArea`, height dinamis, prompt disuntik via **BeforeInput**; simbol prompt = branding `prompt_symbol` `❯ ` (ditambah `" "`), dan bila profile aktif bukan default → `"<profile> ❯ "`. Di style, `'prompt': ''` (warisi terminal) — teks ketikan **tanpa warna hardcoded** agar terbaca di light & dark.
- **input_rule_top/bot**: baris `─` berulang sepanjang lebar, warna `#CD7F32`, tinggi mengikuti `_tui_input_rule_height("top"/"bottom")`.
- **status_bar**: satu baris terakhir, `wrap_lines=False`. Konten (fragments, `_get_status_bar_fragments`) diawali `" ⚕ "` + `model_short` (strong), lalu segmen dipisah `" · "` (dim):
  - context `%` (warna per tingkat), cache‑hit `%`, kompresi `🗜️ N`, bg tasks `▶ N`, bg processes `⚙ N`, bg subagents `⛓ N`, goal, focus, YOLO `⚠ YOLO` (status‑bar‑yolo), durasi, baterai.
  - Lebar < 52 → versi ringkas; < 76 → sedang; ≥ 76 → penuh.
- **spinner_widget**: menampilkan `_spinner_text` (thinking + token‑flow preview `… `) di atas status bar; tinggi dari `_spinner_widget_height`.

### Style TUI (`self._tui_style_base`, verbatim)
```
'input-area': '' , 'placeholder': '#888888 italic', 'prompt': '',
'prompt-working': '#888888 italic', 'hint': '#888888 italic',
'status-bar': 'bg:#1a1a2e #C0C0C0',
'status-bar-strong': 'bg:#1a1a2e #FFD700 bold',
'status-bar-dim': 'bg:#1a1a2e #8B8682',
'status-bar-good': 'bg:#1a1a2e #8FBC8F bold',
'status-bar-warn': 'bg:#1a1a2e #FFD700 bold',
'status-bar-bad': 'bg:#1a1a2e #FF8C00 bold',
'status-bar-critical': 'bg:#1a1a2e #FF6B6B bold',
'status-bar-yolo': 'bg:#1a1a2e #FF4444 bold',
'status-bar-session-title': 'bg:#FFD700 #1a1a2e bold',
'input-rule': '#CD7F32',
'image-badge': '#87CEEB bold',
'completion-menu': 'bg:#1a1a2e #FFF8DC',
'completion-menu.completion': 'bg:#1a1a2e #FFF8DC',
'completion-menu.completion.current': 'bg:#333355 #FFD700',
'completion-menu.meta.completion': 'bg:#1a1a2e #888888',
'completion-menu.meta.completion.current': 'bg:#333355 #FFBF00',
'clarify-border': '#CD7F32', 'clarify-title': '#FFD700 bold',
'clarify-question': '#FFF8DC bold', 'clarify-choice': '#AAAAAA',
'clarify-selected': '#FFD700 bold', 'clarify-active-other': '#FFD700 italic',
'clarify-answer': '#98FB98', 'clarify-countdown': '#CD7F32',
'sudo-prompt': '#FF6B6B bold', 'sudo-border': '#CD7F32',
'sudo-title': '#FF6B6B bold', 'sudo-text': '#FFF8DC',
'approval-border': '#CD7F32', 'approval-title': '#FF8C00 bold',
'approval-desc': '#FFF8DC bold', 'approval-cmd': '#AAAAAA italic',
'approval-choice': '#AAAAAA', 'approval-selected': '#FFD700 bold',
'voice-prompt': '#87CEEB', 'voice-recording': '#FF4444 bold',
'voice-processing': '#FFA500 italic', 'voice-status': 'bg:#1a1a2e #87CEEB',
'voice-status-recording': 'bg:#1a1a2e #FF4444 bold',
```

> Catatan: `Application(full_screen=False)` — TUI Python **tidak** fullscreen; ia menambatkan chrome bawah di atas scrollback. Ini berkontras dengan TUI Hermes‑RS (ratatui). Implikasi untuk paritas didiskusikan di §11.

---

## 9. Branding & copywriting (default skin)

Dari `_BUILTIN_SKINS["default"]["branding"]`:
```
agent_name:      "Hermes Agent"
welcome:         "Welcome to Hermes Agent! Type your message or /help for commands."
goodbye:         "Goodbye! ⚕"
response_label:  " ⚕ Hermes "           # (dengan spasi; dipakai di header kotak)
prompt_symbol:   "❯"                     # renderer menambah trailing space → "❯ "
help_header:     "(^_^)? Available Commands"
tool_prefix:     "┊"
```
`get_branding("welcome", default)` digunakan; teks di atas adalah nilai default skin `default` (satu‑satunya yang terpasang pada instalasi ini).

**Nada/copywriting** (dari observasi string): 
- Hanya kapitalisasi di awal kalimat (bukan ALL CAPS), campur emoji Jepang‑kawaii (`(^_^)?`, `(>_<)`, `⚕`) dengan bahasa santai.
- Simbol prompt `❯` (berbeda dari skin `ares` `⚔`, `poseidon` `Ψ` dll.).
- Sapa selamat datang memakai emoji `⚕` (simbol medis) sebagai maskot; response ber‑label `⚕ Hermes`.

---

## 10. Output `--help` / `--version` (verbatim capture VM)

`usage: hermes [-h] [--version] [-z PROMPT] [--usage-file PATH] [-m MODEL] [--provider PROVIDER] [--reasoning LEVEL] [-t TOOLSETS] [--resume SESSION] [--no-restore-cwd] [--in DIR] [--continue [SESSION_NAME]] [--worktree] [--accept-hooks] [--skills SKILLS] [--yolo] [--pass-session-id] [--ignore-user-config] [--ignore-rules] [--safe-mode] [--tui] [--cli] [--dev] {chat,model,moa,fallback,worktree,browser,secrets,egress,migrate,gateway,proxy,lsp,setup,whatsapp,whatsapp-cloud,slack,send,login,logout,auth,status,pause,resume,cron,sync,webhook,peer,portal,kanban,project,hooks,doctor,verify,security,approvals,dump,debug,backup,checkpoints,import,import-agent,config,skin,console,pairing,skills,bundles,plugins,curator,pets,journey,learning,memory-graph,memory,tools,computer-use,mcp,sessions,insights,monitoring,claw,update,uninstall,acp,profile,completion,dashboard,serve,desktop,gui,logs,prompt-size} ...`

Baris deskripsi: `Hermes Agent - AI assistant with tool-calling capabilities`.

Flag yang relevan untuk paritas mode tampilan: **`--tui`** dan **`--cli`** (mode TUI vs classic CLI). (Capture bantuan sub‑perintah tiap command ada di source; cukup untuk referensi, tidak perlu semua disalin ke spesifikasi.)

---

## 11. Peta ke Hermes‑RS & keputusan paritas (DRAFT — menunggu Matt)

Perbedaan arsitektur kunci yang perlu keputusan Matt:
1. Python punya **dua mode** (`--tui` full chrome bawah vs `--cli` classic streaming). Hermes‑RS saat ini ratatui TUI. **Target paritas yang mana?** Rekomendasi awal: samakan **palet & elemen visual Hermes default** (emas/hitam) pada TUI Hermes‑RS yang sudah ada, dan adopsi branding/teks yang sama; bukan menyalin tata‑letak widget Python baris‑per‑baris.
2. Palet `default` Hermes (emas `#FFD700`/`#FFBF00`/`#CD7F32`/`#B8860B` + tekstur `#FFF8DC` + status‑bar navy `#1a1a2e`) harus dipetakan eksak ke `ratatui::style::Color::Rgb`.
3. Elemen yang bisa diadopsi langsung byte‑exact: ASCII `HERMES_AGENT_LOGO`, brand strings, `prompt_symbol ❯`, `response_label " ⚕ Hermes "`, frame kotak respons `╭─…╮/╰…╯`, `separator '─'`, `tool_prefix ┊`, status‑bar field set & warna, spinner braille `dots`.
4. Pemetaan warna ke Hermes‑RS harus **dark‑canonical** dahulu; light‑mode (`light_colors`) sebagai fase lanjutan.

Item **[BELUM TERVERIFIKASI / butuh konfirmasi]** yang harus dikunci sebelum T02:
- Nilai persis `_accent_hex()` pada skin default (fallback `#FFBF00` vs `#FFD700` di beberapa path) — perlu satu run Python `skin_engine.get_active_skin().get_color(...)` di VM untuk memastikan; tidak dianggap fixed di sini.
- Apakah reasoning di Hermes‑RS memakai `#CC9B1F` (`ui_thinking`) atau `_DIM` (dim/italic) seperti streaming Python.
- Pemakaian `light_colors` (mode terang) sebagai target atau hanya fase lanjutan.

---

## 12. Bukti & sumber (evidence)

File acuan yang sudah diekskavasi dari VM:
- `hermes_cli/skin_engine.py` (1068 baris) — palet skin + TUI style + branding.
- `hermes_cli/banner.py` (1310 baris) — logo ASCII, hero, welcome banner.
- `cli.py` (22.426 baris) — konstanta warna, streaming box, status bar, TUI layout.
- `agent/display.py` — `KawaiiSpinner` frames.
- Capture `hermes --version` / `hermes --help` (VM).

Dokumen ini disusun dari ekskavasi source langsung. Segala nilai yang tidak dapat dipastikan 100% dari source telah ditandai **[BELUM TERVERIFIKASI]**; **tidak ada nilai yang direkayasa** untuk menutup celah.
