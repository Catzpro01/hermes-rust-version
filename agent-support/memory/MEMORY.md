# Project working memory

Repository-tracked memory for coding agents. Read alongside `AGENTS.md` and
`CONTEXT.md`; this is not the Hermes runtime memory store or a copy of the
user's private Python installation.

## Durable user instructions — confirmed 2026-09-14

- Always commit progress notes with meaningful work and push to GitHub on
  the branch assigned to the active session.
- **Confirmed again 2026-09-16 — push on every update.** Setiap pembaruan
  atau progress, sekecil apa pun, langsung di-commit dan di-push ke branch
  sesi. Jangan menahan tree yang belum ter-push, dan jangan menggabung
  beberapa progress jadi satu push besar. Push selalu diizinkan; merge tetap
  butuh perintah eksplisit. Jangan blokir giliran kerja hanya untuk menunggu
  CI selesai — push, laporkan id run beserta statusnya saat itu, dan periksa
  CI pada giliran berikutnya.
- Inherit skills and memory: read the tracked context and previous progress
  before continuing, and leave a usable handoff for the next agent.
- Fix errors found in scope and verify the fixes. Report environmental
  blockers honestly; never turn a failing check into a silent success.
- **Do not merge branches/PRs or enable auto-merge unless the user explicitly
  instructs it.** Commit/push approval is separate from merge approval.
- Preserve Python Hermes and its data. Never commit secrets.

## Skill entry points

- `.agents/skills/ask-matt/SKILL.md` and `docs/agents/matt-pocock-skills.md`

- `docs/agents/skills/progress-handoff/SKILL.md`
- `docs/agents/issue-tracker.md`
- `docs/agents/triage-labels.md`
- `docs/agents/domain.md`
- Architecture and permission decisions: `CONTEXT.md`, `docs/adr/`.

## Active handoff

### Latest checkpoint (supersedes older chronological state below)

