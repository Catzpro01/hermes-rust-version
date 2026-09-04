# 05: Status kesehatan provider & circuit breaker

**What to build:** Catat endpoint/provider yang sedang bermasalah agar tidak
dihujani permintaan berulang dalam satu sesi (circuit breaker ringan / cooldown
in-memory).

**Blocked by:** 03.

**Status:** done — commit di VM, 165/165 test hijau (`cargo test --workspace`),
`clippy --workspace --all-targets -D warnings` bersih.

## Kondisi sekarang (terverifikasi)

Sebelum tiket ini: tidak ada pelacakan status. `FallbackProvider` (tiket 03)
mencoba hop berurutan tetapi setiap turn membangun dari nol — tidak ada memori
bahwa provider X baru saja gagal, sehingga endpoint 5xx bisa dihujani berulang
dalam satu sesi.

## Konsep (dikunci)

Status kesehatan **in-memory untuk durasi proses** (invariant: `state.db` tetap
satu-satunya canonical storage; status runtime tidak dipersist). Ketika suatu
hop gagal (setelah retry tiket 02 habis), `HealthTracker` menandai provider itu
"cooling down" selama cooldown berbatas; `FallbackProvider` melewati hop yang
sedang cooldown. Setelah cooldown lewat (atau hop sukses) provider kembali
berotasi. Manual `/provider B` membangun provider tunggal fresh lewat registry
(tanpa wrapper `FallbackProvider`/tracker) → **bypasses cooldown** (pilihan
eksplisit pengguna selalu dihormati).

## Kriteria

- [x] `HealthTracker` mencatat kegagalan hop per **nama** provider (bukan nilai)
      dengan timestamp; akses lewat `Mutex` → `Send + Sync`.
- [x] Provider yang baru gagal dilewati fallback selama cooldown; setelah
      cooldown elapse (atau sukses) dicoba lagi.
- [x] Cooldown **berbatas & terkonfigurasi**: injectable lewat
      `HealthTracker::new(duration)`, default `DEFAULT_COOLDOWN = 60s`
      (terdokumentasi & di-pin test). Pola sama dengan injectable `RetryPolicy`
      tiket 02 (wiring `config.yaml` ditunda, konsisten dgn review 02).
- [x] Tidak mempersist status ke `state.db` (invariant canonical storage).
- [x] Wiremock E2E: provider A down → A direkam cooling → B melayani; dalam
      cooldown A tak disentuh walau sudah "up"; setelah cooldown A dicoba lagi
      dan melayani. `Cancelled`/SIGINT **tidak** dicatat sebagai "provider
      rusak" (test: A yang cancel tak mulai cooldown).
- [x] **Manual `/provider B` melewati (bypass) cooldown** — path manual
      membangun fresh single provider tanpa tracker; test unit membuktikan
      tracker sesi lain tidak menggating pilihan manual.

## STRIDE

- **DoS:** mencegah pembanjiran berulang ke endpoint yang sedang 5xx (skip
  selama cooldown berbatas, bukan retry tak terbatas).
- Tidak ada credential/eksekusi baru. Penamaan provider (bukan nilai) — tidak
  ada string credential di tracker.
- **Cancellation** bukan kegagalan provider → tidak memicu cooldown.

## Pengujian (di atas 155 → 165, +10)

- Unit `provider/health.rs` (+5): default cooldown 60s dipin; fresh tak cooling;
  gagal → cooling sampai cooldown elapse; sukses → bersih segera; provider
  di-track independen. Waktu diuji dengan instant eksplisit (tanpa sleep).
- Unit `provider/fallback.rs` (+4): hop gagal dilewati saat cooling (counter A
  tak naik); hop cooling dicoba lagi setelah cooldown elapse & recover; hop
  `Cancelled` tidak dicatat failure (tak mulai cooldown); pilihan manual tidak
  di-gating tracker sesi lain.
- Integrasi `tests/provider_fallback_integration.rs` (wiremock, +1):
  `cooldown_skips_a_down_then_recovers_it_after_the_window` — A toggle 500→200
  via shared flag; cooldown 120ms. Turn1 A gagal→B layani + A cooling; turn2
  dalam cooldown A tak di-hit (counter A stabil) walau sudah healthy→B layani;
  sleep lewat cooldown; turn3 A dicoba lagi & melayani ("hello-from-a").

## Perubahan

- `crates/hermes-core/src/provider/health.rs` (baru): `HealthTracker`
  (+ `DEFAULT_COOLDOWN`, `new`, `record_failure`, `record_success`,
  `is_cooling_down`, `cooldown`, `Default`), `Mutex<HashMap<String, Instant>>`.
- `crates/hermes-core/src/provider/fallback.rs`: `FallbackProvider` kini memegang
  `Arc<HealthTracker>`; `with_health(...)`; `first_available` skip hop cooling,
  record failure non-`Cancelled`, record success pada sukses.
- `crates/hermes-core/src/provider/mod.rs`: `pub mod health;` + re-export
  `HealthTracker`, `DEFAULT_COOLDOWN`.
- `crates/hermes-core/tests/provider_fallback_integration.rs`: +1 E2E cooldown.

## Catatan desain

- Trade-off "retry segera" (02) vs "cooldown" (05): retry tetap berjalan di
  dalam tiap hop dulu (bounded); hanya setelah hop menyerah ia masuk cooldown —
  jadi endpoint yang sekali gagal bisa pulih cepat, yang gagal persisten tak
  dihujani.
- Cooldown default 60s injektif; bukan bottleneck (Mutex pendek, dipanggil saat
  failure/skip). Status hanya untuk durasi proses.
- Belum ada knob `config.yaml` untuk cooldown/retry hop (ditunda ke wiring lanjut
  bila diminta; default terdokumentasi & teruji).
- Manual `/provider` = `registry.select` (single, tanpa wrapper) — arsitektur
  yang membuat bypass cooldown berlaku otomatis; tidak perlu kode khusus.

## Dependency

03.
