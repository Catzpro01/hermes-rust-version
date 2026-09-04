# 05: Bukti parity, dokumentasi, dan penutupan Spec 005

**What to build:** Uji end-to-end lintas provider, pembaruan dokumen, dan bukti bahwa instalasi Python Hermes tidak tersentuh.

**Blocked by:** 01, 02, 03, 04.

**Status:** todo

## Kriteria

- [ ] E2E: config dengan dua provider, switch di tengah session, kedua respon tercatat benar di `state.db`.
- [ ] E2E: credential provider A tidak pernah muncul saat provider B aktif — memakai pola yang sudah ada di `search_credential_safety.rs`.
- [ ] Regresi: seluruh suite tetap hijau; jumlah test dilaporkan, bukan diasumsikan.
- [ ] `docs/PARITY.md` diperbarui dengan perilaku routing yang sudah setara dengan Python.
- [ ] `docs/ROADMAP.md`: Spec 005 → Done, hanya setelah suite hijau.
- [ ] Smoke `smoke_python_hermes_untouched` tetap lulus — instalasi Python tidak dimodifikasi.

## Pelajaran dari Spec 004 yang wajib diterapkan di sini

Spec 004 ditutup dengan satu test redaction yang hanya menguji pola
`API_KEY=`. Jalur `sk-` tidak pernah diuji, dan di situlah bug bersembunyi
sampai audit menemukan `min_suffix = 20` di `redact.rs`.

Karena itu untuk tiket ini:

- [ ] Setiap cabang routing punya test-nya sendiri — jangan buktikan satu mode lalu menyimpulkan mode lain ikut benar.
- [ ] Test negative ditulis lebih dulu: apa yang **tidak** boleh terjadi (credential bocor, turn tercampur dua provider, provider aktif berubah saat init gagal).
- [ ] Ambang dan batas yang dipakai di kode disebut eksplisit di test, supaya perubahan angka langsung mematahkan test.
