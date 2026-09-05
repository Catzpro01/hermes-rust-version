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

---

# v0.21.0 Total Parity (Spec 017) — Hasil Re-Archaeology (Fase 0)

> Ekstraksi AST verbatim dari `~/.hermes/hermes-agent` (Hermes Python
> **v0.21.0**, `pyproject.toml: version = "0.21.0"`), READ-ONLY.
> Semua string di bawah diambil lewat `ast`/`ast.unparse` (bukan
> transkripsi manual); katalog lengkap ada di
> `docs/hermes-ui-spec/017/verbatim/*.txt` (provenance: file + baris).
> Tanda **[BELUM TERVERIFIKASI]** = tidak bisa dikonfirmasi dari source;
> tanda **[KOREKSI]** = asumsi dispatch berbeda dari reality source.

## 0. Metode & Bukti

- Ekstraktor: `ast.parse` → (a) semua `ast.Constant` string + scope,
  (b) semua `ast.JoinedStr` (f-string) di-unparse, (c) assignment
  top-level dengan ≥3 string literal di-`ast.unparse` (katalog),
  (d) semua signature fungsi.
- File yang di-AST: `banner.py`, `setup.py`, `cli_agent_setup_mixin.py`,
  `curses_ui.py`, `completion.py`, `model_catalog.py`, `config_defaults.py`,
  `tools_config.py`, `main.py`, `models.py`, `tips.py`, `agent_import.py`,
  `commands.py`, `journey.py`, `console_engine.py`, `sessions_cmd.py`,
  `gateway.py` (hanya `_PLATFORMS`).
- Digest string per modul tersimpan di VPS `/tmp/phase0/*.txt` (16 file,
  ~16.000 record). Katalog verbatim final: `docs/hermes-ui-spec/017/verbatim/`.

## A. Header Banner (banner.py) — STRUKTUAL BEDA DARI ASUMSI

**[KOREKSI] Tidak ada** header 2-baris "⚕ NOUS HERMES - AI Agent Framework"
di v0.21.0 (tidak ditemukan di seluruh `hermes_cli/` maupun `agent/`).
**[KOREKSI] Tidak ada** box double-line `╔═╗` di code path banner: banner
adalah **satu Rich `Panel`** (border default Rich = single-line `┌─┐`,
`border_style = #CD7F32`) yang memuat **grid 2 kolom**
(`Table.grid(padding=(0, 2))`): kiri (center) = hero caduceus,
kanan (left) = baris info.

### A.1 Anatomi (banner.py `build_welcome_banner`, L968+)

```
Panel(
    Table.grid(padding=(0, 2)),     # kolom kiri justify=center, kanan left
    title=title_markup,             # bold #FFD700, [link=] ke release tag
    border_style=border_color,      # #CD7F32 (skin: banner_border)
)
```

- **Kiri**: `left_lines = ["", _hero, ""]` — hero = `HERMES_CADUCEUS`
  (15 baris braille, verbatim di `verbatim/banner_caduceus_art.txt`), bisa
  diganti skin (`banner_hero`).
- **`HERMES_AGENT_LOGO`** (6 baris block-art, verbatim di
  `verbatim/banner_logo_art.txt`) adalah konstanta lain (dipakai di luar
  startup banner — TUI/portal); warna per baris: baris 1-2 `[bold
  #FFD700]`, baris 3-4 `[#FFBF00]`, baris 5-6 `[#CD7F32]`.
- **Warna skin** (banner.py L1016-1019, L1288-1289, fallback default):
  | key | warna | dipakai |
  |---|---|---|
  | `banner_accent` | `#FFBF00` | nama model, header section |
  | `banner_dim` | `#B8860B` | pemisah `·`, label sekunder |
  | `banner_text` | `#FFF8DC` | body teks |
  | `session_border` | `#8B8682` | baris `Session:` |
  | `banner_title` | `#FFD700` | title panel (versi) |
  | `banner_border` | `#CD7F32` | border panel |

### A.2 Baris-baris kolom kanan (format verbatim, L1030-1294)

Urutan `right_lines` (yang ada saja yang dicetak):

