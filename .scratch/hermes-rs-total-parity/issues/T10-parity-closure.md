# T10 — Parity, docs & closure (Spec 017)

Status: IN REVIEW — dokumen tersedia, closure belum disetujui.

Review 2026-09-14: lihat [T11](T11-closure-review.md) untuk koreksi bukti,
gate format CI, dan persyaratan §J.7 yang masih terbuka.

## Cakupan (tiket penutup, tanpa perubahan kode)

Sinkronisasi dokumen dengan kode yang sudah mendarat (Fase 0–T09),
reconcile status T04, dan bukti closure. Constraint penerimaan seperti
013-06/014-08: dokumen + bukti saja.

## Perubahan dokumen pada T10 awal (sebelum review T11)

- `docs/ROADMAP.md`: baris fase 017 → `Done`; seksi baru "Spec 017 —
  total v0.21.0 parity closure" (tabel tiket + komit/PR, closure proof,
  non-negotiables, tiket lanjutan); Verification → run terakhir
  (PR #6 CI: 576 passed, clippy `-D warnings` bersih).
- `docs/PARITY.md`: seksi baru "Spec 017 — total v0.21.0 parity" (tabel
  elemen + verdict per area); baris `setup` di tabel Spec 014 → ✅;
  baris baru `sessions browse` → ✅.
- Board ini: T04 → DONE (reconcile, lihat bawah), T09 → MERGED (PR #6),
  T10 → DONE.

## Reconcile T04

Baris README T04 sebelumnya "IN REVIEW — display menunggu Matt (§J.4)".
Keputusan display sudah mendarat semuanya:

- Opsi 1 (skip display, parity murni) = perilaku startup: tidak ada tip
  setelah `WELCOME` (T03 menghapus hardcoded `✦ Tip:`; modul `tips.rs`
  parity-faithful tanpa caller untuk path startup).
- Opsi 3 (composer placeholder) = ghost text prompt kosong di TUI,
  diimplementasikan di T08 (verdict /ask-matt: TUI-only — disetujui).
- Opsi 2 (fitur baru `Tip:` setelah WELCOME) = DITOLAK implisit:
  tidak ada flag `display.tips`, tidak diimplementasikan.

T04 = DONE, tidak ada keputusan display yang menggantung.

## Bukti parity yang tersedia — verifikasi mesin (belum menutup §J.7)

Side-by-side capture manual (§J.7: banner panel, wizard steps, picker
frame, completion dropdown, summary line) belum tersedia sebagai satu set
lengkap di repo. Ada sembilan referensi banner ternormalisasi di
`welcome.rs`; PTY E2E menjalankan binary nyata di terminal semu dan menegaskan string/perilaku terpilih, tetapi tidak membuktikan
seluruh frame byte-identik dengan Python. Penggantian bukti wajib §J.7
memerlukan persetujuan eksplisit; review ini tidak memberikan sign-off.
Bukti otomatis yang tersedia:

| Artefak §J.7 | Bukti otomatis |
|---|---|
| Banner panel (title + grid) | `welcome.rs`: 9 referensi plain-text ternormalisasi; `banner_e2e.rs` (PTY): title gold bold, border bronze, logo ≥95, welcome-only setelah banner |
| Setup wizard tiap step | `wizard_e2e.rs` (PTY): first-time, not-wired, tulis atomik, ESC rollback, backup + merge |
| Session picker frame | `session_picker_e2e.rs` (7 PTY): frame verbatim, Enter/↓/filter/`d`+`y`/Esc |
| Completion dropdown (candidates) | unit `completion.rs` (101 entri vs verbatim, alias, subcommand, stacked skills, path, ghost) |
| Summary line | unit `summary_line_matches_python_join_rule` + banner grid (T02/T03) |

Pin verbatim tambahan (unit): `art.rs` byte-exact (logo/caduceus),
katalog 39/26/8 vs file verbatim, string §F per-spasi, tips 380/11
vs verbatim.

## Closure proof (Spec 017)

CI `fmt + clippy + test` pada PR #6 (T09, komit terakhir kode Spec 017):
clippy `--workspace --all-targets -D warnings` bersih; `cargo test
--workspace` — 576 passed, 0 failed (termasuk 23 tes T09: 11 unit
picker, 1 core delete, 4 piped E2E, 7 PTY E2E; suite `subcommands_e2e`
33/33, `session_picker_e2e` 7/7).

Catatan review: angka 576 dan clippy success dikonfirmasi dari anotasi
check GitHub `103764353101`; status fmt tidak terbukti oleh CI hijau
karena `|| true` pada langkah format lama. Belum ada run lokal baru.

Non-negotiables yang dicatat sepanjang slice: instalasi Python tidak
boleh disentuh. `smoke_python_hermes_untouched` hanya membandingkan mtime
`~/.hermes/state.db` bila home ada (skip bila tidak), bukan audit seluruh
instalasi Python; `state.db` kanonik (browse
read-only kecuali `d`+`y` eksplisit); SIGINT exit 130 (termasuk di
dalam picker); kredensial terredaksi di semua path output; sanitasi
hanya di boundary render; ANSI hanya di TTY (piped byte-stable);
tulis config atomik + backup; konfirmasi `[y/N]` default-deny.

## Tiket lanjutan (di luar Spec 017, eksplisit tidak diklaim)

1. Resume display §H (`↻ Resumed session …` + recap `●`/`◆`).
2. `sessions list/stats/prune/export/rename/delete` (+ `--source` filter).
3. Live model list per provider (§G.3 remote catalog + cache).
4. Nous Portal OAuth (saat ini: notice eksplisit + picker biasa).
5. Egress firewall Docker (saat setup backend Docker dieksekusi).
6. `tools.enabled_toolsets` → wiring ke registry tool Rust.
7. Verifikasi palette Ctrl+P (implementasi upstream tak ditemukan).
8. Session picker TUI + `HERMES_TUI_RESUME`.
9. `hermes setup agent`, `--quick` (tiket kecil, diputuskan di T05).

## Catatan prasasti

- `docs/PARITY.md` § "Known Gaps" dan baris "FTS index | Not yet" di
  "Differences" adalah keusangan pre-existing (function calling +
  FTS5 sudah mendarat di Spec 002/004) — awalnya tidak disentuh di
  T10; diperbaiki dalam review T11 bersama default sandbox dan jumlah katalog.
- `crates/hermes-cli/src/tui/main.rs` adalah file yatim (tidak
  dikompilasi; signature kedaluwarsa) — kandidat hapus di tiket hygiene.
