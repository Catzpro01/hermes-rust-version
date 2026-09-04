# 07: Parity, dokumentasi, dan penutupan Spec 006

**What to build:** Uji end-to-end lintas provider dengan fallback, pembaruan
dokumen, dan bukti instalasi Python Hermes tidak tersentuh — mengikuti pola
penutupan Spec 005 (tiket 05).

**Blocked by:** 01–06.

**Status:** done — commit di VM, 166/166 test hijau (`cargo test --workspace`),
`clippy --workspace --all-targets -D warnings` bersih.

## Kriteria

- [x] E2E: provider A down → otomatis fallback ke B; respon B tercatat benar di
      `state.db` (tiket 03: `b_response_is_what_gets_stored_in_state_db`).
- [x] E2E negative: credential provider A tidak pernah muncul saat B menjawab
      (tiket 03: `falls_back_to_b_when_a_is_persistently_down` — assert per
      server bahwa A hanya lihat a-key, B hanya b-key; + tiket 07 config-driven).
- [x] E2E: retry habis lalu fallback, dan provider yang tadinya down dipulihkan
      setelah cooldown (tiket 05: `cooldown_skips_a_down_then_recovers_it...`).
- [x] Regresi: seluruh suite hijau; jumlah test dilaporkan (166), bukan
      diasumsikan.
- [x] `docs/PARITY.md` diperbarui dengan perilaku fallback/retry/health vs Python
      (section `Spec 006 — retry, fallback & health parity`).
- [x] `docs/ROADMAP.md`: Spec 006 → Done, hanya setelah suite hijau (section
      `Spec 006 closure` + commit per tiket + verification 166).
- [x] `smoke_python_hermes_untouched` tetap lulus — instalasi Python tidak
      dimodifikasi (tidak ada perubahan pada jalur/config Python; smoke hijau).

## Pelajaran yang wajib diterapkan (Spec 004/005)

- [x] Setiap cabang (retry sukses, retry habis → fallback, fallback ke B,
      cooldown) punya test-nya sendiri (tersebar di unit + integrasi 01–05).
- [x] Test negative lebih dulu: credential bocor lintas hop dicegah & diuji;
      `Cancelled` tak dicatat failure; turn/provider lain tak tersentuh.
- [x] Ambang/batas (retry count default 3, backoff 200ms/2s, status transien
      429/5xx, cooldown 60s, `char/4`) dieja eksplisit di test (di-pin).

## Perubahan

- `crates/hermes-core/tests/provider_fallback_integration.rs`: +1 test
  config-driven `config_driven_fallback_chain_serves_via_b_and_isolates_keys`
  (parse `HermesConfig` dgn `model.fallback_chain`, resolve lewat
  `select_with_fallback`, A down → B jawab via wire, isolasi key dua server).
- `docs/PARITY.md`: section `Spec 006 — retry, fallback & health parity` (+
  rapikan heading "Differences" duplikat).
- `docs/ROADMAP.md`: Spec 006 → Done; section `Spec 006 closure`; verification
  166/166.

## Dependency

01–06.
