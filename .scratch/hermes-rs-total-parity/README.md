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
| T03 | Info line | IN REVIEW — `issues/03-info-line.md`; post-banner hanya `WELCOME` (spec §B/§E koreksi) |
| T04 | Tips rotating | IN REVIEW — `issues/04-tips.md`; katalog 380+11 verbatim + selector, display menunggu Matt (§J.4) |
| T05 | Setup wizard multi-step (`hermes setup`) | IN REVIEW — `issues/05-setup-wizard.md`; 4 section + section tunggal, atomic write + backup, ESC = no-write |
| T06 | Provider catalog (39 provider) | IN REVIEW — `issues/06-provider-catalog.md`; `hermes model` TTY = section wizard |
| 007b | Sandbox default on + `--no-sandbox` | IN REVIEW — `issues/08-sandbox-default-on.md` |
| T07 | Toolsets catalog (26 toolset) | IN REVIEW — `issues/07-toolsets-catalog.md`; `hermes tools` baru |
| T08 | Autocomplete /slash + ghost text (parity perilaku prompt_toolkit) | IN REVIEW — `issues/09-autocomplete-slash.md`; registry 101 verbatim + stacked skills + path + trailing-space + Hinter ghost text; T04 opsi 3 = placeholder ghost di TUI |
| T09 | Session picker | Not started |
| T10 | Parity, docs & closure | Not started |

## Artefak Fase 0

- `docs/HERMES_UI_SPEC.md` — section "v0.21.0 Total Parity (Spec 017)".
- `docs/hermes-ui-spec/017/verbatim/` — 11 katalog/string verbatim
  (AST `unparse`), masing-masing dengan header provenance.
- Digest string per modul: VPS `/tmp/phase0/*.txt` (bukan repo).
