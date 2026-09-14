# Roadmap Hermes-RS — tampilan lengkap (14 September 2026)

> **Tampilan gabungan**, disusun dari sumber kanonik: [`docs/ROADMAP.md`](ROADMAP.md)
> (fase + closure proof), [`docs/hermes-ui-spec/017/MILESTONES.md`](hermes-ui-spec/017/MILESTONES.md)
> (milestone visual yang aktif), dan
> [`agent-support/handoff/PROGRESS-ANALYSIS.md`](../agent-support/handoff/PROGRESS-ANALYSIS.md)
> (analisis + urutan kerja). Sumber kebenaran tetap ketiga berkas itu.
> Tidak ada persentase penyelesaian total atau tanggal selesai yang dicantumkan
> karena tidak bisa dipertanggungjawabkan. Tidak ada merge/penutupan tanpa
> instruksi eksplisit pengguna.

## 1. Fase besar (Spec 001–017)

| Fase | Spec | Scope | Status |
|---|---|---|---|
| 1 | 001 | CLI, config, session, streaming | ✅ Done |
| 1 | 002 | Tool calling, agentic loop | ✅ Done |
| 2 | 003 | Session/message inspection CLI | ✅ Done |
| 2 | 004 | FTS5 full-text search | ✅ Done (+ bukti redaksi kredensial) |
| 3 | 005 | Multi-provider runtime routing | ✅ Done (5 tiket, closure E2E) |
| 3 | 006 | Fallback & load balancing | ✅ Done (7 tiket, closure E2E) |
| 4 | 007 | Tool execution sandbox | ✅ Done (ADR 0006, default-on) |
| 4 | 008 | Memory & context management | ✅ Done (6 tiket, closure E2E) |
| 4 | 009 | Planning & reflection | ✅ Done (5 tiket, closure E2E) |
| 5 | 010 | Plugin/extension WASM | 🅿️ **Deferred** ke backlog v2.0 (keputusan Spec 011b) |
| 5 | 011 / 011b | MCP client + hardening | ✅ Done (5 + 4 tiket, closure E2E) |
| 5 | 012 | TUI dashboard (ratatui) | ✅ Done |
| 5 | 013 | Python UI parity (visual) | ✅ Done |
| 6 | 014 | CLI subcommands parity | ✅ Done |
| 7 | **017** | **Total parity Hermes Python v0.21.0** | 🔄 **Implementasi selesai; closure review terbuka** |

Detail closure per spec (005, 006, 007, 008, 009, 011/011b, 012, 013, 014) ada di
`docs/ROADMAP.md`; yang tersisa di fase 7 adalah **bukti + acceptance**, bukan
implementasi fitur baru.

## 2. Verifikasi terkini

