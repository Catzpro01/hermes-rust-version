# 06: Parity, docs, penutupan Spec 008

**What to build:** Uji end-to-end percakapan panjang yang otomatis terkompresi,
pembaruan dokumen, dan bukti instalasi Python Hermes tidak tersentuh — mengikuti
pola closure Spec 005/006.

**Blocked by:** 01–05.

**Status:** done — Spec 008 closure commit di VM, suite hijau, clippy clean.

## Kriteria

- [x] E2E: percakapan panjang (melebihi batas) → window aktif → request
      berikutnya `estimate_turns_tokens <= limit`; ringkasan/inti hadir.
- [x] E2E: state.db tetap menyimpan seluruh turn asli (window tak menghapus
      canonical), termasuk turn yang di-drop dari kiriman.
- [x] E2E negative: turn yang di-pin tidak pernah hilang dari kiriman; ringkasan
      tidak disuntik sebagai pesan User palsu.
- [x] Regresi: seluruh suite hijau; jumlah test dilaporkan, bukan diasumsikan.
- [x] `docs/PARITY.md` diperbarui dengan perilaku memory/context (sliding
      window, summarization, pin) vs Python.
- [x] `docs/ROADMAP.md`: Spec 008 → Done, hanya setelah suite hijau.
- [x] `smoke_python_hermes_untouched` tetap lulus.

## Pelajaran yang wajib diterapkan (Spec 004/005/006)

- Setiap cabang (window on/off, pin, summarization heuristik, precedence batas)
  punya test-nya sendiri — jangan buktikan satu lalu simpulkan yang lain.
- Test negative lebih dulu: apa yang **tidak** boleh terjadi (turn canonical
  terhapus, ringkasan jadi User palsu, turn berpin hilang, credential bocor).
- Ambang/batas (limit token, jumlah turn dipertahankan, cap ringkasan, jumlah
  pin) dieja eksplisit di test.

## Perubahan (prakiraan)

- `crates/hermes-cli/tests/context_compression_e2e.rs` atau setara (baru).
- `docs/PARITY.md`: section Spec 008.
- `docs/ROADMAP.md`: Spec 008 → Done + closure.

## Dependency

01–05.
