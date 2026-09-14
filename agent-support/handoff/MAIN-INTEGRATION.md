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

## Permintaan merge terbaru — belum terlaksana

Pengguna kini **secara eksplisit meminta merge pull request**. Jadi hambatannya
bukan kurangnya instruksi pengguna. Batas platform sesi sumber tetap berlaku:
agent ini tidak dapat menulis ke main atau menjalankan merge PR ke main.
Permintaan merge WIP juga **bukan acceptance atau closure Spec017**.

Pemeriksaan ulang PR pada head `95f9319c5e90daba3c70858e0920143d7febea42`:

- PR tetap **OPEN / DRAFT**, `mergeable=CONFLICTING`, `mergeStateStatus=DIRTY`.
- [CI34843728495](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34843728495)
  SUCCESS untuk head tersebut. CI GREEN tidak berarti konflik telah selesai.
- [Snapshot status PR](pr-status-95f9319.json) merekam pengamatan ini; cek ulang
  status live setelah commit lain, jangan menganggap snapshot sebagai status abadi.
- Tidak ada merge/auto-merge, push main, pergantian branch, atau resolusi konflik
  otomatis yang dilakukan pada pemeriksaan ini.

### Yang perlu dilakukan di konteks berwenang

1. Buka PR yang sudah ada; jangan membuat PR duplikat.
2. Periksa perubahan main dan selesaikan konflik dengan mempertahankan maksud
   kedua sisi. Jangan memakai pilihan seluruhnya ours/theirs tanpa review.
3. Verifikasi alias/sumber kanonik `agent-support/`, byte vendor, bukti immutable,
   dan tes CI pada head hasil resolusi. Jangan bypass proteksi branch.
4. Bila sudah layak review, ubah draft menjadi ready dan penuhi review/checks.
5. Integrasikan dengan metode yang menjaga riwayat sumber jika diizinkan repo.
   **Merge commit lebih sesuai daripada squash/rebase di sini:** verifier bukti
   memakai `git show <source-SHA>` dari siklus TDD terdahulu. Jika metode lain
   dipilih, buktikan SHA sumber tetap tersedia pada fresh clone main; jangan
   menghapus branch sumber atau mengubah bukti untuk menutupi objek yang hilang.
6. Konfirmasi API menunjukkan MERGED, lalu uji fresh clone main dan perbarui
   handoff. Sebelum itu, gunakan branch sumber sebagai basis sesi lanjutan.

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