1. **Model line** (3 varian):
   - MoA: `[{accent}]MoA: {preset_name}[/]{agg_str}{ctx_str} [dim {dim}]·[/] [dim {dim}]Nous Research[/]`
     dengan `agg_str = f' [dim {dim}]·[/] [dim {dim}]agg {agg_label}[/]'`
   - Normal: `[{accent}]{model_short}[/]{ctx_str} [dim {dim}]·[/] [dim {dim}]Nous Research[/]`
   - `ctx_str = f' [dim {dim}]·[/] [dim {dim}]{_format_context_length(context_length)} context[/]'`
   - Tanpa model: `[bold red]no model configured[/] [dim {dim}]— run /model or hermes setup[/]`
   - `model_short`: di-truncate dengan `...` (L1048/L1066); suffix `.gguf`
     ditangani khusus (L1063).
2. **YOLO** (jika `HERMES_YOLO_MODE`): `[bold red]⚠ YOLO mode[/] [dim {dim}]— all approval prompts bypassed[/]`
3. **CWD**: `[dim {dim}]{cwd}[/]`
4. **Session** (jika ada): `[dim {session_color}]Session: {session_id}[/]`
5. **Section `Available Tools`** (jika ada tools):
   header `[bold {accent}]Available Tools[/]`, lalu per toolset:
   `[dim {dim}]{toolset}:[/] {tools_str}` — nama tool warnai:
   unavailable → `[red]{name}[/]`, lazy → `[yellow]{name}[/]`,
   normal → `[{text}]{name}[/]`, dipisah `', '`; toolset tersisa →
   `[dim {dim}](and {remaining_toolsets} more toolsets...)[/]`
6. **Section `MCP Servers`** (jika ada `mcp_servers`):
   header `[bold {accent}]MCP Servers[/]`; per server:
   - connected: `[dim {dim}]{srv['name']}[/] [{text}]({srv['transport']})[/] [dim {dim}]—[/] [{text}]{srv['tools']} tool(s)[/]`
   - disabled: `[dim {dim}]{srv['name']}[/] [dim]({srv['transport']})[/] [dim {dim}]— disabled[/]`
   - connecting: sama + `[yellow]— connecting[/]`
   - configured: sama + `[dim {dim}]— configured[/]`
   - failed: `[red]{srv['name']}[/] [dim]({srv['transport']})[/] [red]— failed[/]`
7. **Section `Available Skills`** (jika ada skills):
   header `[bold {accent}]Available Skills[/]`; disabled →
   `[dim {dim}]Skills toolset disabled[/]`; per kategori:
   `[dim {dim}]{category}:[/] [{text}]{skills_str}[/]`
   (override → `[red]`, optional → `[yellow]`; `', +{n} more'` /
   `'+{n} more'`); kosong → `[dim {dim}]No skills installed[/]`
8. **Summary line**: `[dim {dim}]{' · '.join(summary_parts)}[/]` dengan
   parts = `{len(tools)} tools`, `{total_skills} skills`,
   `{mcp_connected} MCP servers`, `/help for commands`
   → contoh akhir: `5 tools · 3 skills · 1 MCP servers · /help for commands`
9. **Update notice** (jika behind, async prefetch non-blocking):
   `[bold yellow]⚠ {behind} commit(s) behind[/][dim yellow] — run [bold]{recommended_update_command()}[/bold] to update[/]`

### A.3 Version label (title panel)

`format_banner_version_label()` (banner.py L658):

```python
base = f"Hermes Agent v{VERSION} ({RELEASE_DATE})"
# up-to-date / tak ada git state:
f"{base} · upstream {upstream}"
# ahead of upstream:
f"{base} · upstream {upstream} · local {local} (+{ahead} carried {commit|commits})"
```

`title_markup = f"[bold {title_color}][link={_url}]{version_label}[/link][/]"`
(WithURL release tag terbaru; tanpa info release → tanpa `[link]`).

## B. Info Line — **[KOREKSI]**

**[KOREKSI]** Tidak ada bullet `●` dan tidak ada format
`"● {model} · N tools · toolsets: … · provider: …"`. Reality:

- Baris model = **A.2(1)** di atas (nama model accent `#FFBF00`, tanpa
  bullet, tanpa `provider:` — provider tidak dicetak di banner).
- Baris ringkas = **A.2(8)**: `N tools · M skills · K MCP servers ·
  /help for commands` (dim `#B8860B`, pemisah ` · `).
- `●` memang ada di v0.21.0, tapi di **recap sesi resume**:
  `  ● You: ` (dim bold, warna `session_label` `#DAA520`) dan
  `  ◆ Hermes: ` (bold `banner_text` `#FFF8DC`) — lihat F.4.

## C. Setup Wizard Multi-Step (setup.py + curses_ui.py)

### C.1 Arsitektur navigasi (curses_ui.py)

