# Pemulihan akses dispatch GitHub Actions

Status 2026-09-14: **BLOCKED — otorisasi koneksi GitHub perlu tindakan user/pemilik integrasi.**

## Yang diperiksa agent

- Pembacaan workflow/run bekerja. CI `34779850516` pada `3a644c7` berhasil
  di kedua job. Ini tidak membuktikan izin memulai run manual.
- Pembacaan pengaturan `GET /repos/Catzpro01/hermes-rust-version/actions/permissions`
  ditolak HTTP 403 `Resource not accessible by integration`.
- Percobaan pemulihan melalui dispatch capture tanpa kandidat juga ditolak:

  ```bash
  gh workflow run visual-evidence.yml \
    --ref arena/01a09c1e-hermes-rust-version \
    -f regression_phase=none
  ```

  Hasil: HTTP 403 `Resource not accessible by integration`. Tidak ada run
  baru atau hasil RED/GREEN dari percobaan ini.
- Tidak ada perubahan pengaturan izin, token, secrets, gate CI, atau sumber
  Rust. Push progress tetap bekerja; jangan menyimpulkan seluruh Actions mati.

GitHub mendokumentasikan **Actions repository permission: write** untuk
GitHub App/fine-grained authorization pada [Create a workflow dispatch
event](https://docs.github.com/en/rest/actions/workflows#create-a-workflow-dispatch-event).
Kegagalan yang diamati adalah penolakan otorisasi pada pemanggilan API;
penyebab spesifik pada konfigurasi integrasi tidak dapat dipastikan karena
pengaturan izin tidak dapat dibaca. Mengubah `permissions:` milik job tidak
memperluas izin koneksi agent yang memanggil API tersebut.

## Tahapan yang memerlukan pemilik koneksi

Ini tahapan manual yang diusulkan sesuai rute `/wizard`, bukan klaim bahwa
sebuah wizard interaktif sudah dijalankan. Tidak ada nilai kredensial yang
perlu disalin ke chat, `.env`, atau GitHub secrets.

| Tahap | Tindakan | Nilai yang dikumpulkan |
|---|---|---|
| 1 | Sambungkan ulang GitHub melalui kontrol koneksi GitHub di Arena, untuk akun/repository yang benar. Posisi/nama menu Arena tidak diverifikasi dari sandbox ini. | Tidak ada |
| 2 | Jika diminta GitHub, tinjau dan setujui akses aplikasi yang relevan ke `Catzpro01/hermes-rust-version`, termasuk permintaan izin Actions yang dibutuhkan. Jangan membuat atau mengirim PAT. | Tidak ada |
| 3 | Beri tahu agent bahwa koneksi telah diperbarui. Agent mengulang dispatch pada branch sesi dan memverifikasi run sebenarnya. | Konfirmasi non-rahasia saja |

Jika 403 tetap muncul setelah reconnect, hubungi dukungan Arena/pemilik
integrasi: minta pemeriksaan izin **Actions: write** untuk repository ini.
Pemilik instalasi tidak selalu dapat menambahkan izin yang belum diminta
oleh aplikasi. Jangan menonaktifkan pembatasan organisasi atau memberikan
izin luas yang tidak terkait untuk mengatasi masalah ini.

## Bukti pemulihan yang diperlukan

- [ ] Dispatch diterima oleh GitHub pada branch sesi.
- [ ] Run baru benar-benar muncul dengan SHA/ref yang sesuai.
- [ ] Job yang dimaksud berjalan; kegagalan build/test dilaporkan terpisah.

Hanya setelah itu status akses dinyatakan pulih. Keberhasilan CI push atau
pembaruan skill tidak menggantikan bukti tersebut.

## Rute kerja setelah akses pulih

`/tdd` pada output ANSI banner → verifikasi RED/GREEN dan fmt/check/clippy/test
→ `/code-review` → capture ulang → sisa kasus T12. Gunakan `/diagnosing-bugs`
jika loop tidak mengonfirmasi dugaan penyebab. Tetap di sesi/branch sekarang;
Q1–Q8 tidak dibuka ulang, dan closure/merge tidak otomatis disetujui.

## Wizard satu kali sudah disiapkan

Atas permintaan user untuk menjalankan urutan `/wizard` → `/tdd` →
`/code-review` → capture ulang, agent mencoba dispatch kandidat RED.
Hasilnya masih HTTP 403; regresi tidak berjalan.

Deliverable sesi: `/home/user/actions-access-wizard.sh`. Unduh file tersebut
ke komputer Anda, lalu jalankan di terminal interaktif:

```bash
bash actions-access-wizard.sh
```

Wizard memakai library upstream tanpa perubahan dan tiga tahap di atas.
Tidak mengumpulkan nilai rahasia, menulis `.env`/secrets, menjalankan git,
atau mencoba membuktikan izin Arena melalui kredensial gh di komputer lokal.
Ia membuka halaman umum Arena/GitHub dan meminta konfirmasi tindakan manual.
Nama/lokasi menu Arena belum diverifikasi; panduan menyebutkan batas ini.
Jika izin tidak tersedia, wizard mengarahkan ke pemilik integrasi, bukan
memperluas izin secara otomatis.

Verifikasi: `bash -n` lulus; tiga stage dan tidak adanya pemanggilan helper
credential/storage/dispatch pada bagian stage diperiksa secara statis.
Shellcheck tidak tersedia. Wizard **belum dijalankan end-to-end**, karena
memerlukan browser dan input manusia. SHA-256:
`99b541e05e042e651310d4ba93dfe1e2f54a809c108c686edffebd89defdefc9`.

Sesuai skill upstream, script ini adalah deliverable satu kali, bukan
installer permanen yang ditambahkan ke Git. Catatan, lingkup dan checksum
tersimpan di Git; template sumber tetap tersedia di skill `wizard`.
Setelah reconnect, kirim **“tersambung, cek Actions”** di sesi Arena ini.
Selesainya wizard bukan bukti izin pulih; agent tetap harus memverifikasinya.

## Rekomendasi untuk layar repository Settings → Actions → General

User kemudian menyetujui rekomendasi dengan “baik terapkan”. Target berikut
sudah disepakati; status penerapan dibedakan pada tabel di bagian terakhir:

- Pilih **Allow Catzpro01, and select non-Catzpro01, actions and reusable
  workflows**, lalu izinkan lima referensi yang benar-benar dipakai:

  ```text
  actions/checkout@*,
  actions/setup-python@*,
  actions/upload-artifact@*,
  dtolnay/rust-toolchain@*,
  Swatinem/rust-cache@*
  ```

- Aktifkan **Require actions to be pinned to a full-length commit SHA**.
  Semua `uses:` pada kedua workflow telah dimigrasikan ke SHA penuh yang
  diverifikasi melalui API upstream. Tag asal dicatat sebagai komentar.
  Pola `@*` pada allowlist mengizinkan ref dari lima repository itu; aturan
  full-SHA tetap membatasi workflow agar tidak memakai tag mutable.
- Retensi artifact/log: **90 hari** untuk investigasi ini. Bukti terpilih juga
  sudah disimpan dalam Git; retensi artifact bukan pengganti paket bukti.
- Fork PR: **Require approval for all external contributors**.
- Workflow permissions: **Read repository contents and packages permissions**.
- **Allow GitHub Actions to create and approve pull requests**: tidak dicentang.

Pilihan ini membatasi action yang boleh dipakai dan izin `GITHUB_TOKEN` di
job. Itu **bukan** izin pemanggil API dari Arena dan tidak menjamin pemulihan
403 dispatch. Untuk itu koneksi/integrasi tetap memerlukan otorisasi Actions
write yang sesuai. Jangan memilih izin workflow luas sebagai pengganti.

## Koreksi CI akibat aturan full-SHA yang sudah aktif

Setelah inspeksi awal, run `34780902412` pada `ec78897` ditemukan gagal saat
**Set up job**, bukan saat tes Rust. Annotation kedua job secara eksplisit
menyebut seluruh action harus dipin ke full-length commit SHA. Karena itu,
rekomendasi awal untuk menunda checkbox tersebut digantikan dengan migrasi
workflow, bukan mematikan aturan. Lima tag diresolusikan lewat API repository
upstream pada 2026-09-14; seluruh sepuluh `uses:` sekarang berupa SHA 40 digit.
CI baru tetap perlu diperiksa; ini tidak memperbaiki otorisasi dispatch Arena.

## Penerapan setelah persetujuan user

User menyetujui konfigurasi dengan **“baik terapkan”**. Agent menerapkan bagian
yang dapat dinyatakan di workflow tanpa memperluas izin integrasi:

| Target | Status aktual |
|---|---|
| Action dipin SHA penuh | Sudah diterapkan: 10 `uses:`; CI `34781191605` pada `9349006` lulus setelah perbaikan pin. |
| Token workflow read-only | Kedua workflow kini eksplisit `permissions: contents: read`; scope yang tidak disebut, termasuk pull-requests, tidak diberikan. Default repository belum dapat diverifikasi/diubah. |
| Retensi artifact 90 hari | Kedua upload menetapkan `retention-days: 90` untuk artifact baru, tunduk pada batas platform. Ini tidak mengubah retensi log/default repository atau artifact lama. |
| Allowlist action level repository | Belum diterapkan/diverifikasi via API; user perlu menyimpan daftar yang disepakati di Settings. |
| Approval semua contributor eksternal | Pengaturan repository, belum diterapkan/diverifikasi oleh agent. |
| Larangan membuat/menyetujui PR | Workflow kita tidak memperoleh scope tulis PR; checkbox global repository belum dapat diverifikasi/diubah. |
| Retensi log 90 hari | Pengaturan repository, belum diterapkan/diverifikasi oleh agent. |

GET Actions permissions dan GET default workflow permissions keduanya kembali
HTTP 403. Tidak ada percobaan mengubah pengaturan server secara buta setelah
pemeriksaan akses ini ditolak, dan tidak ada klaim seluruh pilihan Settings
sudah tersimpan. Gunakan halaman Settings sebagai pemilik repository untuk
bagian yang tersisa, atau pulihkan akses administrasi integrasi yang tepat.

Regresi konfigurasi workflow ditambahkan pada suite QA yang ada: RED karena
izin CI dan retensi artifact belum eksplisit, lalu GREEN setelah YAML diperbaiki.
Keenam tes QA lokal lulus. Ini **bukan** RED/GREEN perbaikan ANSI; T12 masih
menunggu akses dispatch dan bukti visual yang memenuhi persyaratan.
