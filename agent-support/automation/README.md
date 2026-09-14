# Otomatisasi checkpoint yang terbatas

Tool ini menjalankan **push setelah commit yang sudah ditinjau**. Tidak ada watcher,
auto-stage, auto-commit, merge, force-push, pergantian branch, atau pengubahan main.
Stage eksplisit + commit kecil tetap tugas agent; hook membuat push tidak terlupa.

```sh
python3 agent-support/automation/verify.py
python3 agent-support/automation/test_checkpoint.py
python3 agent-support/automation/checkpoint.py install --branch '<assigned-arena-branch>'
python3 agent-support/automation/checkpoint.py status
# Setelah commit:
python3 agent-support/automation/checkpoint.py status --require-pushed
# Retry hanya setelah diagnosis:
python3 agent-support/automation/checkpoint.py push
# Matikan hook milik tool ini:
python3 agent-support/automation/checkpoint.py disable
```

Installer hanya membuat `.git/hooks/post-commit` dan state lokal
`.git/hermes-agent-checkpoints.json`. Hook lain (termasuk commit-msg Arena) tidak
diganti. Custom hook system/post-commit yang tidak dikenal menyebabkan penolakan.
Tidak menyimpan remote URL, output network, credentials atau token dalam receipt.
Config/hook **tidak ikut clone**: sesi baru harus bootstrap eksplisit.

Branch harus sama dengan branch aktif yang ditugaskan dan berawalan `arena/`.
Push selalu `git push --no-follow-tags origin <bound-branch>`, tidak menggunakan force atau ref lain (termasuk tag).
Batas waktu120 detik. Tidak ada retry loop atau background process. Installer
memerlukan persetujuan/aturan platform yang membolehkan branch itu; awalan saja
bukan pengganti instruksi platform.

**Kegagalan post-commit tidak membatalkan commit.** Tool memberi pesan gagal,
menyimpan receipt, dan `status --require-pushed` akan nonzero. Diagnosis sebelum
retry; untuk auth, reconnect GitHub Arena. Jangan mengklaim commit sudah di remote
hanya karena `git commit` exit0. Receipt lokal terakhir tidak menggantikan CI.

Periksa secrets SEBELUM commit: hook ini menjaga tujuan push, bukan scanner yang
menjamin tidak ada secret. GitHub tetap menerima hanya commit yang dipilih agent.
Tes offline memakai repo sementara dan origin lokal; contoh branch tidak pernah
dibuat/dipindah pada repository pekerjaan.