- Menu curses bersama: `_run_curses_menu(...)` (L638) — single event loop
  untuk radio (pilih satu) & multiselect; callback `draw_header`,
  `draw_row`, `on_action`; fallback non-curses = menu bernomor
  (`_numbered_fallback`, `input()`).
- **Hint baris 1 (A_DIM)**:
  - multiselect: `"  ↑↓ navigate  SPACE toggle  ENTER confirm  ESC cancel"`
    (+ `"  ← previous"` saat `back_enabled`)
  - radio/single: pola sama (`ENTER select`) — tergambar di `curses_radiolist` (L976+).
- **Key semantics** (`read_menu_key`, L483): bare ESC (tanpa continuation
  byte, short timeout) = `NAV_CANCEL`; `q` juga cancel; arrow decode
  CSI/SS3 manual (Ghostty/Kitty); `←` = back (scope setup);
  Ctrl+C = exit wizard; cursor row = **solid green** (pair 1), tanpa
  marker `●/○` — unselected = warna normal/yellow-dim.
- **Checkbox multiselect** (text fallback + curses): `"[✓]"` (GREEN) vs
  `"[ ]"` (curses_ui.py L1261; L927 `check = "✓" if i in chosen else " "`).
- **Fuzzy search** (`_token_score`, L264): port faithful dari
  `fuzzyScore` TS (`ui-tui/src/lib/fuzzy.ts`) — contiguous runs,
  word-boundary/first-char, prefix, exact > scattered subsequence;
  multi-token = AND, skor dijumlah; tie-break = indeks katalog.
- **Navigation scope** (`_handle_setup_menu_navigation`, setup.py L265):
  `MenuNavigationEvent` ∈ {begin, resolve, cancel, back};
  `_run_setup_steps` (L3041): *Left arrow di choice pertama sebuah section
  → kembali ke section sebelumnya; dari choice tersembunyi → replay
  pilihan sebelumnya tanpa terlihat lalu buka prompt sebelumnya*;
  cancel → pesan `Setup cancelled.`

### C.2 Alur `hermes setup` (setup.py `_run_setup_wizard_impl`, L3028+)

String verbatim (baris source):

- L3294: `No existing configuration found — running first-time setup.`
- L3303: **`How would you like to set up Hermes?`** dengan 3 opsi (L3305-3307):
  1. `Quick Setup (Nous Portal) — free OAuth login, no API keys, model + tools (recommended)`
  2. `Full setup — configure every provider, tool & option yourself (bring your own keys)`
  3. `Blank Slate — everything off except the bare minimum; opt in to each capability`
- Section wizard (label L3395-3398): `Model & Provider` →
  `Terminal Backend` → `Messaging Platforms` → `Tools`
  (subcommand: `hermes setup model|terminal|gateway|tools|agent`).
- L3282-3283: `Tip: jump straight to a section with 'hermes setup model|terminal|gateway|tools|agent', or fill only missing items with --quick.`
- L3280: `Press Enter to keep it, or type a new value to change it.`
- **Configuration Location** (L3338-3344):
  `Configuration Location` / `Config file:  {path}` / `Secrets file: {path}` /
  `Data folder:  {hermes_home}` / `Install dir:  {PROJECT_ROOT}` /
  `You can edit these files directly or use 'hermes config edit'`
- **Backup** (L3405-3407): `Previous config backed up to: {backup_path}` +
  `If setup changed a value you customized, restore it with:` +
  `  cp {backup_path} {config_path}`
- **OpenClaw import** (L2799 `Would you like to see what can be imported?`
  via `_offer_openclaw_migration`; L3348-3350): `Settings were imported from OpenClaw.` +
  `Each section below will show what was imported — press Enter to keep,` +
  `or choose to reconfigure if needed.`

### C.3 Step: Model & Provider (L952+)

- Section `Inference Provider`; L965: **`Choose how to connect to your main chat model.`**
- L966: `   Guide: {_DOCS_BASE}/integrations/providers`
- **Mendelegasi ke `cmd_model()`** — flow yang sama persis dengan
  `hermes model` (picker provider → kredensial → picker model). Satu code
  path; provider baru otomatis tersedia di setup.
- Error: `Provider setup encountered an error: {exc}` +
  `You can try again later with: hermes model`.

### C.4 Step: Terminal Backend (L1405+) — **[KOREKSI]**

