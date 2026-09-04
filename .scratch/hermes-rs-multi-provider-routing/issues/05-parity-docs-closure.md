# 05: Bukti parity, dokumentasi, dan penutupan Spec 005

**What to build:** Uji end-to-end lintas provider, pembaruan dokumen, dan bukti bahwa instalasi Python Hermes tidak tersentuh.

**Blocked by:** 01, 02, 03, 04.

**Status:** done — commit di VM, 119/119 test hijau (`cargo test --workspace`), `clippy --workspace --all-targets -D warnings` bersih.

## Kriteria

- [x] E2E: config dengan dua provider, switch di tengah session, kedua respon tercatat benar di `state.db`.
      `crates/hermes-cli/tests/provider_routing_e2e.rs` (`two_providers_switch_mid_session_and_both_are_recorded`)
      menjalankan binary sungguhan terhadap dua server wiremock; memastikan respon `hello-from-alpha`
      (turn alpha) dan `hello-from-beta` (turn beta) tercatat dalam session yang sama, berurutan, tanpa turn tercampur.
- [x] E2E: credential provider A tidak pernah muncul saat provider B aktif — memakai pola yang sudah ada di `search_credential_safety.rs`.
      Kedua test di file itu menegaskan stdout & stderr bebas dari `sk-alpha-...` maupun `sk-beta-...`;
      test kedua (`switching_to_unavailable_provider_keeps_active_one_and_its_credential`) memeriksa rollback
      menyebut nama variabel yang hilang (`HERMES_E2E_BETA_KEY`), bukan nilai, dan provider aktif tetap alpha.
- [x] Regresi: seluruh suite tetap hijau; jumlah test dilaporkan, bukan diasumsikan.
      `cargo test --workspace` → 119/119, stabil di run penuh; dilaporkan eksplisit di `docs/ROADMAP.md`.
- [x] `docs/PARITY.md` diperbarui dengan perilaku routing yang sudah setara dengan Python.
      Ditambahkan section "Spec 005 — provider routing parity" (selection, credential, mid-session switch,
      wire-mode) serta baris Compatible yang sesuai. Fakta Python diverifikasi dari instalasi
      `~/.hermes/hermes-agent` (provider catalog, `api_key_env_vars`, mid-session model switch).
- [x] `docs/ROADMAP.md`: Spec 005 → Done, hanya setelah suite hijau.
      Status Spec 005 diubah ke `Done`; ditambahkan section "Spec 005 closure" (tabel ticket→commit) dan
      angka verifikasi 119/119 diperbarui. Perubahan docs dikerjakan setelah suite hijau.
- [x] Smoke `smoke_python_hermes_untouched` tetap lulus — instalasi Python tidak dimodifikasi.
      Test ini lolos di full suite; seluruh kerja Spec 005 memakai `HERMES_HOME`/temp, tak pernah menulis
      ke `~/.hermes`.

## Pelajaran dari Spec 004 yang wajib diterapkan di sini

Spec 004 ditutup dengan satu test redaction yang hanya menguji pola
`API_KEY=`. Jalur `sk-` tidak pernah diuji, dan di situlah bug bersembunyi
sampai audit menemukan `min_suffix = 20` di `redact.rs`.

Karena itu untuk tiket ini:

- [x] Setiap cabang routing punya test-nya sendiri — jangan buktikan satu mode lalu menyimpulkan mode lain ikut benar.
      Cabang "switch sukses antar dua provider" (`two_providers_switch_mid_session_and_both_are_recorded`)
      dan "switch gagal → rollback + credential aktif dipertahankan"
      (`switching_to_unavailable_provider_keeps_active_one_and_its_credential`) diuji terpisah.
- [x] Test negative ditulis lebih dulu: apa yang **tidak** boleh terjadi.
      Assert bahwa credential tidak pernah bocor (stdout & stderr), turn tidak tercampur/tertukar antar provider,
      dan provider aktif tidak berubah saat init gagal.
- [x] Ambang dan batas yang dipakai di kode disebut eksplisit di test, supaya perubahan angka langsung mematahkan test.
      Secret test dipakai persis (mis. `sk-alpha-e2e-secret-1111111111`) dan nama env (`HERMES_E2E_ALPHA_KEY`) disebut
      eksplisit; bila kode mengganti nama env atau menyalurkan credential berbeda, test akan gagal.

## Perubahan

- `crates/hermes-cli/tests/provider_routing_e2e.rs` (baru): 2 test E2E (wiremock, runtime tokio multi-thread).
- `docs/PARITY.md`: tambah section parity routing Spec 005 + baris Compatible.
- `docs/ROADMAP.md`: Spec 005 → Done, section closure, verifikasi 119/119.