- **W5 picker status lifecycle — TERVERIFIKASI GREEN (koreksi 2026-09-16, sesi
  `arena/01a0a7d4`).** Dua catatan lama di bawah ini ("harness delivered, gate
  deliberately RED" dan "opsi C diimplementasi namun BELUM TERVERIFIKASI")
  **sudah tidak berlaku**. Penyebab gate tidak pernah jalan bukan kode picker:
  `.github/workflows/ci.yml` mematikan job Rust berat dengan `if: false`, jadi
  "hijau" hanya berarti job workflow ringan lolos. Commit `0286bff`
  mengaktifkannya kembali — dijaga
  `scripts/test_ci_workflow.py::test_heavy_rust_job_is_not_disabled` — dan run
  [35046192977](https://github.com/Catzpro01/hermes-rust-version/actions/runs/35046192977)
  melaporkan `fmt=success clippy=success test=success picker=success`: seluruh
  suite workspace + **14** gate PTY/piksel live, termasuk `picker-status-tags`
  (bentuk lifecycle W5) dan `picker-browse-control` (resize-too-small,
  resize-redraw, long-list, clear-filter-esc, clear-filter-backspace).
  Jadi opsi C (`Stat` mengikuti `classify_session_status` atas baris pesan
  terakhir) terverifikasi **tanpa satu baris pun perubahan classifier**; bukti
  ukurannya tetap capture berpasangan `picker-status-ink-19bbbd5` (Python `intr`
  slot 3 di baris tempat Rust dulu menggambar `done` slot 2). Harness-nya seperti
  catatan lama: fixture `picker-status-tags` + checker + 10 tes, tiga cacat patch
  terkirim sudah diperbaiki; decoder suite kedua checker (24 tes) jalan lokal
  dengan `pyte==0.8.2`/`wcwidth==0.8.3`.
- **Jalur verifikasi yang SAH (dikoreksi 2026-09-16).** Daemon webhook VPS
  **berjalan** dan sudah memakai `make check`: status `a4d96bb` =
  `vps-baremetal/fast-ci` **success** "All fast checks passed via make check
  in 65s!". Cara bacanya: `gh api repos/Catzpro01/hermes-rust-version/
  commits/<sha>/status`.
  **Jangan** menyimpulkan kesehatan VPS dari `curl` ke `203.145.35.218:9000`:
  sandbox ini tidak bisa membuka koneksi TCP ke sana (reset seketika =
  pembatasan egress), jadi `curl` selalu gagal meski daemon sehat. Saya
  pernah salah menyimpulkan "VPS mati" dari itu.
- Status lama "Cargo Check failed (exit 101) (3s)" pada `4709543`/`5800741`/
  `2d36af9` adalah status **basi** dari sebelum daemon dialihkan ke
  `make check`; status commit lama tidak ditulis ulang. Bukan regresi kode.
- **Actions terpisah dari daemon webhook:** `ci.yml` memakai
  `runs-on: [self-hosted, vps, hermes]`. Era "antre lalu dibatalkan" sudah
  lewat: run 35046192977 (2m51s), 35047313528 (3m47s) dan 35048530295 (3m8s)
  semuanya **selesai** di runner itu. `concurrency.cancel-in-progress` tetap
  aktif, jadi push beruntun → hanya run terakhir yang berarti.
- **Runner self-hosted hanya mengambil job workflow `CI` (bukti 2026-09-16).**
  Job capture `ui-evidence.yml` (run `35052151094`) antre >30 menit sementara dua
  job CI di `runs-on: [self-hosted, vps, hermes]` yang **identik** selesai hijau
  dalam jendela itu (`35052151108` untuk `4f44d2f`; run `efe9ef6` 03:55→03:58).
  Jadi bukan runner offline, bukan label salah, dan bukan `if:` job (job yang
  `if`-nya false berstatus skipped, bukan queued). Kemungkinan: runner ephemeral
  diluncurkan hanya untuk event workflow CI, atau layanan runner punya filter.
  Tidak bisa dipastikan dari sandbox (API runner & `actions/permissions` = 403,
  tanpa egress TCP ke VPS). Akibat praktis: **capture empat area T12 tidak bisa
  dijalankan lewat workflow capture-nya sendiri** sampai sisi VPS diperbaiki, atau
  sampai langkah capture dipindahkan ke workflow `CI` sebagai step yang digate
  `request.json`. Jangan menyimpulkan "VPS mati" dari antrean ini.
- **Status commit VPS bisa `pending` walau Actions hijau.** Untuk `0286bff` dan
  `749d6d6` API commit-status tidak punya context sama sekali padahal run
  Actions-nya `success`. Yang otoritatif untuk kode Rust = **hasil run Actions**
  (`gh run list` / anotasi `summary` job), bukan `commits/<sha>/status`.
- **`gh run view --log` dan unduhan artefak TIDAK bisa dipakai di sandbox ini**
  (blob `productionresultssa19.blob.core.windows.net` → EOF). Jalan keluarnya:
  anotasi check-run. `gh api repos/<owner>/<repo>/check-runs/<job_id>/annotations`
  memuat ringkasan `fmt=… clippy=… test=… picker=…`, jumlah tes, **dan** patch
  rustfmt yang diekspor job (base64+gzip di anotasi berjudul
  `rustfmt patch 1/1`) — itulah cara perbaikan fmt `d41be4a` dibuat tanpa
  toolchain lokal.
- Target `make` yang tersedia: `check` (terverifikasi hijau), `fmt`,
  `clippy`, `test`, `build`. `fmt` dan `clippy` **sudah** dijalankan terhadap
  perubahan opsi C oleh job CI berat (run 35046192977, keduanya `success`).
- **Picker selected-row slice delivered**: fix `ee11541` (tested patch 1171 B
  `8eed758f…`: `Color::DarkGreen` + bold replaces reverse video), RED `34866264371`
  → GREEN `34866921566` → capture `34868306211` (bundle `fe8bc2c2…`, 10 cases) →
  CI `34867949911`/`34868819214`. Packet
  `docs/hermes-ui-spec/017/evidence/picker-selection-b4cb408/`:
  **34 of 34 pinned pixel regions identical** (two new selected-row regions per
  width), 4 byte-identical empty-case PNGs, 20 round trips, 12 checker runs, cell
  deltas only on row 4 of the three cursor scenarios (444 cells: inverse off, fg
  slot 2, bold on, palette256).
- **Three real defects found and closed in that cycle** (all in the packet's
  `attempts-result.txt`): `check_picker_message_style.dim_state` read the palette
  index in `38;5;2` as SGR dim (fix `1c40eea`); the PTY harness hit intermittent
  start-up timeouts (45 s allowance + diagnostic message, `400f5e2`/`064e92d`);
  one GREEN request failed unreproducibly and passed on an identical re-request.
- **Picker still open**: status-column ink (no pinned evidence for the Rust `done`
  value — next cycle candidate), documented `Active`/`ID` content, resize/
  long-list/clear-filter, then wizard V1 and the completion product decision.
- **Picker prompt/message slice delivered**: fix `549fc8d` (tested patch 2216 B
  `d1ed7f99…`: `Attribute::Dim` on the no-match message reset in place, and
  `Color::DarkRed` + bold on the delete-confirm prompt), RED `34864078749` →
  GREEN `34864360124` → capture `34864672852` (bundle `a8f37c46…`, 10 cases) →
  CI `34864669012`/`34864672933`. Packet
  `docs/hermes-ui-spec/017/evidence/picker-message-style-fd674dc/`:
  **28 of 28 pinned pixel regions identical** (the no-match row that differed by
  1135 px is closed), 6 byte-identical empty-case PNGs, 20 round trips, 10 checker
  runs; cell deltas are only dim (62 cells) and prompt ink (76 cells). Ordinary CI
  now also runs `test_picker_message_style.py` (sixth live gate).
- **Roadmap view**: `docs/ROADMAP-VIEW.md` (consolidated from ROADMAP/MILESTONES/
  PROGRESS-ANALYSIS; canonical files remain the source of truth).
- Picker **column-layout slice delivered**: fix `641c304` (tested patch 12824 B
  `006bdcc7…`), RED `34860668321` → GREEN `34861285022` (first proposal rejected
  by clippy, kept as `rejected-first-attempt.patch`) → capture `34861588181`
  (bundle `d6860b3d…`) → CI `34861587979`. Packet
  `docs/hermes-ui-spec/017/evidence/picker-column-layout-b732d22/`.
- **Picker still open**: cursor row (` → ` palette2 green + bold instead of
  reverse video — next cycle, gate shape already sketched), status-column ink (no
  pinned evidence for the Rust `done` value), documented `Active`/`ID`
  adaptations, resize/long-list/clear-filter, then wizard V1 and the completion
  product decision.

- Picker filter-header slice **delivered**: runtime commit `48cf587` (patch
  598B SHA `21fcdead…`, DarkCyan = palette slot6). RED `34858664487`, GREEN
  `34858865863`, capture `34859140850` (bundle SHA `a9dd5573…` @ `9cc5cb4`),
  CI `34859140646`; packet `docs/hermes-ui-spec/017/evidence/picker-filter-header-9cc5cb4/`
  with 4 pixel-identical row1 regions and the rejected bright-variant attempt
  recorded. Ordinary CI now also requires `test_picker_filter_header.py`.
  Diagnostic workflows rebound to `arena/01a0a052-hermes-rust-version`.
  Next picker slice: column header colour/indent, then selection + delete prompt.

- **PR7 MERGED ke main** (bukan oleh sesi ini): `Catzpro01`, 2026-09-14T14:28:06Z,
  merge commit `baa7158`, 1.691 file (+143.671/−954). CI `main` run34855804061
  pada `baa7158` **SUCCESS**; anotasi check-run104014924835:
  `fmt=success clippy=success test=success picker=success`, **584 tes lulus**.
  Source branch kini identik dengan main; handoff lama yang menyebut "main belum
  diperbarui" sudah usang (lihat koreksi di `agent-support/handoff/CURRENT.md`).
- Analisis progres menyeluruh ada di `agent-support/handoff/PROGRESS-ANALYSIS.md`:
  implementasi Spec001–017 mendarat; sisa = bukti §J.7 (wizard step, completion
  dropdown, variasi summary non-nol), sisa diff visual picker/wizard, keputusan
  produk dropdown, acceptance user. Verifikasi lokal baru:6 skrip QA CI PASS,
  verifier paket PASS (7/37/164/9), 8 tes auto-push OK; venv deps dari PyPI.
  Blocker lokal diukur ulang: tidak ada cargo; rust/crates/mirror/apt semua gagal
  TLS atau tidak ditemukan; hanya github.com+pypi.org jalan. Auto-push post-commit
  diinstal untuk branch sesi `arena/01a0a052-hermes-rust-version`.
- User repeats explicit PR merge request. CI34854268561/f3dfb91 GREEN. Found
  unrelated roots: main8b6a673 vs source root6ded9dd; full diff only four T10 docs,
  no independent main runtime/Cargo/scripts/workflow changes. Connected main as
  a parent on assigned source branch using reviewed ours strategy; merge tree
  exactly equals pre-operation source tree. Only continuity notes added after
  identity check. No main push/PR merge, no code/evidence/vendor replacement.
  See handoff/history-reconciliation.json and MAIN-INTEGRATION.md; verify new
  source CI/PR mergeability after commit. Wayfinder scope still Spec017.

- Handoff rechecked at95f9319 (organized by b5facfc): verifier7 aliases/37 skills/
  164 originals PASS,8 auto-push tests PASS, installed hook/receipt current.
  CI34843728495/95f9319 SUCCESS. User now explicitly requests PR merge; user
  authorization exists but session cannot write main. PR7 remains OPEN/DRAFT,
  CONFLICTING/DIRTY as checked. No merge/auto-merge attempted. See canonical
  handoff/MAIN-INTEGRATION.md and pr-status-95f9319.json. Preserve source history
  needed by evidence git-show verifiers; fresh-clone validation after integration.
  Selected Wayfinder destination is Spec017 completion; do not ask it again.

- Latest user request: persist all relevant progress/memory/skills/context for
  another session, organize agent-only files, add successor rules/guides and
  automatic checkpoint pushes; also requested main. Canonical package now lives
  in `agent-support/`; read `agent-support/handoff/CURRENT.md` and the start guide.
  Legacy discovery paths remain symlinks, not duplicated logs. Main integration
  is blocked by this session's assigned-branch restriction; do not claim main
  updated. Auto-push is opt-in after reviewed commits, not automatic file staging.
  Local hooks/config must be bootstrapped in a new clone; inspect current status.
  Package b5facfc really auto-pushed; remote SHA matched and CI34843454979 SUCCESS.
  Fresh-clone verifier and8 offline tests also PASS. Draft PR#7 is OPEN toward
  main, not merged/auto-merged; no Spec017 closure. Run receipt is tracked at
  `agent-support/handoff/verification-runs.json`.

- Active `/wayfinder`: user selected **Spec017 completion route** in the destination
  UI. Do NOT ask that destination again. Planning is paused for this handoff;
  next is breadth-first interview of unsettled decisions, then charting. No map,
  child decision ticket, claim/resolution or runtime change. Preserve settled
  Q1–Q8 and explicit final acceptance. Existing local Markdown tracker lacks
  Wayfinding operations; document conventions when charting, not a migration.
  Read official wayfinder/grilling/domain-modeling skills. Source session had
  no native Skill/subagents or permission to create/switch alternate branches.

- Normal-help-header cycle complete: runtime7fef514; capture34837424495 and
  CI34837424464 SUCCESS. RED34836863589/4aa0858 → GREEN34837102702/645f86d →
  exact1171-byte patch90fa6de6e8c260cb783260bafb4bc1502fad98ec6a332fc42c7fa5363c4dfd54.
  Packet `docs/hermes-ui-spec/017/evidence/picker-header-7fef514/REPORT.md`:10 images
  directly inspected, header4/footer color6/geometry10/counter6/hint6 PASS.
  Only row1 fg/bold changes in normal/delete;6 filter/no-match/empty PNGs identical
  toae220ff. Both indexed3+bold (Python palette16/Rust palette256),4 header rows
  pixel-identical; no normalization/new Python capture.20 raw/cast,10 PNG checks,
  34 support tests PASS. CI now requires all three live picker regressions.
  Milestones H1–H5 complete; five picker corrections verified. Filter/column header,
  selection/delete/no-match styles, other body geometry and wizard/completion/T12
  coverage remain open. Limited direct review only; no closure or merge.

- Active normal-help-header `/tdd`: agreed real CLI/PTy normal100x30 seam.
  Python target indexed3 + bold=true; pyte brown/cdcd00 aliases (NOT palette11).
  RED34836863589/4aa0858: real CLI style FAIL3, fmt/check/build PASS.
  Trace11973 bytes SHA56ecf3af0ea8fc93b369d0f23120277926f6ae2d9b6605c957a8b0cb754d5e7e.
  GREEN34837102702/645f86d exact3/fmt/check/clippy/full suite PASS.
  Tested1171-byte patch 90fa6de6e8c260cb783260bafb4bc1502fad98ec6a332fc42c7fa5363c4dfd54
  applied byte-identically; DarkYellow/Bold/Reset on n0 with empty filter.
  H3/H4 completed there; H5 subsequently completed above. Ordinary CI now
  requires live position, color and normal-header tests. Header checker4 PASS. Historical normal/
  delete4 header checks fail for Rust, Python4 PASS;14 policy tests PASS.
  `MILESTONES.md` H1 done/H2 test-first. Preserve other rows, filter header and
  all four completed footer fixes. Final ten-case capture/xterm audit required.

- Post-delivery guidance: observed CI34834196724/7b8ff84 SUCCESS, clean tree.
  Recommended next slice is normal-mode `Browse sessions` help-header styling
  through real CLI/PTy, then remaining picker presentation, then T12/wizard/
  completion coverage. Guidance only; no new test or runtime change started.

- Footer-color cycle complete: runtimeae220ff; source capture34833465322 and
  CI34833465347 SUCCESS. RED34832948842/5515789 → GREEN34833157938/f633f59 →
  exact916-byte patch ea56c272de0cf151168aee03e85803662f9ffe12bc563de97e6e512e73b941a3.
  `docs/hermes-ui-spec/017/evidence/picker-color-ae220ff/REPORT.md`:10 images
  inspected; color6/geometry10/counter6/hint6 PASS. Only row30 foreground changes;
  other rows unchanged; delete/empty4 PNGs byte-identical to0c0704d.20 raw/casts,
  10 PNGs and29 supporting checks PASS. Live color+position regressions in CI.
  Important: Python palette16 / Rust palette256, both index8/dim=false. Mode flags
  differ, preserved explicitly; six unaltered footer pixel regions are identical.
  No normalization/new Python capture. User requested visible milestones; keep
  `docs/hermes-ui-spec/017/MILESTONES.md` current (M1–M5 color complete,4 picker
  corrections verified, remaining styling/geometry/wizard/completion open).
  Limited direct review, not independent/personal approval. No closure/merge.

- Active `/tdd warna footer picker`; user also requests milestone visibility.
  See `docs/hermes-ui-spec/017/MILESTONES.md`: M1/M2 complete: RED34832948842/5515789 verified actual color FAIL3.
  Original trace85502 bytes SHAf14e3920154956dd0d68cf072166b8044ad621b751c0e8e4c6abb6884d977f97.
  GREEN34833157938/f633f59: exact CLI3/fmt/check/clippy/full suite PASS.
  Tested916-byte patch ea56c272de0cf151168aee03e85803662f9ffe12bc563de97e6e512e73b941a3
  applied byte-identically; only footer DarkGrey then foreground Reset.
  M3/M4 completed there; M5 subsequently completed as recorded above. Ordinary CI now requires both
  live position and color tests;13 policy +4 color-checker tests PASS. Agreed real CLI/PTy normal100x30
  seam. pyte palette8 aliases brightblack/7f7f7f accepted by color gate; final
  xterm audit MUST prove actual palette8 mode/index and dim=false. Do not
  confuse decoder aliases with permission to normalize evidence colors.
  Six historical Rust footer-color failures, six Python controls PASS;12 policy
  tests PASS. Preserve geometry/counter/hints and check ten-case style isolation.

- Latest `/ask-matt` routing after delivery0ed7b65: recommend `/tdd` on footer
  foreground color, one primary actual CLI/PTy normal100x30 regression. Six
  retained cases prove Python palette index8, dim=false vs Rust default fg,
  dim=false. Do NOT invent an ANSI dim-attribute requirement from "dim" prose.
  Routing only: no new live RED/test/source change. Preserve row30, counters,
  hints and delete/empty style-isolation controls; then official validation,
  fresh affected capture/direct Standards+Spec review. Other picker styling,
  wizard/completion and T12 coverage remain open. Not personal Matt approval.

- Active footer-position TDD: test-first eedd9a0; one real CLI/PTy normal100x30
  regression requires footer row30. Historical Python10/10 geometry PASS,
  historical Rust6 footer FAIL (rows4/5), delete/empty4 PASS. Live official
  RED34829070839/eedd9a0 verified three actual row5 !=30 failures (no ERROR).
  Trace24779 bytes SHAfc71b1f359b1ea7d40ee06e359b26d8a3308340db9298dfd1163b8aa2c6c43c7.
  Minimal GREEN proposal now anchors final frame line, omits its next-line move;
  GREEN34829279773/984b92f: fmt/check/build, exact CLI3, clippy/full suite PASS.
  Tested1199-byte patch e7ad21d2a7baab771e3b0a5742d5d741720302c1ed082e70448911406208fede
  applied byte-identically. Runtime0c0704d: source capture34829549105 and
  CI34829549118 SUCCESS, including required actual CLI test. New packet
  `docs/hermes-ui-spec/017/evidence/picker-position-0c0704d/REPORT.md`: ten images
  directly inspected, geometry10/10 and counters6/hints6 PASS. Verify.py checks
  20 raw/cast roundtrips,10 PNG hashes, unchanged Python provenance/settings,
  moved footer cells intact; delete only removes stale row5; empty PNGs identical.
  Rust bundle108695 bytes SHA5ab4a98ea7d1cc38b8977e76994b7a885290d04d5bf95407d0ba267fba9d900f.
  Remaining: palette/header/selection/delete styles, body geometry, wizard/
  completion discrepancies, T12 nested fields/full-CLI coverage. Resize/clear
  filter/long-list behavior not proved by this fixed-size slice. No closure/merge.
  Primary real PTY regression is now required in ordinary CI too (four-outcome
  final gate,625 combinations tested). Four supplemental checker tests retain
  historical geometry plus corrupt/missing/mismatched evidence error controls.
  Allowlisted runner now supports `picker_footer_position`; capture grows to
  ten normal/filter/no-match/delete/empty cases. A local policy negative test
  exposed Bash `! grep`/errexit bypass for mixed FAIL+ERROR; explicit exit fixes
  that, with11 policy tests. This does not count as the live product RED.


- Latest counter correction a8d5e8c: no-match footer now `0/2 sessions` for
  two original rows; delete-hint behavior preserved. User `/tdd` + continued
  the recommended public frame_lines seam. RED34826295910/f232480 (3 exact
  failures) → GREEN34826518574/bdd3cb0 (fmt/check/clippy/full suite). Exact
  2008-byte patch SHA40ce7a929dbba8376ec5abf3181d32e2ec18d6ea959f4b6bcf27b6f45c28c66f
  applied unchanged before committed-source capture34826757031 + CI34826756998
  GREEN. New `evidence/picker-counter-a8d5e8c/REPORT.md`: six images inspected,
  counters6/6 and hints6/6 PASS. Four normal/filter PNGs byte-identical to c07
  packet; only Rust cell row4 changes in the two no-match frames.
- At counter delivery, remaining scope was footer placement/palette, wizard/completion
  discrepancies and T12 field/full-CLI coverage. Hint AND no-match counter are
  now fixed; older “counter NOT fixed” entries below describe earlier state.
- `scripts/check_picker_counter.py` validates independent Python footer literals
  plus prior hint checks. At counter delivery, runner accepted two named regressions;
  counter proposals are isolated under diagnostics/picker/counter. Selection
  lives in request.json test field; now use position for its capture. Then ten workflow
  policy tests. Capture metadata includes rustc/Cargo/capture-script hashes.


- Latest runtime correction c07f0c5: picker delete hint now follows actual
  filter-empty/nonempty-selection eligibility. RED34791212317 (3 exact failures);
  first GREEN34791352664 failed full validation due a stale expected hint found
  in source (full log unavailable); corrected GREEN34791539009, exact3397-byte
  patch c866c9cb5f555527e151ab0e16efffde2a84cfc9af5790bff48e88493bef0e15 applied.
  Source capture34791652216 + CI34791652229 GREEN; six images directly inspected
  in `evidence/picker-hint-c07f0c5/REPORT.md`, original-symptom replay6/6 PASS.
  Other T12/T13 findings remain. No-match0/0 vs0/2 counter NOT fixed.
- Diagnosis tools: `scripts/check_picker_filter_hint.py` (pyte0.8.2/wcwidth0.8.3)
  replays either packet; `scripts/capture_picker_diagnostic.py` captures six states.
  Scoped workflow `.github/workflows/picker-diagnostic.yml` uses request phases
  red/green/capture and bounds proposals to session_picker.rs. 9 workflow tests.
- Cache paths may disappear between environments; install isolated deps again
  if needed. Chromium NSS/NSPR come from pinned Sparticuz al2023.tar.br, not apt.
  Git metadata may start at base while worktree preserves previous delivery: only
  align to assigned remote branch after verifying every tracked blob; never lose
  user edits or force-push. Signed artifact/log URLs still fail EOF; checksum
  annotation transport works. Correct package is hermes-rs (see evidence/ERRATA.md).


- Final four-area packet: `docs/hermes-ui-spec/017/evidence/ui-3b39bd7/REPORT.md`.
  48/48 actual paired PNGs directly inspected, including all narrow images.
  96 raw/cast checks, 48 PNG hashes, all5128 Python source hashes verified.
- Runtime checkpoint3b39bd7: capture34788156824 and CI34788156828 GREEN.
  Python1c3c9dd capture reused explicitly; same driver/pinned reference.
- Corrected exit-drain recorder via real60003-byte RED/GREEN; service launches
  now denied alongside networking. Missing gateway deps supplied only in cache.
  Preserve old ui-1e3abe7 rejected and ui-1c3c9dd intermediate packets unchanged.
- Summary example caller newline validated34787994035/e40bf5d and applied
  exactly (755 bytes, SHA639bdad8cd9c6ec12d59a34c60b89f6c45538f73f47f8bef5ed869cac3990728).
  Production UI unchanged. Final0/3-tool summaries match geometry/styles with
  branding; skills/MCP stay0. Empty picker matches, other picker/wizard/menu
  presentations differ. Do not fix production from the rejected summaries.
- `scripts/audit_ui_evidence.py` read-only audit + three behavior tests; actual
  tampered-PNG RED then digest-verification GREEN. CI runs them.
- T12/T13 remain IN PROGRESS. Wizard inventory is section-level, not every
  nested field; completion Python side is native component-host, not full CLI.
  Do not waive these boundaries or known V1/V2/V3 differences. No closure,
  independent/personal Matt approval, Spec018 or merge.


- Work: review Spec 017 closure, correct CI gates/evidence, and preserve
  progress on GitHub. No new Spec 018 implementation is authorized by this work.
- Ticket: `.scratch/hermes-rs-total-parity/issues/T11-closure-review.md`.
- Evidence and chronological updates: `PROGRESS.md`.
- Active `/grill-with-docs` interview: `.scratch/hermes-rs-total-parity/grilling.md`.
  Q1-Q8 are settled (all A): real captures + recordings + metadata, normal
  and risky-state coverage, only existing documented adaptations, unchanged
  originals with separately logged dynamic normalization, and the agreed
  100/80/94/95-column terminal matrix. Q7 confines fixes to the five UI areas
  and related regressions; Q8 requires explicit user acceptance of the final
  report for closure. User confirmed `setuju lanjutkan`: scoped execution is
  authorized; closure and merge are not. Active execution ticket is
  `.scratch/hermes-rs-total-parity/issues/T12-visual-evidence.md`.
  Official Python source is verified in an isolated cache. Four paired banner
  cases are retained under docs/hermes-ui-spec/017/evidence/banner-814c235/.
  They expose a real ANSI whitespace/column-collapse defect; none is a parity
  pass. Python side is a public display-component call, not full CLI startup.
  T12 is IN PROGRESS, not waiting for another permission-setting loop.
  Authorized branch-scoped pushes validate an unapplied welcome.rs-only
  candidate under `.scratch/hermes-rs-total-parity/runner-candidate/`.
  Official runner rustup replaces third-party wrapper actions; remaining
  GitHub-owned actions are SHA-pinned, tokens read-only, artifacts 90 days.
  Server settings and manual dispatch permissions remain unverified/403;
  these are not required for this push route. No policy bypass or merge.
- Actual correct-fixture RED: run 34782931817 on 209d04e passed fmt/check
  then observed the named regression fail. The earlier test accidentally
  reused the wrong model/tool fixture for width 100/80; that flawed baseline
  is superseded, not hidden. References/assertions remain unchanged.
  GREEN run 34783039592 on 3f3d996 passed fmt/check, named regression,
  clippy/full tests and candidate capture. Verified and applied the exact
  3469-byte Rust delta (SHA-256 5ae5e9b3430ad1a32e44cded4cd25b55cf6d6a81800edc613a6f82df4216a79e).
  It removes only the unstyled-space skip and adds the public-writer test.
  Candidate consumed/disabled (phase none). Actual source a2a3d08 has green
  CI 34783196808 and clean-source capture 34783196812. Eight paired PNGs,
  raw/cast recordings and checked metadata are in evidence/banner-a2a3d08/.
  rustc/Cargo 1.98.1 recorded. Space-collapse fixed, NOT full visual parity:
  columns 100/80 match (51/41), but 94/95 Rust=42 vs Python=48/49.
  Bold leaks onto right title border; dim and separator colors also differ.
  Next: public-writer RED/GREEN on long-session layout and style transitions,
  then recapture. See REPORT.md/comparison.json; no original was normalized.
- User requested `/wizard` → `/tdd` → `/code-review` → recapture. Wizard
  was delivered/static checked, not run interactively; no need repeat it.
  `/code-review` still needs a user-supplied fixed point and unavailable
  independent subagents; do not invent completed review. Q1-Q8 remain settled.
  Latest `/ask-matt` routing: Continue here; `/tdd` first for long-session
  94/95-column layout, then separate style slices; `/diagnosing-bugs` if the
  loop/correction resists. The agreed public seam and Q1-Q8 need no re-interview.
  No new regression/fix was executed in that routing turn. Keep focus on Hermes.
- Fresh official toolkit download verified on 2026-09-14: upstream main still
  `3cca18b368ae95cdbdebbff572ccafa662551015`, all 164 files/37 links match.
  Latest is the already-installed version, not a new release. Verification
  added to the existing lock; no vendor edits or global installer execution.
- The missing `/ask-matt` skill blocker is resolved. On the user's download
  request, all 37 official Matt Pocock skills were installed project-locally
  from pinned upstream `3cca18b368ae95cdbdebbff572ccafa662551015`.
  Read `docs/agents/matt-pocock-skills.md` and the original skill on use.
- Correction to earlier session guidance: `/ask-matt` is a skill/flow router,
  not a way to contact Matt or obtain his approval. No personal verdict has
  been received. Its recommendation does not close Spec 017 §J.7.
- All skills are available, not all automatically invoked. Respect user-only
  triggers, experimental/stub status, and the documented missing Skill-tool /
  independent-subagent capabilities. Do not fabricate completion of those steps.
- Verified source checkpoint `3e0e8d9`: CI run `34776992554` succeeded with
  fmt/clippy success, 576 Rust tests, and five CI regression tests. Formatting
  and false error annotations are fixed. Later documentation checkpoints
  must still report their own check status rather than inheriting green.
- Remaining closure work: the real §J.7 evidence set selected in Q1 and
  explicit sign-off. See T11; code CI success does not close visual parity.
  Do not invent reviewer approval or revert to the unselected evidence policy.
- Historical PR #6: 576 tests passed, clippy success; the original workflow
  masked fmt errors. This is not proof that the current commit passes.
- This session's assigned branch is `arena/01a09c1e-hermes-rust-version`.
  A successor must use its own session's assigned branch and must not switch
  to a different branch based only on this historical pointer.

## Current TDD execution — 2026-09-14

User explicitly invoked layout → separate styles → review → recapture.
All four slices have observed RED/GREEN and verified tested deltas applied:
layout 34783950104/34784076114; bold 34784219747/34784316150;
dim 34784445723/34784546009; punctuation 34784685139/34784783352.
See docs/hermes-ui-spec/017/evidence/tdd-followup.md for exact provenance.
Actual Rust changes only welcome.rs; old images remain unmodified FAIL evidence.
Actual source 3e7c89590b22881898b0f0b37e882a5d415d072b passed CI
34784967731. Run 34784967746 verified that final capture/build/exports are
skipped (not a capture success). Candidate removed and request set to phase
none, capture false to respect review-before-recapture.
Next: obtain review fixed point and user choice between explicitly limited
direct review or human-review packet; independent subagents are unavailable.
Do not invent independent review or silently skip to final capture. No merge.

## Review boundary resolved — 2026-09-14

User chose baseline 059cd65 (all ANSI fixes) and delegated best review mode.
Selected/performed direct limited Standards/Spec review, NOT independent
subagents. Report: evidence/review-059cd65.md. One duplication heuristic was
fixed via exact tested patch: review GREEN 34785303446/c2aca92, 2564 bytes,
5b8d335cdb45cf9ab24397afce10aa5f06e1a624c6287d8f2221e42ee811ba49.
Only test decoder reuse; runtime/expected values unchanged. Candidate consumed,
none/true now requests final committed-source recapture. Verify its CI, source
hash and empty Rust diff, then compare all four fresh pairs. Other UI evidence
and explicit user closure acceptance still outstanding. No independent verdict.

## 2026-09-14: Banner centering resolved; actual recapture inspected

Actual source 5a8e12c97dd760bf05f411a5d59585a97f0b5ad6 passed CI 34785921216
and capture 34785921221. New immutable evidence: banner-5a8e12c/ under
`docs/hermes-ui-spec/017/evidence/`. All eight PNGs directly inspected and all
eight raw/event/cast round trips verified; source hash and empty Rust diff
verified. Bundle 81053 bytes, SHA-256
705a6aee68b0a346e8f696a06bbe5b4f2af562220b7dca3ad4e95afbe24bbf93;
transport job 103801264070. Tools columns 51/41/48/49 and Session 4/17/21/21
match. Non-title glyph positions and same-glyph attributes match; background,
inverse/underline also checked on blank cells. Status is fixture match with
documented branding, not raw/pixel identity or overall acceptance.

b56c9a3 remained FAIL_SESSION_CENTER_94. New exact public-writer RED
34785680910 / GREEN 34785789939 resolved it without changing expectations.
Limited direct review addendum recorded. No independent approval. Candidate
consumed; none/false avoids redundant banner capture on documentation push.

Next: real paired wizard/picker/completion/summary evidence. Preliminary
setup/curses imports succeeded in isolated temporary homes, scrubbed env,
network blocked; NOT UI captures. `hermes_cli.completion` import also succeeded
but is shell script generation, NOT REPL candidate UI. Locate actual REPL seam.
Picker reference: hermes_cli/sessions_cmd.py wrapper and main.py
_session_browse_picker. Wizard: setup.py run_setup_wizard; Rust src/wizard/,
with tests/wizard_e2e.rs. Inventory every implemented step before capture.
Preserve Python data; no new features, independent verdict, closure or merge.

## 2026-09-14 — picker V2 cycle 5 delivered (status-tag ink)

Chain: `1781404` (fix, source `804b80f0…`) → `19bbbd5` (capture request) →
packet `picker-status-ink-19bbbd5`. New live gate `picker_status_ink` (8th live
gate; plan entry 8 in picker-diagnostic, 3× regression + trace retention; also
an ordinary CI gate) pins the five-cell status tag on non-cursor rows:
`done` pair1 green, `intr` pair2 yellow, `err` pair5 red, `empty` pair4 palette8,
other A_NORMAL, never bold, at `3 + name_width + 2` (43..48 @100, 25..30 @80).
The mapping came from the pinned upstream source now kept at
`docs/hermes-ui-spec/017/evidence/upstream-status-attr/` (`hermes_cli/main.py`
@63279301, blob `8281cbdd…`, sha `89cde75d…`), because §F named `_status_attr`
without it. RED 34870302745 → GREEN 34871381451 (request patch `e88347c8…`
6007 B; tested/exported patch `73d0322e…` 6109 B after the runner's rustfmt) →
capture 34871743793 (bundle `119e299b…` 204818 B; ten checkers PASS, 3× verified)
→ CI 34871741338 SUCCESS on the fixed source. Audit
`STATUS_TAG_INK_FIXED_ALL_CONTROL_REGIONS_EQUAL`: 34/34 control regions equal,
4 tag-span regions recorded as declared differences (tag word is fixture data),
cell delta only row 5 in normal/delete @both widths (20 cells: fg −1→2,
fm 0→33554432, text unchanged), python records/cells unchanged, 6 PNGs identical.

Honest failures kept: GREEN attempt 1 `34870642732` panicked (usize underflow
`n - 3` on the blank separator row, exit 101 — the panic text is only in the PTY
bytes, recoverable from the trace annotation); GREEN attempt 2 `34871081677`
passed the live gate 3× but failed a new unit test that mixed 20-column geometry
with 100-column offsets; CI `34871743697` (capture commit) hit the known PTY
start-up flake (screen fully drawn, `missing readiness/timeout at stage 0` after
207901 bytes) — the picker repaints every poll timeout, so the harness's
quiet-or-stable snapshot rule can lose the race under load; that repaint
deviation is queued as the next picker cycle. Also solved: cycle 9's
"unexplained" GREEN `34866468131` was a real live-gate failure on the committed
source, masked because `ci.yml`'s picker step is `continue-on-error: true` (step
list shows success, `steps.picker.outcome` is failure, the aggregate step fails
the job) — read the `picker terminal` annotation.

Next: picker resize/long-list/clear-filter + the repaint deviation, then the
`Active`/`ID` documented adaptations, then wizard V1 (14 scenarios / 5 groups)
and the completion-dropdown product decision. Still no whole-picker PASS, new
adaptation, acceptance, or merge.

## 2026-09-14 — picker V2 cycle 6 delivered (redraw cadence)

Chain: `bbd943c` (fix, source `e6cad4ba…`) → `b2db435` (capture request) → packet
`picker-redraw-on-input-b2db435`. The picker now repaints only when the screen
changed: a `dirty` flag starts set, every accepted key press and every `Resize`
sets it again, the 100 ms poll loop is untouched. That matches the pinned
reference (`_curses_browse` draws then blocks in `stdscr.getch()`).
New live gate `picker_redraw_on_input` (9th live gate, plan entry 9, 3× regression
+ trace retention, also an ordinary CI gate) splits each record at the scenario
keystroke writes (terminal replies excluded) and counts frames per input window:
1 before the first keystroke, 1 per typed key after (`topic` = 5, `zzzz` = 4).
RED 34873144309 → GREEN 34873480643 (**first attempt**; tested patch byte-identical
to the bound `925b963e…` 2801 B; traces 279052 → 45748 bytes) → capture 34873776470
(bundle `491713dd…` 46091 B; eleven checkers PASS; 3× verified) → CI 34873775366 and
34873776473 SUCCESS. Audit
`REDRAW_ON_INPUT_FIXED_FRAME_CONTENT_UNCHANGED_ALL_REGIONS_EQUAL`: the same gate
rejects 8/10 cases of the previous packet and accepts all 10 retained Python
records; all ten cell maps and PNGs byte-identical to the previous packet; 34/34
control regions equal; 4 declared tag-span differences at 270 px each; records
shrank wherever the loop runs (13815 → 1463 B in the no-match case).

Side effect: the PTY start-up flake stopped reproducing (the harness's
quiet-or-stable rule no longer races a child that repaints forever), so both CI
runs on the fixed source are green — this closes the flake recorded in the last
two packets. Also reused: cycle 9's "unexplained" GREEN failure was a real
live-gate failure masked by `continue-on-error: true` on the picker step.

Next: resize/long-list/clear-filter, the documented `Active`/`ID` adaptations, a
fixture that exercises the interrupted/error/empty inks live, then wizard V1 and
the completion-dropdown product decision. No whole-picker PASS, new adaptation,
acceptance, or merge.