- L1405: `Choose where Hermes runs shell commands and code.`
- L1458: **`Select terminal backend:`**
- Opsi yang **benar-benar wired di v0.21.0 hanya 2**:
  - `Terminal backend: Local` — `Commands run directly on this machine.`
  - `Terminal backend: Docker` — deteksi: `Docker not found in PATH!` /
    `Install Docker: https://docs.docker.com/get-docker/` /
    `Docker found: {docker_bin}`; image default `nikolaik/python-nodejs:python3.11-nodejs20`;
    egress firewall: `Docker sandboxes can be protected with the egress credential firewall.` /
    `It routes sandbox traffic through iron-proxy so containers receive proxy tokens instead of real API keys.` /
    `  Enable egress firewall for Docker sandboxes?` /
    `Egress firewall enabled in config` /
    `Run \`hermes egress setup\` then \`hermes egress start\` to mint tokens and launch the proxy.` /
    `Skipping egress firewall. You can enable it later with \`hermes egress setup\`.`
- **[KOREKSI] Modal / SSH / Daytona / Vercel Sandbox / Singularity TIDAK ada**:
  L1498 verbatim: `   Docker only for now; Modal, SSH, Daytona, and Singularity are not wired yet.`
  (String billing Modal L1559 `Select how Modal execution should be billed:`
  ada di code path, tapi backend-nya belum wired.)
- `Keeping current backend: {current_backend}` (jika sudah configured).

### C.5 Step: Messaging Platforms / Gateway (L2284+)

- **`Select platforms to configure:`** (L2284); instruksi L2270:
  **`Toggle with Space, confirm with Enter.`**
- Baris pilihan (L2280): `f"{plat['emoji']} {plat['label']}  ({status})"`;
  status ∈ `configured` | `not configured` | `partially` | `plugin disabled`.
- Tanpa pilihan: `No platforms selected. Run 'hermes setup gateway' later to configure.`
- Selesai: `Messaging platforms configured!`
- Home channel hilang (L2333-2339): `No home channel set for: {list}` +
  `   Without a home channel, cron jobs and cross-platform` +
  `   messages can't be delivered to those platforms.` +
  `   Set one later with /set-home in your chat, or:` +
  `     hermes config set {PLATFORM}_HOME_CHANNEL <channel_id>`
- Env vars per platform (L2314-2329): TELEGRAM_BOT_TOKEN / TELEGRAM_HOME_CHANNEL
  (Telegram), DISCORD_BOT_TOKEN / DISCORD_HOME_CHANNEL (Discord),
  SLACK_BOT_TOKEN / SLACK_HOME_CHANNEL (Slack), BLUEBUBBLES_HOME_CHANNEL /
  BLUEBUBBLES_SERVER_URL (BlueBubbles), QQ_APP_ID / QQBOT_HOME_CHANNEL /
  QQ_HOME_CHANNEL (QQBot).
- **Katalog platform** = gabungan (a) first-class (Telegram, Discord,
  Slack, WhatsApp, Signal, iMessage/BlueBubbles, QQ, Yuanbao, Mattermost,
  Weixin, …) dan (b) `_PLATFORMS` (gateway.py L6860, verbatim di
  `verbatim/platforms_gateway.txt`): 6 entry — Mattermost 💬, Signal 📡,
  Weixin/WeChat 💬, BlueBubbles (iMessage) 💬, QQ Bot 🐧, Yuanbao 💎 —
  masing-masing dengan `setup_instructions` (nomor langkah, verbatim) +
  `vars` (prompt/password/help). **[KOREKSI]** bukan satu katalog 27+
  platform dengan emoji — tidak ada list tunggal seperti itu di source.

### C.6 Step: Tools (L3386+ `setup_tools`)

- Tools step memakai flow `hermes tools` (checklist curses multiselect
  `[✓]/[ ]` dari `curses_ui.py`); katalog 26 toolset verbatim di
  `verbatim/toolsets_configurable.txt` (`CONFIGURABLE_TOOLSETS`,
  tools_config.py L100) — setiap tuple `(key, "emoji Judul", "tools...")`:
  web 🔍, browser 🌐, terminal 💻, file 📁, code_execution ⚡, vision 👁️,
  video 🎬, image_gen 🎨, video_gen 🎬, x_search 🐦, tts 🔊, stt 🎙️,
  skills 📚, todo 📋, memory 💾, context_engine 🧩, session_search 🔎,
  clarify ❓, delegation 👥, cronjob ⏰, homeassistant 🏠, spotify 🎵,
  discord 💬, discord_admin 🛡️, yuanbao 🤖, computer_use 🖱️.
