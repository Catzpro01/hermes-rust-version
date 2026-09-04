# 04: Error recovery dengan parameter mutation

**What to build:** Saat tool gagal, minta pendekatan/parameter **berbeda** (bukan
ulang parameter sama), lacak parameter yg sudah dicoba per tool per plan step,
dan beri batas retry agar tak loop tak produktif.

**Blocked by:** 03.

**Status:** breakdown (belum implementasi).

## Kondisi sekarang (terverifikasi)

- Pada error tool, `chat_agentic` mendorong `Turn::Tool` berstatus gagal dan
  iterasi berikutnya model melihat error sbg teks. Tidak ada jejak argumen yg
  sudah dicoba; model bisa memanggil tool yg sama dgn `arguments` identik sampai
  `MaxIterations`.
- `ToolCall.arguments` & `ToolError::{Timeout,Denied,Failed}` & `ToolCallRecord`
  (tersimpan di tabel `tool_calls` bila ada `store_ctx`) tersedia utk melacak
  apa yg sudah dicoba. `ProviderError::Timeout` (Spec 006) juga tersedia utk
  klasifikasi retryable.

## Konsep

- `RetryTracker` per (tool, plan step): kumpulan fingerprint argumen yg sudah
  dicoba (hash deterministik argumen) + hitungan. Saat tool gagal dgn status
  retryable, recovery meminta model memilih parameter berbeda yg **belum**
  dicoba.
- Bila argumen identik dgn yg pernah gagal → tolak/beri sinyal ke model utk
  mutasi (jangan ulang persis). `MAX_RETRIES` per tool per step (mis. sejalan
  `RetryPolicy` Spec 006 yg bounded).
- Error non-retryable (Denied) tidak di-retry (policy: denial tak boleh
  di-bypass dgn retry); hanya sinyal utk ganti pendekatan.

## Kriteria

- [ ] Fingerprint argumen deterministik; retry dgn argumen identik ditolak
      (uji: model tak bisa ulang parameter sama).
- [ ] `MAX_RETRIES` per (tool, step) di-enforce; habis → `MaxIterations` atau
      tandai step `Blocked` (tiket 01/03).
- [ ] `Denied` tidak pernah di-retry (policy-safety). `Timeout`/`Failed`
      retryable sesuai klasifikasi.
- [ ] Tak menambah tool/shell; recovery murni re-instruksi model + tracker.
- [ ] Test unit + integration (identik ditolak, batas retry, denial tak
      di-retry); clippy bersih.
- [ ] `state.db`/`tool_calls` tak berubah semantik; recovery in-memory.

## STRIDE

- **Denial of service / abuse:** membatasi retry mencegah hammering tool mahal.
  `Denied` (keputusan manusia) tak bisa di-bypass retry — menjaga boundary
  confirmation Spec 002.
- Fingerprint berbasis argumen; tidak mengeksekusi apa pun baru.

## Risiko

- Mutasi parameter bebas justru makin mahal — batas & heuristic conservatism.
- Fingerprint false-collision jarang — gunakan hash deterministik argumen;
  uji dgn argumen serupa tapi beda.

## Dependency

03.
