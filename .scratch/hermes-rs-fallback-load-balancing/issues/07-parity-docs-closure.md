# 07: Parity, dokumentasi, dan penutupan Spec 006

**What to build:** Uji end-to-end lintas provider dengan fallback, pembaruan
dokumen, dan bukti instalasi Python Hermes tidak tersentuh — mengikuti pola
penutupan Spec 005 (tiket 05).

**Blocked by:** 01–06.

**Status:** todo

## Kriteria

- [ ] E2E: provider A down → otomatis fallback ke B; respon B tercatat benar di
      `state.db` untuk sesi yang sama (pola `provider_routing_e2e.rs`).
- [ ] E2E negative: credential provider A tidak pernah muncul saat B menjawab
      hasil fallback (pola `search_credential_safety.rs`).
- [ ] E2E: retry habis lalu fallback, dan provider yang tadinya down dipulihkan
      setelah cooldown (kesehatan).
- [ ] Regresi: seluruh suite hijau; jumlah test dilaporkan, bukan diasumsikan.
- [ ] `docs/PARITY.md` diperbarui dengan perilaku fallback/retry/health vs Python
      (verifikasi perilaku Python dari `~/.hermes/hermes-agent` seperti Spec 005).
- [ ] `docs/ROADMAP.md`: Spec 006 → Done, hanya setelah suite hijau.
- [ ] `smoke_python_hermes_untouched` tetap lulus — instalasi Python tidak
      dimodifikasi.

## Pelajaran yang wajib diterapkan (Spec 004/005)

- Setiap cabang (retry sukses, retry habis → fallback, fallback ke B, cooldown)
  punya test sendiri — jangan buktikan satu lalu simpulkan lainnya.
- Test negative lebih dulu (credential bocor, turn tercampur, hop tak berubah
  pada `Cancelled`).
- Ambang/batas (retry count, backoff, status transien, cooldown) dieja eksplisit
  di test agar perubahan angka mematahkan test.

## Dependency

01–06.