- `[BELUM TERVERIFIKASI]` label persis `"Tools for 🖥️ CLI"` — tidak
  ditemukan di setup.py; header step-nya `Tools` (L3389).
- `_DEFAULT_OFF_TOOLSETS` (tools_config.py L159, verbatim) menentukan
  toolset yang mati default.

### C.7 Quick Setup & first-run (L3411+ / cli_agent_setup_mixin.py)

- Quick Setup (Nous Portal): `Nous Portal` / `One subscription, 300+ models, plus the Tool Gateway:` /
  `  web search, image generation, TTS, browser automation.` /
  `Sign up: https://portal.nousresearch.com/manage-subscription` /
  `Connect a messaging platform? (Telegram, Discord, etc.)` /
  opsi: `Set up messaging now (recommended)` | `Skip — set up later with 'hermes setup gateway'` /
  `Setup complete! You're ready to go.` / cancel: `Nous Portal setup cancelled.`
- **First-run offer** (REPL interaktif, belum ada provider, TTY —
  cli_agent_setup_mixin.py L255+):
  - `⚕ No inference provider is configured yet — let's fix that.`
  - `  You'll pick a provider (Nous Portal OAuth is the fastest; no API key needed) and a model.`
  - **`  Set up a provider now? [Y/n]: `**
  - skip: `  Skipped. Run 'hermes model' or 'hermes setup' any time.`
  - cancel: `  Setup cancelled. Run 'hermes model' any time.`
  - sukses: `  ✓ Provider configured — you're ready to chat.`
  - gagal: `  Provider setup didn't complete. Run 'hermes model' to retry.` /
    `  ⚠️  Provider setup failed: {exc}` + `  Run 'hermes model' to try again.`

## D. Autocomplete /slash + Completion (commands.py, completion.py)

### D.1 `COMMAND_REGISTRY` (commands.py L147, 101 entri)

`CommandDef(name, description, category, aliases=..., args_hint=...,
subcommands=..., busy_policy=..., cli_only/gateway_only/desktop=...)` —
**verbatim lengkap di `verbatim/commands_registry.txt`** (16.6 KB).
Kategori: `Session`, `Configuration`, `Info`. Contoh:
`CommandDef('clear', 'Clear screen and start a new session', 'Session', cli_only=True, desktop='terminal')`,
`CommandDef('palette', 'Open the fuzzy command palette (also Ctrl+P)', 'Info', ...)`.

### D.2 `SlashCommandCompleter` (commands.py L1632)

- Autocomplete: built-in slash commands + subcommands + **skill commands**
  (token skill dinormalisasi: underscore ≡ hyphen).
- **Stacked skill completions**: setelah `/skill-a ` baris yang masih
  seluruhnya token skill → tawarkan skill lain; deskripsi dibatasi
  `⚡ {short_desc}` (L1741).
- **Path completions**: word dianggap path jika mulai `./`, `../`, `~/`,
  `/`, atau mengandung `/` (kecuali `://` → URL, dikecualikan)
  (L1772-1806).
- **Trailing-space trick** (L1752): completion text = `{cmd} ` agar
  dropdown tetap visible & backspace memicu ulang; **KECUALI**
  `_PICKER_COMMANDS` = `model`, `personality`, `skin` — tanpa spasi
  (Enter boleh langsung membuka picker).
- `SlashCommandAutoSuggest` — ghost text (Tab accepts auto-suggestion,
  sesuai tips).

### D.3 Shell completion (completion.py)

`hermes completion bash|zsh|fish` — generator skrip verbatim
(generate_bash L100, generate_zsh L202, generate_fish L251): profil
(`-p/--profile` → list dari `~/.hermes/profiles/`), top-level subcommands
dengan `-d '{help}'`, sub-subcommand. **Ini completion SHELL, bukan
popup REPL** (asumsi dispatch T08 "ganti rustyline→reedline" = fitur
baru, bukan parity — lihat §9).

### D.4 Palette (Ctrl+P)

`/palette` terdaftar di registry (`Open the fuzzy command palette (also
Ctrl+P)`) dan placeholder composer menyebutnya, **tapi implementasi
fuzzy palette tidak ditemukan** di `hermes_cli/` maupun `agent/`
(hanya `journey._palette()` = warna Star Map, dan `skin_engine`
palette = warna skin). **[BELUM TERVERIFIKASI]** — kemungkinan fitur
desktop/TUI di luar paket Python yang terpasang.

