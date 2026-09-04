# 04: Pinned messages (/pin)

**What to build:** Pengguna bisa menandai turn penting agar **tidak pernah**
terdorong keluar oleh sliding window (ticket 02) — misalnya fakta kunci, batasan,
atau instruksi lintas-percakapan panjang.

**Blocked by:** 02 (window harus hormati kumpulan pin).

**Status:** done — commit di VM, 192/192 test hijau (`cargo test --workspace`),
`clippy --workspace --all-targets -D warnings` bersih.

## Keputusan desain (dikunci /ask-matt)

- **Pin in-memory dulu** (per-session, `ConversationRunner`), konsisten dengan
  pola Opsi 3 (Spec 008 #03). Tidak dipersist ke `state.db` (persist bisa jadi
  sub-tiket).
- **Pinned turns dihitung dalam token budget** `context_limit`. Jika pinned +
  turn terbaru sendiri sudah melebihi limit, tetap dikirim (muncul warning,
  tidak di-drop).
- **`turns_to_send` ditulis ulang** menjadi seleksi berbasis indeks (bukan
  drop-prefix): karena turn berpin bisa berada di tengah/tua, model drop-front
  kontigu lama tidak lagi memadai. Algoritma `keep_indices` selalu menyertakan
  setiap indeks berpin + turn terbaru (pertanyaan aktif), lalu mengisi dari
  terbaru ke tertua selama budget memungkinkan; output diurutkan ascending agar
  konteks tetap kronologis.

## Yang dibangun

- `ConversationRunner`:
  - field `pinned: HashSet<usize>` (indeks ke `self.turns`).
  - `pin(index)`, `unpin(index)`, `pinned() -> Vec<usize>`, `is_pinned(index)`.
  - `keep_indices()` (helper window berbasis indeks) → dipakai `turns_to_send()`
    & `dropped_turns()` (kini mengembalikan `Vec<Turn>` = komplement indeks
    terkirim, urutan asli).
  - `replace_turns` mengosongkan pin (histori diganti oleh `/new`/`/resume`
    membatalkan indeks lama — cegah pin menggantung).
- REPL: `/pin <n>`, `/unpin <n>`, `/pinned` (daftar dengan preview disanitasi),
  `/info` menampilkan jumlah pinned.

## Kriteria (per /ask-matt)

- [x] `/pin <turn_index>` menandai turn (in-memory, per-session).
- [x] `/unpin <turn_index>` menghapus pin.
- [x] `/pinned` mendaftar semua pinned turns.
- [x] Sliding window (`turns_to_send`) TIDAK pernah drop pinned turns.
- [x] Pinned turns selalu muncul di `turns_to_send` meskipun di luar window.
- [x] `state.db` tidak berubah (pin in-memory only); integrasi membuktikan full
      history (termasuk turn berpin) tetap tersimpan & terbaca via resume/list.
- [x] `/info` menampilkan jumlah pinned turns.
- [x] Test: pin turn #1 (index 0), window geser, turn #1 tetap ada di sends.
- [x] 192/192 tests green, clippy clean.

## STRIDE

- **Prompt-injection/kepercayaan:** pin memperbesar bobot turn itu; murni pilihan
  pengguna (perintah eksplisit `/pin`). Pin tidak memperkenalkan jalur I/O/eksekusi
  baru. Indeks out-of-range ditolak dengan pesan jelas, tidak panic.
- Tidak ada surface credential/eksekusi baru.

## Perubahan

- `crates/hermes-core/src/conversation/mod.rs`: field + metode pin, `keep_indices`,
  `turns_to_send`/`dropped_turns` berbasis indeks, `replace_turns` clear pin.
  +5 unit test (pin protect, pin rules, out-of-range/duplicate, budget, replacement).
- `crates/hermes-core/src/conversation/context.rs`: `turn_tokens()` (single source
  per-turn; `estimate_turns_tokens` & window memakainya).
- `crates/hermes-core/tests/conversation_session_integration.rs`: +1 integrasi
  pin survive window & tetap di state.db.
- `crates/hermes-cli/src/repl.rs`: `/pin`, `/unpin`, `/pinned`, `/info` pinned count.

## Catatan desain

- **Indeks pin** mengacu posisi 0-based di `self.turns`; karena `self.turns` tak
  pernah di-mutasi oleh window (hanya di-append), indeks stabil selama sesi —
  pin mengikuti turn, bukan posisi sesudahnya.
- Window kini memilih subset untuk dikirim; `dropped_turns()` = komplement (untuk
  display `/info`), bukan prefix — mengakomodasi pin yang tersebar.
- ADR untuk `Turn::Summary`/injeksi ringkasan tetap tertunda (dicatat Ticket 03);
  pin tak terkait langsung dengan itu.

## Dependency

02.
