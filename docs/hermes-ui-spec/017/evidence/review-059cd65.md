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
Koreksi test-only dikirim untuk fmt/check/regresi/clippy/full tests sebelum
aplikasi; tidak ada perubahan runtime atau nilai expected.

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
mencakup: banner panel ... tiap step wizard, picker screen, completion
candidates, summary line.” Wizard, picker, completion, dan variasi summary
belum lengkap; tetap terbuka di T12. Ini bukan izin mengganti bukti dengan tes.

Dalam lingkup runtime yang diperiksa, perubahan sesuai §A (kolom, dim,
warna nama/punctuation) dan Q7; tidak ditemukan tambahan fitur atau
penyimpangan runtime baru pada diff ini. Tidak memperluas allowlist adaptasi.
Validasi visual berikutnya tetap dapat menemukan hal yang tidak tertangkap
pemeriksaan kode ini.

**Ringkasan:** Standards: 1 heuristik pemeliharaan (deduplikasi sedang diuji),
0 pelanggaran keras ditemukan. Spec: 2 gap bukti; yang terluas adalah cakupan
§J.7 belum lengkap. Kedua sumbu tidak digabung menjadi satu verdict kelulusan.
