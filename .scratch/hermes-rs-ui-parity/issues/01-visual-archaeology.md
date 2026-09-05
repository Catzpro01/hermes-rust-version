# 013-01: Visual Archaeology (HERMES_UI_SPEC)
**Status:** DONE (commit `7608316`).

## Cakupan
- Eksekavasi **verbatim** dari `~/.hermes/hermes-agent` (source Python asli,
  bukan tebakan): 40+ warna hex skin `default`, logo ASCII 6 baris, braille
  caduceus, prompt `❯`, `response_label " ⚕ Hermes "`, separator `─`,
  `tool_prefix ┊`, style dict prompt_toolkit §8, branding §9, layout
  bottom-chrome §8, kotak streaming §5, spinner/kawaii §7.
- Deliverable: `docs/HERMES_UI_SPEC.md` (12 bagian + evidence list).

## Keputusan terkunci (review Matt)
- Accent = `#FFBF00`; `#FFD700` untuk `banner_title`/`response_border`.
- Reasoning = `_DIM` (dim + italic).
- Dark-canonical; `light_colors` fase lanjutan.
- Item [BELUM TERVERIFIKASI] diberi penanda eksplisit; tidak ada nilai direkayasa.
