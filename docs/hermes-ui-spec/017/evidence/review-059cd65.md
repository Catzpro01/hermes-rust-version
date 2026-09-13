# Review langsung terbatas — seluruh perbaikan ANSI

Tanggal: 2026-09-14. User memilih baseline `059cd65` dan mendelegasikan
pemilihan metode terbaik. Agent memilih pemeriksaan langsung untuk melanjutkan
pekerjaan, **bukan dua reviewer independen**. Tidak ada subagent/personal Matt
verdict atau persetujuan closure/merge.

Checkpoint yang diperiksa: `e0a52fd1cf87ce15117b85e1d00f14d85edb2558`.
Baseline: `059cd6513ee6abeeec0cc61e478dcb38896634d2`.
Keduanya resolve, diff tidak kosong (48 path; termasuk artefak asli).

```bash
git diff 059cd6513ee6abeeec0cc61e478dcb38896634d2...e0a52fd1cf87ce15117b85e1d00f14d85edb2558
git log --oneline 059cd6513ee6abeeec0cc61e478dcb38896634d2..e0a52fd1cf87ce15117b85e1d00f14d85edb2558
```

Riwayat yang diperiksa: e92cc63, 1f0db4f, 209d04e, 3f3d996, a2a3d08,
905b5e9, 7f7854d, cc458e9, 33209fd, 9aef403, 3dffb6c, b4e0ee7,
871faaf, 98793df, 7af74af, 3e7c895, e0a52fd.

Sumber Standards: AGENTS.md, CONTEXT.md, progress-handoff/SKILL.md,
ADR 0001/0002, serta seluruh smell baseline pada skill code-review.
Sumber Spec: docs/HERMES_UI_SPEC.md §A/§J.7, T12, grilling.md Q1–Q8,
dan adaptasi yang sudah dicatat di docs/PARITY.md pada b38d09e.

## Standards

**S1 — possible Duplicated Code (heuristik, bukan pelanggaran keras).**
Dua tes geometry di welcome.rs mengulang blok `let mut escape = false;`
sampai `assert!(!escape, "unterminated SGR")`, sementara tes style sudah
memiliki decoder SGR publik. Risiko pemeliharaan: decoder bisa berbeda
perilakunya. Sesuai smell baseline “the same logic shape appears in more
than one hunk”, usulkan memakai `observed_banner_cells` untuk keduanya.
Koreksi test-only memakai decoder yang sama; GREEN 34785303446 pada c2aca92
lulus fmt/check, regresi dan clippy/full tests. Delta 2564 byte, SHA-256
`5b8d335cdb45cf9ab24397afce10aa5f06e1a624c6287d8f2221e42ee811ba49`,
diverifikasi/diperiksa dan diterapkan persis sebelum commit. Tidak ada
perubahan runtime atau nilai expected. S1 terselesaikan.

Tidak ditemukan pelanggaran keras tambahan dari pemeriksaan langsung:
perubahan runtime terbatas pada banner; shared theme/Python tidak diubah;
seam publik dan referensi independen dipakai; delta diuji sebelum commit;
workflow tetap read-only/SHA-pinned. Cek format/lint tidak dihitung ulang
sebagai temuan review karena sudah ditangani tooling.

## Spec

**P1 — bukti final banner belum diperbarui pada checkpoint review.**
Q7: “Fix deviations ... regenerate affected evidence.” Screenshot a2a3d08
masih menunjukkan kegagalan lama, bukan hasil akhir koreksi. TDD saja belum
menutup persyaratan ini. Tindak lanjut: setelah koreksi review diverifikasi,
ambil capture committed-source yang baru; jangan menimpa atau menyatakan
PASS untuk gambar lama.

**P2 — cakupan §J.7 masih parsial.** Spec: “side-by-side capture wajib
mencakup: banner panel (title + grid), setup wizard tiap step (curses frame),
session picker frame, completion dropdown (string candidates), summary line.” Wizard, picker, completion, dan variasi summary
belum lengkap; tetap terbuka di T12. Ini bukan izin mengganti bukti dengan tes.

Dalam lingkup runtime yang diperiksa, perubahan sesuai §A (kolom, dim,
warna nama/punctuation) dan Q7; tidak ditemukan tambahan fitur atau
penyimpangan runtime baru pada diff ini. Tidak memperluas allowlist adaptasi.
Validasi visual berikutnya tetap dapat menemukan hal yang tidak tertangkap
pemeriksaan kode ini.

**Ringkasan:** Standards: 1 heuristik pemeliharaan (deduplikasi terverifikasi),
0 pelanggaran keras ditemukan. Spec: 2 gap bukti; yang terluas adalah cakupan
§J.7 belum lengkap. Kedua sumbu tidak digabung menjadi satu verdict kelulusan.

## Addendum setelah recapture b56c9a3

Pemeriksaan langsung menemukan sisa label Session bergeser satu kolom pada
94×30; bukti b56c9a3 disimpan FAIL, tidak dinormalisasi. Regresi baru public
writer: RED 34785680910/5a2207d, GREEN 34785789939/97ce7b4. Seluruh referensi
lama, fmt/check, clippy dan tes workspace tetap lulus.

Delta terbatas yang diperiksa: lebar teks terlihat (tanpa separator whitespace
akhir hasil wrap) dipakai untuk offset centering; teks/bukti tidak diubah.
Tes memakai decoder bersama dan fixture Python nyata. Tidak ditemukan tambahan
pelanggaran keras Standards atau scope creep Spec pada delta ini. Patch 3644
byte SHA-256 d306f7bb2d8861e1c1a50dc6cc1e2f2871f5b4a9c11539058e4c6119652095c8
terverifikasi dan diterapkan persis. Recapture aktual berikutnya tetap wajib.
Ini masih pemeriksaan langsung terbatas, bukan reviewer independen.
