# Spec 017 — Total Byte-Level Parity dengan Hermes Python v0.21.0

Paritas total (feel/look/interaksi) dengan Hermes Python v0.21.0:
banner, info line, setup wizard multi-step, katalog provider/toolset,
autocomplete /slash, tips, session picker. Sumber ground truth:
`~/.hermes/hermes-agent` (READ-ONLY, v0.21.0).

## Tiket

| # | Tiket | Status |
|---|---|---|
| Fase 0 | [Re-Archaeology total](issues/00-phase0-rearchaeology.md) | DONE — menunggu review Matt (HARD STOP sebelum coding) |
| T01 | `inquire` + wizard skeleton | Not started |
| T02 | Banner v0.21.0 | Not started |
| T03 | Info line | Not started |
| T04 | Tips rotating | Not started |
| T05 | Setup wizard multi-step (`hermes setup`) | Not started |
| T06 | Provider catalog (39 provider) | Not started |
| T07 | Toolsets catalog (26 toolset) | Not started |
| T08 | Autocomplete /slash (parity perilaku prompt_toolkit) | Not started |
| T09 | Session picker | Not started |
| T10 | Parity, docs & closure | Not started |

## Artefak Fase 0

- `docs/HERMES_UI_SPEC.md` — section "v0.21.0 Total Parity (Spec 017)".
- `docs/hermes-ui-spec/017/verbatim/` — 11 katalog/string verbatim
  (AST `unparse`), masing-masing dengan header provenance.
- Digest string per modul: VPS `/tmp/phase0/*.txt` (bukan repo).
