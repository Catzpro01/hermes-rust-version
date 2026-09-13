# 013-07: Race — event display hilang saat turn selesai tanpa yield

**Status:** IN REVIEW (CI-verified) — menunggu review Matt (PR terpisah,
diekstrak dari branch T08 sesuai verdict /ask-matt 2026-09-13).

Follow-up dari **013-04** (streaming box & spinner).

## Bug

Loop turn di `crates/hermes-cli/src/repl.rs` mem-drain event display dari
channel `disp_rx` **sebelum** tiap `tokio::select!`, lalu `break` begitu
turn future selesai. Masalahnya:

- `select!` mencantumkan branch `res = &mut turn` **pertama** — kalau saat
  dip-poll turn sudah selesai DAN channel masih punya event, branch turn
  menang dan event yang masih numpuk di `disp_rx` tidak pernah dibaca.
- Turn future bisa sampai selesai dalam satu poll tanpa yield ke executor
  (provider offline/fake, tool exec sinkron, stream `stream::iter` — semua
  `Ready` langsung). Saat itu event `ToolStarted`, `Chunk`, `Done` yang
  dikirim tepat sebelum selesai menumpuk di channel dan **hilang**.

Gejala (terbukti di CI): e2e `streaming_e2e::tool_activity_is_plain_when_piped`
flaky — baris `  [tool] read_file` hilang dari stdout piped, dan karena
`renderer.any_text()` false, `emit_final` tetap merender box jawaban
( jadi output "tampak normal" kecuali baris `[tool]` yang hilang).
Probabilitas flake bergantung pada scheduling runner (load rendah = lebih
mungkin semua poll `Ready` tanpa yield).

## Fix

Final drain `try_recv` setelah loop `select!` selesai (turn complete),
sebelum handle `result`:

```rust
while let Ok(ev) = disp_rx.try_recv() {
    let _ = crate::streaming::apply_event(
        &mut renderer, &mut spinner, &mut std::io::stdout(), &ev,
    );
}
```

- Event diproses urut tiba; `Done` (close box) tetap terakhir.
- Sisi efek bonus: `any_text()` kini benar sebelum `emit_final`, jadi
  tidak ada lagi risiko double-render box saat Chunk sempat ter-drain
  tapi event `Done` hilang.
- Branch `sigint` tetap discarding partial turn (sengaja, invariant Spec
  013 #2: exit 130).

## Bukti

- Reproduksi: PR run pertama branch T08 (`34766120550`) gagal di
  `tool_activity_is_plain_when_piped` padahal tree yang sama hijau 6×
  di run push — determinisme pipeline fake provider memastikan ini race
  scheduling, bukan bug logika streaming.
- Setelah fix: PR run `34766375422` hijau (fmt + clippy + test, 214 unit
  + seluruh e2e).
- Catatan: fix ini tidak menghilangkan akar "semua poll Ready dalam satu
  langkah"; ia menjamin event yang sudah dikirim tetap ter-render.
