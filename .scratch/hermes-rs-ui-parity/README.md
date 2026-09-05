# Spec 013 — Hermes Python UI Parity

Vertical slice: mereplikasi **identitas visual Hermes Python** (skin default
"gold and kawaii": gold `#FFD700`, accent `#FFBF00`, bronze `#CD7F32`, teks
`#FFF8DC`, status bar navy `#1a1a2e`) ke Hermes-RS. Sumber kebenaran =
eksavasi **verbatim** dari instalasi Python asli `~/.hermes/hermes-agent`
(Ticket 01 → `docs/HERMES_UI_SPEC.md`), bukan tebakan UX.

## Prinsip (dikunci review Matt, Ticket 01)

- **Dark-canonical dulu.** `light_colors` ditunda (Spec 013b).
- `_accent_hex()` = `#FFBF00` (banner_accent); `#FFD700` khusus
  `banner_title` + `response_border`.
- Reasoning = `_DIM` (dim + italic), bukan `ui_thinking` yang tidak terverifikasi.
- Palet + branding + hierarki visual; **bukan** replikasi per-widget Python.
- Truecolor → fallback 256-color (`detect_color_depth` + `truecolor_to_256`).
- Sanitasi (ANSI scrub, redaksi) hanya di render boundary; byte kanonik
  `state.db` tak tersentuh. Banner TTY-only: stdout piped tetap byte-stable
  & bebas-ANSI (dijaga E2E).

## Tiket

| # | Tiket | Status |
|---|---|---|
| 01 | [Visual archaeology → HERMES_UI_SPEC.md](issues/01-visual-archaeology.md) | DONE (`7608316`) |
| 02 | [Color palette & theme system](issues/02-colors-theme.md) | DONE (`7608316`) |
| 03 | [Banner, prompt & strings](issues/03-banner-prompt-strings.md) | DONE (commit ini, review Matt pending) |
| 04 | [Streaming box, reasoning box & spinner](issues/04-streaming-box-spinner.md) | DONE (commit ini, review Matt pending) |
| 05 | [Status bar parity](issues/05-status-bar.md) | DONE (commit ini, review Matt pending) |
| 06 | Parity, docs, E2E closure proof (TBD) | Not started |

## Invariant yang tetap berlaku

Semua invariant `docs/ROADMAP.md`: `state.db` canonical; SIGINT exit 130;
credential terredaksi; instalasi Python Hermes **tidak disentuh** (hanya dibaca
sebagai sumber ekskavasi); UI tidak menambah surface eksekusi.