## E. Tips & Composer Placeholders (tips.py) — **[KOREKSI]**

- **`TIPS` = 380 string** (tips.py L11; verbatim di
  `verbatim/tips.txt`, 36.3 KB) — isi: hint slash-command, `@file:` /
  `@url:` references, keybinding, `hermes <cmd>`, `config.yaml` options,
  env vars, tips tool.
- **`COMPOSER_PLACEHOLDERS` = 11 string** (L495; verbatim di
  `verbatim/composer_placeholders.txt`) — placeholder composer kosong
  (C-09, terinspirasi opencode/codex), contoh: `'Ask anything, or type / for commands…'`,
  `'Type / to browse commands, or Ctrl+P for the palette'`.
- Rotasi: `get_random_tip()` = **`random.choice(TIPS)`** (L485) — random,
  tanpa dedup (`exclude_recent` reserved, tidak dipakai).
- **[KOREKSI] TIDAK ADA prefix `✦ Tip:`** di source; dan
  **`get_random_tip` / `get_random_composer_placeholder` tidak punya
  caller di `hermes_cli/`** → keduanya **dead code di v0.21.0**
  (string tersedia, tidak pernah dicetak). `[BELUM TERVERIFIKASI]`
  caller dari paket lain (TUI/desktop) — tidak ada di checkout ini.
- Yang benar-benar tampil sebagai "Tip:" di v0.21.0 hanya string
  kontekstual, mis. setup.py L3282 (lihat C.2) dan update_cmd.py L10874
  `Tip: You can now select a provider and model:`.

## F. Session Picker (main.py `_session_browse_picker`, L1326)

`hermes sessions browse` (juga dipanggil `cmd_sessions` di
`sessions_cmd.py` L1208): curses browser + **live search filter**.

- **Header baris hint**:
  - normal: `  Browse sessions — ↑↓ navigate  Enter select  Type to filter  Esc quit`
  - saat mengetik filter: `  Browse sessions — filter: {search_text}█`
- **Baris sesi** (L1387):
  `f"{name:<{name_width}}  {status:<5}  {msgs_str:>5}  {last_active:<10}  {source:<5} {sid}"`
- **Header kolom** (L1464):
  `  {'Title / Preview':<{name_width}}  {'Stat':<5}  {'Msgs':>5}  {'Active':<10}  {'Src':<5} {ID}`
- Status lifecycle: `done` / `intr` / `err` / `empty` (dari
  `complete`/`interrupted`/`error`/`empty`, berwarna via `_status_attr`).
- **Footer**: `  {cursor+1}/{len(filtered)} sessions` (+
  ` (filtered from {len(sessions)})`) + `   d delete`.
- Filter kosong + tidak ada sesi: `No sessions found.`; ada filter
  tanpa hasil: `  No sessions match the filter.`; terminal kecil:
  `Terminal too small`.
- **Delete**: tekan `d` (filter harus kosong) →
  `  Delete session '{label}'? [y/N]` → `Deleted.` / `Delete failed.`
  (explicit [y/N] — selaras invariant Hermes-RS).
- Fallback non-curses (L1639): `\n  Browse sessions  (enter number to resume, q to cancel)\n`
- **[KOREKSI] Tidak ada opsi `n. New session`** di picker — new session
  = jalankan `hermes` tanpa argumen / `hermes -c` untuk resume.

## G. Provider Catalog (models.py + model_catalog.py)

### G.1 `CANONICAL_PROVIDERS` (models.py L1343, 39 entri)

`ProviderEntry(id, display_label, description)` — **verbatim lengkap di
`verbatim/providers_canonical.txt`**. Contoh (verbatim):
`ProviderEntry('nous', 'Nous Portal', 'Nous Portal (Everything your agent needs, 300+ models with bundled tool use)')`,
`ProviderEntry('fireworks', 'Fireworks AI', 'Fireworks AI (OpenAI-compatible direct model API)')`,
`ProviderEntry('openrouter', 'OpenRouter', 'OpenRouter (Pay-per-use API aggregator)')`.
39 id: nous, fireworks, openrouter, moa, novita, lmstudio, anthropic,
openai-codex, openai-api, alibaba (Qwen Cloud), xai-oauth, xiaomi,
tencent-tokenhub, tencent-tokenplan, nvidia, copilot, copilot-acp,
huggingface, gemini, vertex, deepseek, xai, zai, kimi-coding,
kimi-coding-cn, stepfun, minimax, minimax-oauth, minimax-cn, ollama-cloud,
arcee, gmi, kilocode, opencode-zen, opencode-go, bedrock, azure-foundry,
ai-gateway, qwen-oauth.
**[KOREKSI]** = 39, bukan "40+", dan **tidak ada field
`api_url`/`key_env`/`has_submenu` di entry** — base URL & key_env
di-resolve dari config (`entry.get("key_env") or entry.get("api_key_env")`,
models.py L3370).

