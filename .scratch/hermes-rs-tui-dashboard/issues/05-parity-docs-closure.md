# 012-05: Parity, docs, E2E closure proof

**Status:** breakdown.

**What to build:** Uji end-to-end TUI (tanpa terminal sungguhan — pakai buffer
off-screen / terminal headless Ratatui), pembaruan PARITY/ROADMAP/SECURITY,
dan penutupan Spec 012. Mengikuti pola closure Spec 004–011.

## Kriteria

- [ ] E2E: jalankan `App` dgn mock event stream (token/tool/status) di terminal
      buffer off-screen; assert panel berisi data yg diharapkan & tidak ada teks
      mentah tak ter-redaksi/ANSI.
- [ ] Flag `--tui` mengaktifkan jalur TUI; tanpa flag REPL readline identik
      (regresi nol; suite lama hijau).
- [ ] Sanitasi: teks model/tool di semua panel sudah bersih (tidak ada kredensial
      / ANSI mentah) — test menyuntik kredensial & assert tak tampil.
- [ ] SIGINT/keluar rapi → restore terminal, exit code konsisten.
- [ ] Regresi: seluruh suite hijau; jumlah test dilaporkan.
- [ ] `docs/PARITY.md` — section Spec 012; `docs/ROADMAP.md` — Spec 012 Done
      (hanya setelah suite hijau).
- [ ] `docs/SECURITY.md` — catatan sanitasi TUI (UI bukan jalur baru).
- [ ] `smoke_python_hermes_untouched` tetap lulus.

## Perubahan (prakiraan)

- `crates/hermes-cli/Cargo.toml` — `ratatui`, `crossterm`.
- `crates/hermes-cli/src/tui/**` (event, app, layout, panels) + `--tui` di main.
- Reuse `ConversationRunner` aksesor & `sanitize_untrusted_output`/redaksi.
- `docs/PARITY.md`, `docs/ROADMAP.md`, `docs/SECURITY.md`.

## Dependency

01–04.
