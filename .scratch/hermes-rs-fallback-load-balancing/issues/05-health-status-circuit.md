# 05: Status kesehatan provider & circuit breaker

**What to build:** Catat endpoint/provider yang sedang bermasalah agar tidak
dihujani permintaan berulang dalam satu sesi (circuit breaker ringan / cooldown
in-memory).

**Blocked by:** 03.

**Status:** todo

## Kondisi sekarang (terverifikasi)

Tidak ada pelacakan status. Setiap `chat` membangun permintaan dari nol; tidak
ada memori bahwa provider X baru saja gagal. `ProviderRegistry` membangun
provider secara lazy dan stateless.

## Konsep

Status kesehatan bersifat **in-memory untuk durasi proses** (invariant: `state.db`
tetap satu-satunya canonical storage; status runtime tidak dipersist). Ketika
suatu hop gagal (setelah retry habis), tandai provider itu "cooling down"
selama jeda tertentu; fallback (tiket 03) melewatkan provider yang sedang
cooldown.

## Kriteria

- [ ] Pelacak status (mis. `HealthTracker`) mencatat kegagalan hop dengan
      timestamp, disuntikkan ke wrapper fallback (testable, bukan global).
- [ ] Provider yang baru saja gagal dilewati fallback selama cooldown; setelah
      lewat, dicoba lagi.
- [ ] Cooldown berbatas & terkonfigurasi (default terdokumentasi); tidak ada
      "blacklist permanen".
- [ ] Tidak mempersist status ke `state.db` (invariant canonical storage).
- [ ] Diuji dengan wiremock: provider yang tadinya down lalu up — verifikasi
      permintaan berkurang saat cooldown dan dilanjutkan setelahnya.
- [ ] `Cancelled`/SIGINT tidak dicatat sebagai "provider rusak".
- [ ] **Manual switch `/provider B` melewati (bypass) cooldown** — pengguna
      eksplisit memintanya, jadi jangan diblokir oleh status cooldown dari
      fallback sebelumnya. Keputusan ini didokumentasikan & diuji.

## STRIDE

- **DoS:** mencegah pembanjiran berulang ke endpoint yang sedang 5xx.
- Tidak ada credential/eksekusi baru. Perhatikan penamaan provider (bukan nilai)
  jika ada log.

## Risiko

- Trade-off antara "segera retry" (tiket 02) dan "cooldown" (tiket 05). Harus
  jelas: retry adalah dalam satu hop; cooldown adalah antar-hop/jeda.
- State bersama harus di-`Send + Sync` aman dan tidak menjadi bottleneck.

## Dependency

03.
