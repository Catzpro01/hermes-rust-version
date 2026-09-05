# 05: Parity, docs, penutupan Spec 009

**What to build:** Uji end-to-end tugas kompleks (goal → plan → execute →
reflect → recover → done), pembaruan dokumen, dan bukti instalasi Python Hermes
tak tersentuh — mengikuti pola closure Spec 004/005/006/008.

**Blocked by:** 01–04.

**Status:** ready-for-review (implementasi selesai di VM; menunggu review
`@matt` sebelum push).

## Kriteria

- [x] E2E: tugas kompleks yang butuh >1 tool step → goal diekstrak → plan
      terbentuk → tool dieksekusi → refleksi menilai → recovery bermutasi-param
      saat gagal → `Done` (goal `Achieved`) dalam batas iterasi.
      (`planning_reflection_e2e::full_pipeline_plans_reflects_recovers_and_marks_goal_achieved`)
- [x] E2E negative: argumen gagal identik tak dieksekusi ulang (counter tool
      `["bad","good"]` — repeat `bad` di-skip); `Denied` tak pernah di-retry
      (`denied_in_planned_session_blocks_and_is_never_retried`); tak ada fake
      `User` turn (jumlah `User` turn == 1); plan/reflection tak jadi role db
      (keduanya ephemeral/in-memory, tak pernah jadi turn).
- [x] E2E: mode reaktif (tanpa `/plan on`) = Spec 002 regresi nol
      (`reactive_mode_is_zero_regression_spec002`).
- [x] Regresi: 247/247 hijau (241 + 6 baru), clippy `-D warnings` bersih.
- [x] `docs/PARITY.md` — section "Spec 009 — planning, reflection & recovery".
- [x] `docs/ROADMAP.md` — Spec 009 → Done + section closure + verification 247.
- [x] `smoke_python_hermes_untouched` tetap lulus (bagian dari suite 247).
- [x] Tak perlu ADR role baru sekarang (plan/reflection tidak dipersist; ADR
      0003/0004/0005 menetapkan representasi ephemeral).

## Catatan tambahan (Ticket 05)

- Runner: goal aktif InProgress kini ditandai `Achieved` pada penyelesaian
  normal tanpa tool **hanya saat gate refleksi on**; mode reaktif (refleksi off)
  tidak menutup goal secara otomatis (regresi nol untuk `/goal` reaktif).
- 3 unit test baru di `conversation/mod.rs` + 3 integration di
  `planning_reflection_e2e.rs`.

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
