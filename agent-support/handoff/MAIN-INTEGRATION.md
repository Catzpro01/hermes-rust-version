# Menyimpan ke main — status dan batasan

Pengguna meminta semua progres/ingatan/skill/konteks ditata dan disimpan ke main.
**Main tidak dapat ditulis dari sesi sumber ini:** platform hanya mengizinkan
commit/push ke `arena/01a09c1e-hermes-rust-version`. Tidak ada perpindahan branch,
force-push, merge, atau auto-merge yang dilakukan oleh paket auto-push.

## Draft PR sudah tersedia

[PR #7 — Preserve Hermes progress and organized agent handoff (Spec017 WIP)](https://github.com/Catzpro01/hermes-rust-version/pull/7)
berstatus **OPEN / DRAFT**, dari branch sumber ke main. Tidak ada merge/auto-merge.
Checkpoint paket `b5facfc` sudah ter-push otomatis dan CI34843454979 SUCCESS.
Cek ulang CI head terbaru setelah checkpoint catatan berikutnya; PR mengikuti
push branch sumber. **Main belum diperbarui.**

## Jalur integrasi yang aman

- Seluruh perubahan dipertahankan pada branch sumber; gunakan branch tersebut
  sebagai basis sesi baru sampai integrasi main benar-benar selesai.
- Draft PR dari branch sumber ke main dapat menjadi wadah review. Draft PR
  **bukan** berarti isi main sudah berubah atau Spec017 telah diterima.
- Tinjau diff kumulatif branch: PR mencakup progres UI yang telah berjalan,
  bukti visual, skill vendored, serta penataan agent-support—bukan hanya dokumen ini.
- Cek CI commit PR terbaru. Jangan bypass branch protection/review requirements.
- Merge hanya melalui konteks yang memiliki izin platform/repo dan instruksi
  pengguna yang berlaku. Agent berikutnya tetap harus memeriksa batasannya sendiri.
- Permintaan integrasi WIP tidak otomatis menutup T12/T13/Spec017. Persetujuan akhir
  visual tetap terpisah. Jika menggabungkan checkpoint WIP, status open harus tetap
  jelas di main; jangan mengubahnya menjadi overall PASS.

Tidak ada janji bahwa PR/merge berhasil sebelum API/CI mengonfirmasinya. Link/status
PR yang benar dilaporkan saat tersedia; cari PR existing terlebih dahulu agar tidak
membuat duplikat. Auto-push helper sengaja menolak main/master/detached HEAD.
