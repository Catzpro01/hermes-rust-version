# Panduan meneruskan di sesi baru

## Untuk pengguna

1. Pastikan checkpoint handoff sudah tersedia di GitHub pada branch sumber
   `arena/01a09c1e-hermes-rust-version` (lihat commit terakhir di sana).
2. Selama integrasi main belum selesai, **pilih branch sumber itu sebagai basis
   sesi baru**, bukan main yang belum memuat handoff ini. Arena boleh memberikan
   branch kerja baru; agent harus mengikuti branch yang ditugaskan platform.
3. Berikan instruksi siap-pakai di bawah. Tidak perlu menyalin seluruh chat lama.
4. Jangan berikan token/password lewat chat. Bila GitHub auth gagal, reconnect
   integrasi GitHub di Arena.

### Prompt siap salin

> Lanjutkan project Hermes dari paket handoff di repo. Baca AGENTS.md,
> agent-support/RULES.md, agent-support/handoff/CURRENT.md, lalu MEMORY/PROGRESS
> kanonik dan skill yang ditunjuk. Hormati branch yang ditugaskan platform.
> Jalankan verifier paket dan periksa git status/CI; jangan reset kerja yang
> belum diverifikasi. Aktifkan auto-push checkpoint untuk branch aktif sesuai
> panduan setelah memeriksa hook yang sudah ada. Tugas substantif berikutnya
> adalah /wayfinder untuk rute penuntasan Spec017: tujuan sudah dipilih, lanjut
> wawancara breadth-first sebelum membuat peta keputusan. Jangan langsung
> implementasi header filter, jangan membuka ulang Q1–Q8 yang telah disepakati,
> dan jangan menyatakan Spec017 selesai. Simpan kemajuan kecil beserta catatan.
> Jika perlu mengintegrasikan ke main, baca MAIN-INTEGRATION.md dan batasan sesi;
> jangan melakukan merge/force-push atau mengubah branch tanpa wewenang.

## Untuk agent — bootstrap minimum

Jalankan dari repo yang diberikan platform; jangan mengandalkan path host lama.

```sh
git status --short
git branch --show-current
git log -3 --oneline
python3 agent-support/automation/verify.py
python3 agent-support/automation/test_checkpoint.py
```

Baca [CURRENT](CURRENT.md), [DECISIONS](DECISIONS.md), [VERIFICATION](VERIFICATION.md)
dan [Rules](../RULES.md). Skill discovery tetap `.agents/skills/<name>/SKILL.md`;
path kanonik fisiknya `agent-support/skills/`. Jangan menjalankan semua skill
sekaligus. Pertahankan gate skill user-invoked. Baca skill `wayfinder`, `grilling`
dan `domain-modeling`; gunakan `tdd` hanya bila kemudian benar-benar diminta/
diarahkan ke implementasi yang sesuai.

### Aktifkan auto-push lokal secara eksplisit

```sh
# Ganti placeholder dengan branch yang benar-benar ditugaskan platform.
python3 agent-support/automation/checkpoint.py install --branch '<assigned-arena-branch>'
python3 agent-support/automation/checkpoint.py status
```

Tool memeriksa bahwa branch tersebut adalah branch aktif `arena/`, bukan main,
master atau detached HEAD. Tidak membuat/mengganti branch. Konfigurasi disimpan
di `.git/` lokal, **tidak ikut clone**. Jangan mengaktifkan hooks dari vendor.
Jika sudah ada custom hook system/post-commit yang bukan milik tool ini, installer
menolak; jangan menghapus hook pengguna. Minta persetujuan untuk integrasi hook
atau gunakan commit/push manual yang eksplisit.

### Setiap checkpoint kecil yang bermakna

1. Selesaikan satu keputusan/analisis/tes/perbaikan yang bisa ditinjau.
2. Catat hasil nyata di `agent-support/memory/PROGRESS.md`; perbarui pointer memory
   dan handoff bila fokus berubah. Update milestone produk bila relevan.
3. Jalankan pemeriksaan sesuai perubahan; lihat VERIFICATION untuk Rust.
4. Tinjau diff, stage **path eksplisit**, periksa staged diff dan secrets, commit.
   Hook mendorong commit otomatis. Tidak ada auto-stage/auto-commit/watcher.
5. Verifikasi receipt; jangan menyamakan commit lokal dengan push berhasil:

```sh
python3 agent-support/automation/checkpoint.py status --require-pushed
```

Jika gagal: commit tetap lokal. Diagnosis koneksi/non-fast-forward; jangan pakai
force-push. Setelah penyebab selesai:

```sh
python3 agent-support/automation/checkpoint.py push
python3 agent-support/automation/checkpoint.py status --require-pushed
```

Receipt adalah bukti percobaan lokal terakhir, bukan pemeriksaan CI atau janji
bahwa remote tidak berubah setelahnya. Periksa head remote/Actions dengan git/gh.

## Wayfinder — tepat melanjutkan titik jeda

Tujuan telah dipilih: **rute penuntasan Spec017**. Berikutnya wawancara breadth-first
atas keputusan yang belum jelas. Teliti fakta yang bisa dibaca dari T12/T13/spec;
jangan meminta pengguna mengulang fakta yang tersedia. Jangan mengubah acceptance
atau allowlist adaptasi supaya terlihat selesai.

Tracker tetap Markdown di `.scratch/<feature>/issues/`. Sebelum charting, tambahkan
bagian **Wayfinding operations** di `agent-support/guidance/issue-tracker.md`:
identitas file, parent map, labels `wayfinder:*`, assignee sebagai claim, blocked-by,
frontier open/unblocked/unclaimed, dan resolution comment bertanggal. Ini fallback
untuk tracker yang tidak memiliki native relationships, bukan pembuatan tracker baru.

Destination/Notes harus settled sebelum map dibuat. Map hanya index keputusan
tertutup + fog; child tickets memuat pertanyaan, bukan potongan implementasi.
Charting membuat/wiring ticket, tidak menyelesaikannya. Saat bekerja pada map,
claim dahulu dan maksimal satu tiket per sesi selain pengecualian research.
HITL tidak boleh dijawab oleh agent menggantikan pengguna.

Tool Skill/subagent/research background tidak tersedia pada sesi sumber. Sesi baru
harus memeriksa kemampuannya sendiri; jangan mengarang eksekusi paralel. Instruksi
skill untuk branch prototype/research tidak mengalahkan pembatasan platform.

## Jika Git tampak mundur tetapi file terbaru ada

Jangan `reset --hard`, clean, restore, checkout paksa, atau menimpa file. Kasus
metadata-lag memang pernah terjadi. Fetch hanya ref yang diizinkan, bandingkan
setiap blob/mode/symlink terhadap source yang diketahui dan inventaris file ekstra.
Perbedaan sekecil apa pun harus disimpan/ditinjau dahulu. Metadata hanya boleh
selaras setelah bukti cocok dan tanpa mengganti branch yang ditugaskan.

Path alias bekerja untuk filesystem, **bukan sebagai direktori pada git tree**.
Untuk `git show COMMIT:path`, pakai path kanonik pada commit itu; versi sebelum
penataan memakai path lama, versi setelahnya memakai `agent-support/`.
