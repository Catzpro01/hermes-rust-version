# T04 — Tips rotating (Spec 017)

Status: IN REVIEW (CI-verified) — data + selector port; **display wiring
menunggu keputusan Matt** (spec §J.4).

## Fakta (spec §E [KOREKSI])

- `TIPS` = 380 string, `COMPOSER_PLACEHOLDERS` = 11 string (verbatim).
- `get_random_tip()` = `random.choice(TIPS)`, tanpa dedup.
- **Tidak ada** prefix `✦ Tip:`; kedua helper **dead code** di v0.21.0
  (tidak pernah dicetak di `hermes_cli/`).

## Yang dikerjakan

- `crates/hermes-cli/src/tui/tips.rs`: `TIPS`, `COMPOSER_PLACEHOLDERS`
  (di-generate dari `docs/hermes-ui-spec/017/verbatim/*.txt`),
  `random_tip()`, `random_composer_placeholder()`, `tip_at(seed)`.
- Tes: `catalog_matches_verbatim_files` re-parse file verbatim dan
  membandingkan 380/11 string satu per satu; `no_display_prefix_in_catalog`;
  `selectors_return_catalog_members`.
- Parity-faithful: modul `#![allow(dead_code)]`, tidak ada caller — sama
  seperti upstream. Hardcoded `✦ Tip: BROWSER_CDP_URL…` lama sudah dihapus
  di T03.

## Keputusan Matt yang dibutuhkan (opsi)

1. **Skip display** (parity murni) — tidak ada tip di startup. Default
   saat ini.
2. **Fitur baru**: cetak `random_tip()` dim satu baris setelah `WELCOME`
   (format tanpa `✦`, mis. `Tip: {tip}` seperti string kontekstual
   setup.py/update_cmd.py) — perlu flag `display.tips: true` agar opt-in.
3. **Composer placeholder**: pakai `random_composer_placeholder()` sebagai
   ghost text prompt kosong di TUI (T08 wilayah autocomplete).
