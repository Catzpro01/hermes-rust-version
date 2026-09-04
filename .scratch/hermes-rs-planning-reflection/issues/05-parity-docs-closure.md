# 05: Parity, docs, penutupan Spec 009

**What to build:** Uji end-to-end tugas kompleks (goal → plan → execute →
reflect → recover → done), pembaruan dokumen, dan bukti instalasi Python Hermes
tak tersentuh — mengikuti pola closure Spec 004/005/006/008.

**Blocked by:** 01–04.

**Status:** breakdown (belum implementasi).

## Kriteria

- [ ] E2E: tugas kompleks yang butuh >1 tool step → goal diekstrak → plan
      terbentuk → tool dieksekusi → refleksi menilai → recovery bermutasi-param
      saat gagal → `Done` (goal `Achieved`) dalam batas iterasi.
- [ ] E2E negative: argumen tool gagal yg diulang identik **tidak** terjadi
      (RetryTracker menolak); `Denied` tak pernah di-retry; tak ada fake `User`
      turn; plan/reflection tak jadi role db baru.
- [ ] E2E: mode reaktif (tanpa `/plan on`) berperilaku seperti Spec 002 (regresi
      nol).
- [ ] Regresi: seluruh suite hijau; jumlah test dilaporkan, bukan diasumsikan.
- [ ] `docs/PARITY.md` diperbarui dgn perilaku planning/reflection (Rust-side;
      Python tak punya padanan → dicatat sbg perbedaan).
- [ ] `docs/ROADMAP.md`: Spec 009 → Done, hanya setelah suite hijau.
- [ ] `smoke_python_hermes_untouched` tetap lulus.
- [ ] (Jika plan/reflection akan dipersist di masa depan) ADR analog 0003
      ditulis sebelum role baru masuk db.

## Pelajaran yang wajib diterapkan (Spec 004/005/006/008)

- Setiap cabang (mode on/off, goal tercapai/tidak, retry vs denial, identik
  ditolak) punya test-nya sendiri — jangan buktikan satu lalu simpulkan yg lain.
- Test negative lebih dulu: apa yang **tidak** boleh terjadi (fake User turn,
  retry identik, denial di-bypass, role db baru tanpa ADR, plan melebihi budget
  di-drop tanpa warning).
- Ambang/batas (iterasi ≤ 10, `MAX_RETRIES`, `MAX_REFLECTIONS`, token budget)
  dieja eksplisit di test.

## Perubahan (prakiraan)

- `crates/hermes-core/tests/planning_reflection_e2e.rs` atau setara (baru).
- `crates/hermes-cli/src/repl.rs` (+ `/goal`, `/plan`, `/info` bila perlu).
- `docs/PARITY.md`: section Spec 009. `docs/ROADMAP.md`: Spec 009 → Done.

## Dependency

01–04.
