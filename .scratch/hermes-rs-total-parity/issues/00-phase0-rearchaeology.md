# 017-Fase 0: Re-Archaeology Total (Hermes Python v0.21.0)
**Status:** DONE — menunggu review Matt. HARD STOP: coding T01+ hanya
setelah spec di-approve.

## Cakupan

Ekstraksi AST verbatim (bukan transkripsi) untuk 7 area dispatch
(A banner, B info line, C wizard, D autocomplete, E tips, F session
picker, G provider catalog) dari `~/.hermes/hermes-agent` v0.21.0:

- 17 modul di-AST (banner, setup, curses_ui, commands, models, tips,
  sessions_cmd, gateway, main, dsb.); ~16.000 record string/f-string/
  signature → digest VPS `/tmp/phase0/*.txt`.
- 11 katalog/string kunci di-`ast.unparse` →
  `docs/hermes-ui-spec/017/verbatim/*.txt` (provenance file+baris).
- Section baru di `docs/HERMES_UI_SPEC.md` dengan format verbatim +
  tabel koreksi asumsi + provenance per item.

## Temuan kunci (vs asumsi dispatch)

1. Banner = Rich Panel (single-line) + grid 2 kolom (caduceus | info);
   **bukan** box `╔═╗`, **bukan** header "⚕ NOUS HERMES".
2. Info line **tanpa** bullet `●`; = model line accent + summary line
   `N tools · M skills · K MCP servers · /help for commands`.
3. Wizard: `How would you like to set up Hermes?` (Quick/Full/Blank
   Slate), 4 section; navigasi curses (bare ESC cancel, q cancel,
   ← back, SPACE toggle, ENTER confirm, hint baris dim).
4. Terminal backend hanya **Local + Docker** wired (Modal/SSH/Daytona/
   Singularity "not wired yet" — verbatim L1498).
5. Provider = 39 `ProviderEntry(id,label,description)` (bukan 40+,
   tanpa field api_url/key_env); toolset = 26 tuple verbatim.
6. `TIPS` (380 string) + composer placeholders (11) **dead code**
   (tanpa caller); tidak ada prefix `✦ Tip:`.
7. Autocomplete = prompt_toolkit completer (commands+subcommands+skills,
   path completion, stacked skills `⚡`, trailing-space trick, picker
   commands tanpa spasi, ghost text). `Ctrl+P` palette terdaftar tapi
   implementasi tak ditemukan [BELUM TERVERIFIKASI].
8. Session picker: format baris/header/hint verbatim, delete `d`+[y/N],
   **tanpa** opsi "n. New session".

## Bukti

- Semua string di spec bersumber dari AST (reproducible:
  `/tmp/phase0_extract.py` di VPS).
- Python source READ-ONLY: `smoke_python_hermes_untouched` tetap green
  (tidak ada write ke `~/.hermes/hermes-agent`).