### G.2 Relasi katalog lain (models.py)

- `_PROVIDER_MODELS` (L270): model list per provider (fallback statis).
- `_PROVIDER_ALIASES` (L1514, verbatim di `verbatim/provider_aliases.txt`).
- `_PROVIDER_RETIRED_ALIASES` (L3715).
- `_AGGREGATOR_PROVIDERS` (L3735, verbatim), `_LIVE_FIRST_PICKER_PROVIDERS` (L3776).
- `list_available_providers()` (L3152): sumber truth =
  `CANONICAL_PROVIDERS` (dipakai `hermes model`, `/model`, dsb.).

### G.3 Remote model catalog (model_catalog.py)

- Manifest JSON: `https://hermes-agent.nousresearch.com/docs/api/model-catalog.json`
  (fallback: `https://raw.githubusercontent.com/NousResearch/hermes-agent/main/website/static/api/model-catalog.json`).
- Cache: `~/.hermes/cache/model_catalog.json` (atomic write `.tmp`+rename),
  TTL config `model_catalog.ttl_hours`, SWR refresh off-thread,
  User-Agent `hermes-cli/{VERSION}`.
- Schema v1: `{version, updated_at, metadata, providers: {<name>: {metadata, models: [{id, description, metadata, default?}]}}}`.
- `get_default_model_from_cache(provider)` — **jangan pernah fetch
  network** di path hot (cache-only; fallback konstanta in-repo).
- Model picker = live API list provider (jika bisa) → fallback
  `_PROVIDER_MODELS` statis (`curated_models_for_provider`, L3687).

## H. Resume Display (bonus — terkait session UX)

- `↻ Resumed session [bold]{id}[/bold]{ "title"} ({n} user message(s), {m} total messages)`
  (accent `banner_accent`; cli_agent_setup_mixin.py L750)
