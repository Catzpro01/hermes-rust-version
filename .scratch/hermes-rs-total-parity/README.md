# Spec 017 — Total Byte-Level Parity dengan Hermes Python v0.21.0

Paritas total (feel/look/interaksi) dengan Hermes Python v0.21.0:
banner, info line, setup wizard multi-step, katalog provider/toolset,
autocomplete /slash, tips, session picker. Sumber ground truth:
`~/.hermes/hermes-agent` (READ-ONLY, v0.21.0).

## Tiket

| # | Tiket | Status |
|---|---|---|
| Fase 0 | [Re-Archaeology total](issues/00-phase0-rearchaeology.md) | DONE — APPROVED Matt (8 koreksi diterima) |
| T01 | [inquire + wizard skeleton](issues/01-wizard-skeleton.md) | DONE — APPROVED Matt (fa69b80) |
| T02 | Banner v0.21.0 | APPROVED (per /ask-matt) — banner byte-identik dgn Python v0.21.0 (9 referensi) |
| T03 | Info line | DONE — MERGED (PR #1, `07093dc`) |
| T04 | Tips rotating | IN REVIEW — `issues/04-tips.md`; katalog 380+11 verbatim + selector, display menunggu Matt (§J.4) |
| T05 | Setup wizard multi-step (`hermes setup`) | DONE — MERGED (PR #1, `07093dc`) |
| T06 | Provider catalog (39 provider) | DONE — MERGED (PR #2, `319d719`) |
| 007b | Sandbox default on + `--no-sandbox` | DONE — MERGED (PR #2, `319d719`) |
| T07 | Toolsets catalog (26 toolset) | DONE — MERGED (PR #2, `319d719`) |
| T08 | Autocomplete /slash + ghost text (parity perilaku prompt_toolkit) | DONE — MERGED (PR #3, `8d66bbb`); verdict /ask-matt 2026-09-13: cap 60 char, skill discovery best-effort, TUI-only placeholder — disetujui |
| T09 | Session picker | Not started |
| T10 | Parity, docs & closure | Not started |

## Artefak Fase 0

- `docs/HERMES_UI_SPEC.md` — section "v0.21.0 Total Parity (Spec 017)".
- `docs/hermes-ui-spec/017/verbatim/` — 11 katalog/string verbatim
  (AST `unparse`), masing-masing dengan header provenance.
- Digest string per modul: VPS `/tmp/phase0/*.txt` (bukan repo).
