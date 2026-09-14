# Verifikasi dan reproduksi

## Paket agent (stdlib Python, tanpa network)

```sh
python3 agent-support/automation/verify.py
python3 agent-support/automation/test_checkpoint.py
```

Verifier memeriksa alias,37 tautan skill,164 file upstream beserta Git blob/size/
mode asli, dan link panduan. Tes auto-push menggunakan repo temporer + bare origin
lokal; tidak membuat commit/branch di repo proyek dan tidak menulis ke GitHub.

Sumber skill terkunci pada3cca18b368ae95cdbdebbff572ccafa662551015. Lock ada di
`agent-support/guidance/vendor/mattpocock-skills.lock.json`. Pemindahan folder tidak
mengubah byte upstream atau lisensinya. Jangan menjalankan installer/hook vendor.

## Regresi proyek pendukung

Bila dependency cache hilang, install versi yang sama di tempat terisolasi:

```sh
python3 -m pip install --target /tmp/hermes-qa pyte==0.8.2 wcwidth==0.8.3 PyYAML==6.0.3
export PYTHONPATH=/tmp/hermes-qa
python3 scripts/test_ci_workflow.py
python3 scripts/test_picker_normal_header_checks.py
python3 scripts/test_picker_footer_color_checks.py
python3 scripts/test_picker_footer_position_checks.py
python3 scripts/test_capture_ui.py
python3 scripts/test_audit_ui_evidence.py
```

Snapshot header source7fef514 telah lolos34 checks pendukung saat delivery. Jangan
menganggap angka/hasil historis sebagai hasil baru; simpan output tes terbaru.

## Bukti UI immutable

Baca [laporan header](../../docs/hermes-ui-spec/017/evidence/picker-header-7fef514/REPORT.md)
dan script verifikasinya. Raw/casts/PNGs, hash, reference reuse dan run receipts
tersimpan dalam paket. `python3 .../verify.py` memakai stdlib dan git source blobs.
`sha256sum -c SHA256SUMS` dijalankan dari direktori paket. Jangan menimpa originals.

Renderer: pinned `scripts/visual-renderer/package-lock.json`, xterm5.5.0,
Chromium138.0.7204.0, DejaVu Mono5.2.5. Dependency cache/node_modules dan NSS/NSPR
bukan deliverable Git. Bila perlu render baru, install dependency terkunci dan
pakai **direktori baru**; jangan mengklaim PNG AI sebagai screenshot terminal.
Pixel checker memakai browser yang sama, tanpa normalisasi warna/geometri.

## Rust / Actions

Tidak ada cargo lokal pada sesi sumber. Pola yang sudah terbukti:

1. Regresi pada seam publik disepakati → actual RED resmi, bukan setup/zero-test.
2. Proposal minimum dibatasi path/hash → official fmt/check/clippy/full suite.
3. Ambil exact tested patch, verifikasi digest/kelengkapan/path, `git apply --check`.
4. Terapkan byte-identik → commit → capture source committed → review gambar/CI.

Jangan commit Rust yang belum diverifikasi hanya untuk mengatasi ketiadaan cargo.
Paket yang benar adalah **hermes-rs**, bukan hermes-cli. Workflow diagnostic saat
ini dipin ke branch sesi sumber dan dipicu request.json. Sesi baru dengan branch
berbeda harus menyesuaikan trigger/gate secara teruji di branch yang diizinkan;
jangan mengubah guard read-only/SHA/retention atau kembali ke branch lama tanpa hak.

Actions artifacts/log signed URLs pernah gagal. Gunakan artifact jika tersedia;
bila tidak, gunakan checksum-verified annotations. Jangan mencetak URL bertoken.
Lihat [progress-handoff](../guidance/skills/progress-handoff/SKILL.md), workflow dan
laporan bukti. Cache helper di sesi lama mungkin tidak tersedia lagi; algoritme
transport dan digest ada di sumber/script/laporan, bukan bergantung pada chat.

CI source7fef514:34837424464, capture34837424495, keduanya SUCCESS. Laporan memiliki
link lengkap dan receipts. CI akhir penataan harus dicek terpisah melalui `gh`.

## Hasil baru penataan — 2026-09-14

- Verifier paket PASS:7 alias,37 skill links,164 original vendor blobs/modes,
 9 panduan/link yang diperiksa. Symlink upstream `AGENTS.md -> CLAUDE.md` dihash
  sebagai link Git, bukan isi target; tidak ada byte upstream yang dikoreksi.
-42 unit tests PASS:8 auto-push offline +34 pendukung proyek (14 workflow,
 4 header,4 color,4 position,5 recorder,3 auditor).
- Perbandingan terhadap checkpoint4785700:1788 objek yang harus tetap sama
  (termasuk file yang hanya dipindah), blob+mode semuanya identik;0 mismatch.
  Entrypoint/log/skill-link/CI yang sengaja diubah dikecualikan secara eksplisit.
- Verifier provenance/numeric header dan seluruh `SHA256SUMS` paket PASS.
  Tidak ada render/capture/Python source audit baru; data/pixel checks historis
  tetap tersimpan, tidak dimodifikasi. Runtime Rust tetap7fef514.
- Hook lokal auto-push diinstal pada branch sumber. Existing commit-msg Arena
  SHA256 `1a3b9ea63392175d75878b3a6a65a9448ca32af439c644a7cce4bb5cff071b0e`
  tetap sama. Pembuktian push nyata/CI dilakukan setelah commit checkpoint;
  hasil aktual berikutnya dicatat di PROGRESS, bukan diasumsikan dari tes offline.

### Pembuktian remote dan runner resmi

Checkpoint `b5facfcd6aeac43bb5d15695f2c060aab8a59e63` otomatis ter-push dari
post-commit; `status --require-pushed` PASS dan `git ls-remote` cocok persis.
[CI34843454979](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34843454979)
SUCCESS: package/automation, QA gates, fmt, clippy, full Rust suite dan tiga
actual picker regressions. [Run/job/step receipt](verification-runs.json) disimpan.
Warnings nonblocking: pinned actions masih mendeklarasikan Node20, runner memaksa
Node24; tidak mengubah pin/allowlist tanpa task terpisah.

Fresh clone Git dari commit itu: verifier7/37/164/9 PASS dan8 automation tests PASS.
Hook/state tidak ikut clone, sesuai desain. CI lama bf0fd81/34838109837 dan
4785700/34839170321 juga dikonfirmasi SUCCESS, bukan lagi status unknown.
Catatan hasil ini ada pada commit lanjutan; cek Actions untuk head terbaru.