- `Session {id} found but has no messages. Starting fresh.`
- Recap: header `Previous Conversation` (dim `session_label` #DAA520),
  border `session_border` #8B8682, `ui_ok` #8FBC8F;
  `  ● You: ` (dim bold), `  ◆ Hermes: ` (bold #FFF8DC);
  event: `  ◈ {text}` (dim italic);
  tool calls: `[N tool calls: name1, name2, ...]`;
  terpotong: `  ... {n} earlier messages ...` (dim italic).

## I. Daftar [BELUM TERVERIFIKASI] / [KOREKSI] (ringkas)

| # | Asumsi dispatch v0.21.0 | Reality source | Status |
|---|---|---|---|
| 1 | Header `⚕ NOUS HERMES - AI Agent Framework` 2 baris | Tidak ada; title panel = `Hermes Agent v{V} ({DATE}) · upstream {sha}` bold gold + link | **[KOREKSI]** |
| 2 | Box double-line `╔═╗` | Rich `Panel` default (single-line `┌─┐`), border `#CD7F32` | **[KOREKSI]** (kecuali skin custom) |
| 3 | Info line `● {model} · N tools · toolsets: …` | Model line accent (tanpa bullet) + summary line `N tools · M skills · K MCP servers · /help for commands` | **[KOREKSI]** |
| 4 | Wizard: `Would you like to see what can be imported?` (Step 1) | Ada, tapi hanya di jalur **OpenClaw migration** (`_offer_openclaw_migration` L2799) | terkonfirmasi, scope beda |
| 5 | Terminal backend 7 opsi | Hanya **Local + Docker** wired; sisanya "not wired yet" (L1498 verbatim) | **[KOREKSI]** |
| 6 | 27+ chat platform satu katalog | Tidak ada list tunggal; first-class + `_PLATFORMS` (6 entry) | **[KOREKSI]** |
| 7 | 40+ provider catalog + api_url/key_env/has_submenu | 39 `ProviderEntry(id,label,description)`; URL/key_env dari config | **[KOREKSI]** |
| 8 | `✦ Tip:` rotating | `TIPS` (380) ada tapi **dead code** (tanpa caller); tidak ada prefix `✦` | **[KOREKSI]** |
| 9 | Autocomplete popup = ganti rustyline→reedline | Python pakai **prompt_toolkit** `SlashCommandCompleter` + AutoSuggest (ghost text); popup = dropdown prompt_toolkit | paritas = perilaku, bukan crate |
| 10 | Session picker "n. New session" | Tidak ada; delete via `d` + `[y/N]`; fallback numbered + `q` | **[KOREKSI]** |
| 11 | `Ctrl+P` fuzzy palette | Terdaftar di command registry; implementasi tidak ditemukan di Python checkout | **[BELUM TERVERIFIKASI]** |
| 12 | Label `"Tools for 🖥️ CLI"` | Header step `Tools` (L3389) | **[BELUM TERVERIFIKASI]** label persis |

## J. Implikasi port Hermes-RS (input T01-T10)

1. **Banner T02 (spec 017)**: bukan bikin box baru — samakan *panel title
   (version label) + grid 2 kolom + urutan baris kanan* dari §A. Logo
   block-art (`HERMES_AGENT_LOGO`) untuk TUI, caduceus 15 baris untuk
   REPL banner (sudah ada di Hermes-RS sejak Spec 013 — cocokkan warna
   per-baris: 2 baris pertama gold bold, dst., lihat art files verbatim).
2. **Wizard T05**: bangun di atas helper `inquire` (atau curses-like TUI
   crate) dengan semantics key persis §C.1 (bare ESC cancel, `q` cancel,
   `←` back, SPACE toggle, ENTER confirm; hint baris 1 dim). Config write
   = atomic + backup (sudah invariant spec 017). String pertanyaan/opsi
   **wajib verbatim** (§C.2-C.7).
3. **Catalog T06/T07**: salin `verbatim/providers_canonical.txt`
   (39 entry, `{id, label, description}`) dan
   `verbatim/toolsets_configurable.txt` (26 tuple `{key, label, tools}`)
   sebagai static data — **tanpa** field tambahan (api_url/key_env/
   submenu) karena tidak ada di source; key_env di-resolve dari config
   seperti Python.
4. **Tips T04**: `verbatim/tips.txt` (380) + `composer_placeholders.txt`
   (11) — keputusan Matt: port sebagai fitur baru (Python-nya dead code)
   atau skip; format display `✦ Tip:` **tidak ada** di Python.
5. **Autocomplete T08**: parity perilaku = completions
   commands+subcommands+skills, path completion, stacked skills
   (`⚡ desc`), trailing-space trick, picker commands tanpa spasi, ghost
   text — di atas readline Rust apa pun (reedline/inquire optional).
6. **Session picker T09**: format §F verbatim (baris, header, hint,
   footer, `d`+[y/N], fallback numbered).
7. **Parity T10**: side-by-side capture wajib mencakup: banner panel
   (title + grid), setup wizard tiap step (curses frame), session
   picker frame, completion dropdown (string candidates), summary line.

## K. Provenance per item (file:baris Python v0.21.0)

| Item | Source |
|---|---|
| Logo/caduceus art | banner.py L70 / L77 |
| Skin keys | banner.py L1016-1019, L1288-1289 |
| Model line / MoA / YOLO / cwd / Session | banner.py L1030-1074 |
| Available Tools / MCP / Skills / summary | banner.py L1077-1244 |
| Version label | banner.py L658-672, L1294 |
| Update notice | banner.py L736-744 |
| Setup mode question + opsi | setup.py L3303-3307 |
| Section wizard | setup.py L3395-3398 |
| Config backup | setup.py L3405-3407 |
| Terminal backend | setup.py L1458-1511 |
| Gateway platforms | setup.py L2270-2339; gateway.py L6860 |
| Quick setup | setup.py L3411-3480 |
| First-run offer | cli_agent_setup_mixin.py L255-316 |
| Resume recap | cli_agent_setup_mixin.py L747-956 |
| Menu nav/hint/key semantics | curses_ui.py L483-637, L917, L976+ |
| COMMAND_REGISTRY | commands.py L147-1414 |
| SlashCommandCompleter | commands.py L1632-1830 |
| Shell completion | completion.py L100, L202, L251 |
| TIPS / placeholders / rotation | tips.py L11, L478-485, L495-511 |
| Session picker | main.py L1326-1650 |
| CANONICAL_PROVIDERS | models.py L1343-1382 |
| Remote catalog | model_catalog.py L66-516 |

