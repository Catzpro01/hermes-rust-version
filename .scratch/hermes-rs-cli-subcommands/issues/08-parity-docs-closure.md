# 014-08: Parity, docs & closure
**Status:** DONE — menunggu review Matt.

## Cakupan (Ticket 08)
- `placeholder()` (T01) dan pesan `coming soon: … (Spec 014)` dihapus —
  seluruh varian `Commands` sekarang nyata; `name()` dipertahankan (pinned
  untuk 12 varian termasuk `setup`/`version`).
- Dokumen disinkronkan dengan kode: `docs/cli_subcommands.md` (tabel status,
  tests), `docs/ROADMAP.md` (Fase 6 → Done + bagian "Spec 014 closure"),
  `docs/PARITY.md` (baris CLI surface + tabel subcommand),
  `specs/SPEC-014-CLI-SUBCOMMANDS.md` (status DONE), README board ini.
- `Cargo.lock.bak` (duplikat byte-identik `Cargo.lock`) dihapus dari repo.

## Closure proof
`crates/hermes-cli/tests/subcommands_e2e.rs` (26 test) menjalankan binary
nyata: setiap subcommand exit 0 tanpa prompt REPL dan tanpa ESC saat
piped; `sessions`/`inspect`/`messages`/`tool-calls`/`search`/`info` (baris
1) dibandingkan **terhadap output REPL hidup pada store yang sama**, bukan
salinan format; kredensial (`API_KEY=…`, `sk-proj-…` di `env` MCP) tidak
pernah bocor; subcommand read-only tidak membuat `state.db` dan snapshot
baris kanonik tidak berubah; invocation kosong tetap masuk REPL (zero
regression); `--version` bekerja tanpa home / dengan config rusak.

## Verifikasi
CI GitHub Actions (`.github/workflows/ci.yml`) hijau pada 2026-09-13: 509 test lulus, clippy `-D warnings` bersih.
