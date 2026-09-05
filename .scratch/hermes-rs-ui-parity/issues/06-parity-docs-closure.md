# 013-06: Parity, Docs & Closure
**Status:** DONE — menunggu review Matt (commit ini).

## Cakupan (Ticket 06)
Verifikasi side-by-side visual Hermes-RS (REPL & TUI) vs Hermes Python
default skin "gold and kawaii", pembaruan `docs/PARITY.md` +
`docs/ROADMAP.md`, penutupan Spec 013 + Phase 5 (Ecosystem). **Tanpa
perubahan kode** — dokumen + bukti saja (constraint penerimaan T06).

## Metode
1. **Capture Python (live, read-only).** Interpreter asli venv
   (`~/.hermes/hermes-agent/venv/bin/python`): `--help` →
   `/tmp/t06_py_help.txt` (14,149 B), `--version` →
   `/tmp/t06_py_version.txt`. Python CLI interaktif membutuhkan provider
   live (tak ada fake provider di sisi Python — `fake_boot` di `main.py:8538`
   hanya flag GUI desktop), jadi ground truth in-app = capture verbatim T01
   (`docs/HERMES_UI_SPEC.md`) + source (`cli.py`, `hermes_cli/banner.py`).
2. **Capture Hermes-RS via pty** (Python `os.forkpty` + `TIOCSWINSZ`,
   driver `COLUMNS`/`LINES`) — byte mentah **termasuk escape SGR**:
   - REPL 80 col — sesi penuh (banner, `hello`, input reasoning, `tool`,
     input panjang >60 sel, `/exit`): `t06_rs_repl_80.bin`
   - REPL 50/60/100 col — tier status bar (compact/medium/full):
     `t06_rs_repl_{50,60,100}.bin`
   - REPL 110 col — logo penuh (≥101 col): `t06_rs_repl_110.bin`
   - TUI 100 col (`--tui`, exit `q`, status 0): `t06_rs_tui_100.bin`
   - `--help`/`--version` stdout: `t06_rs_help.txt`, `t06_rs_version.txt`
   (semua di `/tmp/t06_out/` — salinan permanen transkrip 80 col: `06-evidence-transcript.txt` di folder ini; skrip driver `t06_capture.py`,
   `t06_logodiff2.py`, `t06_round2.py` di `/tmp/`)
3. **Diff byte-level** elemen per elemen terhadap source Python.

## Hasil Verifikasi
- **Logo figlet `HERMES_AGENT_LOGO` (tampil ≥95 col):** 6/6 baris
  byte-identik (101/101/98/98/98/98) + tier SGR identik (baris 1-2
  `1;38;2;255;215;0` bold gold, 3-4 `38;2;255;191;0` `#FFBF00`, 5-6
  `38;2;205;127;50` `#CD7F32`) — diverifikasi live di 110 col.
  **Edge ditemukan:** tepat di 100 col, sel terakhir baris 1-2 ter-clip
  (baris 101 sel dalam buffer panel 100 col); di ≥101 col tampil penuh.
  Python sendiri wrap baris 101 col di lebar 100 — beda 1 sel,
  didokumentasikan di PARITY.md (bukan cacat data: konstanta
  `art.rs` byte-exact, ter-pin `logo_is_verbatim_bytes`).
- **Caduceus:** 15/15 baris identik (teks 30 sel + warna
  `#CD7F32`/`#FFBF00`/`#FFD700`/`#B8860B`) vs `HERMES_CADUCEUS`
  (`banner.py:77`).
- **Border panel:** `#CD7F32` — capture `38;2;205;127;50` ✓ spec §3.
- **Prompt `❯ `:** hermes-rs string polos (warisi terminal) — sama dengan
  Python (style `'prompt': ''`, spec §2 baris 85/262) ✓.
- **Kotak respons:** header/footer `╭─ ⚕ Hermes ──╮`/`╰──╯` bold
  `#FFD700` (`1;38;2;255;215;0`), isi `#FFF8DC` (`38;2;255;248;220`) ✓
  spec §5.1; lebar = baris terpanjang, clamp ke terminal (test T04).
- **Baris tool:** `  ┊ ◇ read_file` SGR `2;3` (dim+italic) ✓ spec §6.
- **Status bar (tier full, 80 col):** baris full-width bg navy
  `48;2;26;26;46`; model bold gold; separator ` │ `, bar `[░░░░░░░░░░]`,
  `--` (ctx tak dikenal → Dim), durasi — semua dim `38;2;139;134;130`
  `#8B8682`; padding tepat ke lebar terminal ✓ spec §8 + T05 (22 unit
  test). Tier 50/60/100 col terekam (`t06_rs_repl_{50,60,100}.bin`).
- **Goodbye:** `Goodbye! ⚕` (capture) = `cli.py:18000` ✓.
- **Reasoning box + spinner:** ter-pin byte-level di test e2e/unit T04 —
  tak bisa live-capture: `fake` provider tak punya trigger reasoning dan
  respons <120 ms (spinner ter-clear sebelum frame pertama). Ditandai
  "test-verified" di tabel PARITY.md.
- **Perbedaan terdokumentasi** (PARITY.md, bukan cacat):
  - brand strings `Hermes-RS …` vs `Hermes Agent …` (keputusan T02,
    di-approve);
  - pesan user = echo prompt `❯ <teks>` vs baris scrollback Python
    `─`×40 + `●` bold (model echo line-REPL; batasan inherent, diterima
    review Matt di T05);
  - `[iter N/10]` = baris informasi Rust-only (Python tak mencetak);
  - `--help`/`--version` = subset Rust (`hermes-rs 0.1.0` vs
    `Hermes Agent vX`) — scope Spec 013 = look & feel in-app; flag
    mode-tampilan `--tui` ada di kedua sisi.

## Perubahan Dokumen
- `docs/PARITY.md`: seksi baru **"Spec 013 — UI parity (visual)"** —
  tabel elemen + verdict, pernyataan
  **"Status-bar visual customizers: 100% compatible"** (tier, separator,
  threshold konteks, gauge, title badge, pad/trim full-width), lokasi
  artefak capture.
- `docs/ROADMAP.md`:
  - baris fase 010 → `Deferred (v2.0 backlog — Spec 011b)`;
  - baris baru 013 → `Done`;
  - seksi baru **"Spec 013 — Hermes Python UI parity closure"**
    (T01–T06 + commit + non-negotiables yang terjaga);
  - **Phase 5 (Ecosystem) = 100% DONE** (011/012/013 Done; 010 deferred
    formal per keputusan Spec 011b) — catatan fase di-rewrite;
  - Verification → 400 passed (2026-09-05).
- `.scratch/hermes-rs-ui-parity/README.md`: baris tiket 03–05 →
  DONE + APPROVED (commit), baris 06 → DONE.

## Bukti (gate)
- `cargo test --workspace` → RC 0, **400 passed / 0 failed** — log VM
  `/tmp/t06_test.log` (termasuk `smoke_python_hermes_untouched`: folder
  Python tidak disentuh).
- `cargo clippy --workspace --all-targets -- -D warnings` → RC 0 — log VM
  `/tmp/t06_clippy.log`.
- Tree bersih di `c7a2950` sebelum commit ini; tanpa perubahan kode.