- `main` HEAD `baa7158` (PR #7 **MERGED** oleh `Catzpro01`, 2026-09-14T14:28:06Z):
  kedua job CI SUCCESS; anotasi ringkasan `fmt=success clippy=success
  test=success picker=success`, **584 tes Rust lulus / 0 gagal** (28 binary tes).
- Branch kerja sesi: `arena/01a0a052-hermes-rust-version` (berbasis `baa7158`).
  Commit paket terakhir `8b89b12` → CI `34862320398` **SUCCESS**.
- CI biasa menjalankan 6 gate live PTY picker (posisi footer, warna footer,
  header normal, header filter, tata letak kolom, prompt hapus + pesan no-match);
  setiap perilaku baru menambah satu gate sebelum siklusnya ditutup.
- **Tidak ada toolchain Rust di sandbox** — setiap perubahan Rust diverifikasi di
  GitHub Actions (RED → GREEN → capture), lalu diangkut sebagai patch ber-checksum
  lewat anotasi run. Tidak ada klaim `cargo` lokal.

## 3. Spec 017 — lima area §J.7

| # | Area | Status | Catatan |
|---|---|---|---|
| 1 | Banner panel (judul + grid) | 🟡 Sebagian | Bukti berpasangan terakhir `banner-5a8e12c` (8 PNG diperiksa, kolom `Available Tools` 51/41/48/49 dan `Session` 4/17/21/21 cocok); bukan klaim pixel/raw identity seluruh variasi |
| 2 | Wizard tiap step | ⏳ V1 | 14 skenario × 2 lebar sudah di-capture; palette, geometri, hint, label, checklist tools masih berbeda |
| 3 | Session picker frame | 🔄 V2 berjalan | 8 siklus selesai; seluruh region piksel yang dipatok kini identik, sisa 4 perilaku (lihat §4) |
| 4 | Completion dropdown | ⏳ Butuh keputusan | Python (`prompt_toolkit`) menampilkan daftar alternatif; Rust melakukan cycling inline — perlu keputusan produk: bangun dropdown atau amandemen §J.7 |
| 5 | Summary line | 🟡 Sebagian | Kasus 0/3 tool sudah cocok (teks counter, posisi, gaya); variasi non-nol (skills/MCP) belum ada buktinya |

## 4. Picker V2 — daftar siklus (satu perilaku per siklus)

| # | Perilaku | Status | Bukti |
|---|---|---|---|
| 1 | Hint `d delete` hanya saat ada baris yang bisa dihapus | ✅ | `c07f0c5` |
| 2 | Counter tanpa hasil tetap `0/2 sessions` | ✅ | `a8d5e8c` |
| 3 | Footer di baris terakhir (30) | ✅ | `0c0704d` |
| 4 | Warna footer = indeks palet 8 | ✅ | `ae220ff` |
| 5 | Header bantuan mode normal = palette3 + bold | ✅ | `7fef514` |
| 6 | Header filter = palette6 + bold | ✅ | `9cc5cb4` |
| 7 | Tata letak kolom (kolom kursor, baris pemisah, medan nama, tinta header) | ✅ | `b732d22` + paket `picker-column-layout-b732d22` |
| 8 | Prompt hapus (merah + bold) + pesan no-match (atribut **dim**) | ✅ | `549fc8d` + paket `picker-message-style-fd674dc` (28/28 region piksel identik) |
| 9 | Baris terpilih ` → ` hijau + bold (mengganti reverse video) | 🔄 berikutnya | ditemukan dari perbandingan piksel paket kolom |
| 10 | Warna kolom status | ⏳ | referensi memakai `intr` (brown); belum ada bukti terpinn untuk nilai `done` di sisi Rust — jangan diwarnai tanpa bukti |
| 11 | Konten kolom `Active` / `ID` | 🅿️ adaptasi | `8`-karakter sid dan format `Active` adalah adaptasi terdokumentasi di `docs/PARITY.md` |
| 12 | Resize, daftar panjang, clear-filter | ⏳ | belum dibuktikan oleh fixture ukuran tetap ini |

Setiap siklus punya rantai bukti yang sama: **RED nyata** (gagal karena alasan
benar, bukan error setup) → **patch minimal** → **GREEN resmi** (gate live 3×,
`fmt`/`check`/`clippy -D warnings`/seluruh suite) → **penerapan patch terverifikasi
SHA-256** → **capture dari sumber ter-commit** → **audit + laporan + CI hijau**.

## 5. Wizard V1 — rincian yang belum setara

- Palette: Python header kuning, seleksi hijau, daftar memakai alternate-screen;
  Rust memakai teks pengantar + prompt inline cyan dengan ±7 opsi terlihat.
- Geometri: pada 80 kolom Python **memotong** label, Rust **membungkus**.
- Teks ESC/SPACE, pemilihan terminal, dan pembatalan belum setara.
- `wizard-tools` mencapai checklist dengan entri berbeda (Python 17/27 enabled +
  suffix `[no API key]`; Rust 26 entri statis).
- Quick setup OAuth/backend adalah **deferral eksplisit** — perbedaan tangkapan
  layar tidak boleh dipakai untuk mengesahkan implementasi backend baru.

## 6. Urutan kerja yang disetujui (P1–P3)

| Prioritas | Isi | Status sekarang |
|---|---|---|
| **P1** | Siklus TDD picker V2 satu per satu (temuan harness H1–H5 sudah menjadi gate CI) | 🔄 siklus 8 dari 12 (perilaku) |
| **P1** | Habiskan daftar V2, lalu masuk **V1 wizard** memakai 14 skenario baseline | ⏳ |
| **P2** | **Keputusan produk completion dropdown** (implementasi vs amandemen §J.7) | ⏳ butuh jawaban pengguna |
| **P2** | Bukti **summary non-nol** (skills/MCP) untuk melengkapi area kelima | ⏳ |
| **P3** | Tutup checklist **T12** + review Standards/Spec → **acceptance eksplisit pengguna** → sign-off T11 → status final T10 | 🔒 |

## 7. Syarat penutupan Spec 017

- Bukti §J.7 lengkap untuk kelima area: banner (panel + grid), tiap step wizard,
  frame session picker, dropdown completion, dan summary line.
- Semua pasangan PNG + rekaman mentah + metadata reproduksi; rekaman Python yang
  dipakai ulang harus disebutkan (bukan capture Python baru).
- Perbedaan in-scope sudah diperbaiki lalu di-capture ulang; adaptasi hanya yang
  sudah terdokumentasi di `docs/PARITY.md`.
- CI relevan **hijau** — tetapi CI hijau **bukan** penutupan.
- **Baris terakhir checklist T12: pengguna secara eksplisit menerima laporan bukti
  final.** Setelah itu baru T11 (review Standards/Spec) dan status final T10 bisa
  ditutup. Tidak ada merge tanpa instruksi pengguna.

## 8. Backlog yang tidak diklaim

Resume display §H, daftar model live, Nous OAuth, egress firewall, wiring toolset,
palette Ctrl+P (belum terverifikasi di checkout Python), picker TUI, dan Spec 010
(plugin WASM, v2.0).

## 9. Invariant yang berlaku di semua pekerjaan

- Instalasi Python pengguna tidak disentuh; `state.db` tetap kanonik; fixture
  dummy terisolasi; tidak ada rahasia yang di-commit.
- Tes otomatis **mendukung**, bukan **menggantikan**, bukti visual.
- Sanitisasi hanya di batas render; SIGINT keluar 130; tidak ada eksekusi senyap.
- Tanpa instruksi eksplisit: tidak ada merge, force-push, atau perpindahan branch.

## 10. Rujukan cepat

| Kebutuhan | Berkas |
|---|---|
| Fase + closure proof tiap spec | [`docs/ROADMAP.md`](ROADMAP.md) |
| Milestone visual picker/wizard yang aktif | [`docs/hermes-ui-spec/017/MILESTONES.md`](hermes-ui-spec/017/MILESTONES.md) |
| Analisis status + urutan kerja | [`agent-support/handoff/PROGRESS-ANALYSIS.md`](../agent-support/handoff/PROGRESS-ANALYSIS.md) |
| Paket bukti visual (immutable) | `docs/hermes-ui-spec/017/evidence/` |
| Adaptasi vs paritas murni | [`docs/PARITY.md`](PARITY.md) |
| Spesifikasi UI verbatim | [`docs/HERMES_UI_SPEC.md`](HERMES_UI_SPEC.md) |
| Tiket terbuka | `.scratch/hermes-rs-total-parity/issues/T1*.md` |
