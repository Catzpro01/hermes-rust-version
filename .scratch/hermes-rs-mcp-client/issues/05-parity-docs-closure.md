# 05: Parity, docs, E2E closure proof

**What to build:** Uji end-to-end MCP (spawn child MCP sungguhan → handshake →
`tools/list` → tool terdaftar → agentic loop memanggilnya → hasil kembali →
shutdown bersih), pembaruan PARITY/ROADMAP, dan penutupan Spec 011 — mengikuti
pola closure Spec 004/005/006/008/009.

**Blocked by:** 01–04

**Status:** ready-for-review (implementasi selesai di VM; menunggu review
`@matt` sebelum push).

## Kriteria

- [x] E2E: helper MCP server (`mcp_test_server`, child sungguhan) di-spawn;
      handshake → `tools/list` (2 tool) → tool terdaftar `{server}__{tool}` →
      agentic loop memanggil (hasil jadi `Turn::Tool`) → shutdown kill child.
      `initialize`/`initialized` → `tools/list` (≥2 tool) → tool terdaftar di
      `ToolRegistry` dgn nama `"{server}__{tool}"` → agentic loop memanggilnya
      → hasil benar kembali → shutdown: child mati (PID berhenti) saat runner
      di-drop.
- [ ] E2E negative: server yang salah-command/handshake gagal → startup tetap
      jalan, server tsb tak terdaftar, pesan jelas; `confirm: true` → tool
      memerlukan konfirmasi, penolakan → `Denied` tak dieksekusi.
- [ ] Mode default (tanpa `mcp_servers`) → nol child spawn, nol tool MCP (zero
      regression; suite lama tetap hijau).
- [ ] Regresi: seluruh suite hijau; jumlah test dilaporkan.
- [ ] `docs/PARITY.md` — section Spec 011 (MCP client; Python padanan dicatat).
- [ ] `docs/ROADMAP.md` — Spec 011 → Done hanya setelah suite hijau.
- [ ] `docs/SECURITY.md` — catatan execution surface MCP (config trusted,
      `confirm`, redaksi env, timeout).
- [ ] `smoke_python_hermes_untouched` tetap lulus.

## Pendekatan helper E2E

Tambahan kecil: helper MCP server minimal (bisa berupa `[[bin]]` kecil di
`hermes-core` atau executable fixture) yang berbicara NDJSON JSON-RPC: menjawab
`initialize`, `tools/list` (2 tool: mis. `echo` & `fail`), dan `tools/call`
(echo mengembalikan argumen; fail mengembalikan isError). Integration test
menspawn helper tsb sebagai "command" server → proof child-process sungguhan
tanpa butuh jaringan/npx eksternal (deterministik, offline, cepat). Ini juga
dipakai test di tiket 03/04.

## Perubahan (prakiraan)

- `crates/hermes-core/src/mcp/**` (modul inti, dari tiket 01–04).
- `crates/hermes-cli/src/repl.rs` (+ registrasi MCP saat startup; `/mcp list`
  opsional utk melihat server/tool).
- helper MCP server untuk test.
- `docs/PARITY.md`, `docs/ROADMAP.md`, `docs/SECURITY.md`.

## Dependency

01–04.
