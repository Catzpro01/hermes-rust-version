# 04: Error recovery dengan parameter mutation

**What to build:** Saat tool gagal, minta pendekatan/parameter **berbeda** (bukan
ulang parameter sama), lacak parameter yg sudah dicoba per tool per plan step,
dan beri batas retry agar tak loop tak produktif.

**Blocked by:** 03.

**Status:** done (approve `@matt`). Commit `9d514ef`; 241/241 hijau, clippy
bersih. Push ke GitHub tertunda — butuh one-shot token (origin saat ini tanpa
token).

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

**Keputusan desain (pilihan user = Opsi A / Option 1):** hasil tool gagal
di-annotasi dgn set argumen yg sudah dicoba (`[already tried: ...]`) sehingga
model memilih parameter berbeda. Tidak ada kanal instruksi baru; hanya tracker
in-memory + re-instruksi lewat hasil tool. Variant `AgenticResult::Blocked`
BARU (tidak reuse `MaxIterations`).

- [x] Fingerprint argumen deterministik (FNV-1a ter-canonical-kan dgn trim;
      deviasi hash dipin dalam doc module); retry argumen identik ditolak di
      loop tool sebelum eksekusi.
- [x] `MAX_RETRIES` per tool = 3 (const `recovery::MAX_RETRIES`); habis →
      goal `Blocked` (verdict Ticket 03) → early-stop `AgenticResult::Blocked`.
- [x] `Denied` tidak pernah di-record/retry (spec-invariant 002); hanya
      `Error`/`Timeout` yg retryable dicatat.
- [x] Tak menambah tool/shell; recovery = `RetryTracker` in-memory + note.
- [x] Test unit + integration; clippy bersih. 8 test baru: 4 unit
      `recovery.rs` (determinisme, repeat identik terdeteksi, batas retry,
      note), 2 unit runner di `mod.rs`, 2 integration di `agentic_loop_tests`
      (retryable bounded + repeat tak dieksekusi; denial tak di-retry).
- [x] `state.db`/`tool_calls` tak berubah semantik; recovery murni in-memory.

## Catatan implementasi (mirror, sblm push)

- Module baru `crates/hermes-core/src/conversation/recovery.rs`.
- `conversation/mod.rs`: field `recovery: RetryTracker` (in-memory), aksesor
  `recovery_enabled` (= `reflection_enabled`; off default → reactive zero
  regression), `reset_recovery` (dipanggil di awal task & `replace_turns`).
- Di loop tool: (1) setelah status tool, jika retryable `Error`/`Timeout` →
  `recovery.record`; jika tak `can_retry` → goal `Blocked`. (2) Sebelum
  eksekusi: jika argumen identik sdh dicoba (`is_attempted`) → tolak, kirim
  note `duplicate... already tried`, `continue`; tak ulang persis.
- Early-stop: di atas loop, jika `recovery_enabled` && goal `Blocked` →
  `Ok(AgenticResult::Blocked{reason})`.
- REPL: match arm baru utk `AgenticResult::Blocked`.
- Interaksi dgn Ticket 03 anti-loop (`MAX_REFLECTIONS`) masih berlaku —
  recovery & reflection sama-sama bisa tandai Blocked; keduanya off default.

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
