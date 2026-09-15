# W5 — Kontrak status lifecycle kolom `Stat` picker (intr/err vs adaptasi T09)

- Status: OPEN
- Type: wayfinder:grilling
- HITL: yes
- Owner: Arena agent (sesi arena/01a0a493)
- Parent map: [Wayfinder map — Rute penuntasan Spec017](WAYFINDER-spec017-closure.md)
- Blocked-by: —

## Question

`docs/PARITY.md` mencatat adaptasi terdokumentasi T09: kolom status picker
Rust hanya `done` (ada pesan) atau `empty` (tanpa pesan). Referensi Python
v0.21.0 memetakan **empat** status lifecycle (`complete`/`interrupted`/
`error`/`empty` → `done`/`intr`/`err`/`empty`), dan T13 Cycle 7 kandidat (c)
menuntut "fixture that exercises the interrupted/error/empty inks in a live
capture (today only `complete`/`done` is captured)".

Keputusan yang diperlukan: apakah adaptasi T09 dipertahankan sebagai batas
scope (maka sisa coverage §J.7 untuk kolom status ditutup sebagai
keterbatasan yang dinyatakan), atau kontrak referensi di atas dip/port
sebagian/penuh — dan kalau diport, dari sumber data mana status diturunkan,
karena skema `messages` bentukan Rust belum punya kolom `tool_calls` /
`finish_reason` yang dibaca referensi.

## Contract as pinned (riset inline, tidak perlu tiket terpisah)

Bukti: [`upstream-lifecycle-status/`](../../../docs/hermes-ui-spec/017/evidence/upstream-lifecycle-status/provenance.json)
(`hermes_state.py` blob `166c39a5…` diverifikasi dengan `git hash-object`,
commit `63279301`; excerpt verbatim `hermes_state_excerpt.py`).

1. Sumber: **satu baris pesan terakhir** per session id — `MAX(id)` lalu join
   (indeks `idx_messages_session_id`), **tidak pernah** scan transkrip.
2. Tidak ada baris pesan → `empty`.
3. `finish_reason` (strip + lowercase) ∈ {`error`,`agent_error`,`content_filter`}
   → `error` — diperiksa **sebelum** role.
4. Role `assistant` dengan kolom `tool_calls` tidak NULL → `interrupted`
   (hasil tool belum pernah mendarat); role `user` atau `tool` → `interrupted`.
5. Bentuk lain → `complete` (default benign; picker tidak boleh panik).
6. Kata tag dari `_session_status_tag`: `complete`→`done`, `interrupted`→`intr`,
   `error`→`err`, `empty`→`empty`, tidak dikenal → `-`; tinta/kolom tag sudah
   dipatok di `evidence/upstream-status-attr/`.
7. `_annotate_session_statuses` **menelan semua error** query status: baris tetap
   tanpa tag → picker merender `-`, bukan gagal.

Fakta Rust saat ini (`crates/hermes-cli/src/session_picker.rs::collect_rows`):
status = `empty` bila `session.turns.is_empty()` else `done`; `status_ink`
sudah memetakan keempat tag, jadi cabang `intr`/`err` hidup di display tetapi
tidak dapat dijangkau oleh data.

## Options yang diajukan

- **A — Tidak mengubah runtime** (hanya bukti): adaptasi T09 dipertahankan;
  Cycle 7 (c) ditutup sebagai keterbatasan yang dinyatakan + unit test pemetaan
  tinta; `docs/PARITY.md` menambah kalimat bahwa `intr`/`err` tidak dapat
  muncul di Rust. Biaya CI minimal; coverage §J.7 kolom status tetap parsial.
- **B — Port penuh apa adanya**: jalankan query referensi apa adanya terhadap
  kolom `messages.tool_calls` / `messages.finish_reason`, dan ikuti jalur
  fallback referensi (`-`) bila kolom tidak ada. Paling identik secara byte,
  tetapi DB bentukan Rust sendiri akan berubah dari `done` menjadi `-` untuk
  semua baris — regresi tampilan nyata pada alur utama pengguna Rust.
- **C — Port bertahap sesuai bentuk data** (rekomendasi): turunkan status dari
  baris pesan terakhir juga, pakai `finish_reason`/`tool_calls` **bila** kolom
  itu ada (DB warisan Python = target kompatibilitas `~/.hermes`), dan bila
  tidak ada gunakan aturan role referensi saja (`user`/`tool` → `intr`,
  `assistant` → `done`, tanpa pesan → `empty`) — `err` memang tidak dapat
  terjadi di DB bentukan Rust, persis seperti referensi. Tidak ada kolom baru,
  tidak ada regresi `done`, dan `intr`/`err` menjadi tercapturable di hidup.
  Fixture capture meng-seed DB berbentuk Python agar gate live benar-benar
  melihat keempat tag.

## Rekomendasi

**C.** Alasan: tetap di dalam batas Spec017 (parity tampilan, bukan fitur
penyimpanan baru), tidak menyentuh skema kanonik maupun instalasi Python,
tidak menghapus adaptasi terdokumentasi yang sudah diterima (Delivered rows
`done`/`empty` tetap), dan membuka satu-satunya jalur yang membuat T13 Cycle
7 (c) dan baris "status ink untuk nilai Rust `done`" di T12 bisa ditutup
dengan bukti, bukan dengan penghapusan tuntutan.

## Penerapan patch sesi `arena/01a0a600` — harness gate sudah diverifikasi

Patch `01a0a493-1c05-7ccc-a6dc-983ef8d7b471.patch` (7 berkas, ~17,9 k baris —
sebagian besar `hermes_state.py` referensi yang disematkan sebagai bukti
`upstream-lifecycle-status/`) diterapkan bersih ke `a008e52`. Isinya hanya
sisi harness: fixture capture `picker-status-tags`, checker
`check_picker_status_tags.py`, dan `test_picker_status_tags_checks.py`.
**Tidak ada baris runtime Rust yang diubah**, jadi pilihan A/B/C di atas masih
terbuka — keputusan tetap HITL.

Patch dikirim dalam keadaan rusak; tiga cacat nyata ditemukan dan diperbaiki
sebelum gate bisa dipakai (bukti: `unittest` lokal, 10/10 hijau):

1. `body_row()` memakai `i` yang tidak pernah didefinisikan → `NameError` di
   **seluruh 10 tes**. Diperbaiki dengan meneruskan indeks baris dari `frame()`
   lewat parameter `index`, sekaligus membuat kolom ID di akhir baris
   menampilkan prefiks sesi yang di-seed (`0000000<index+1>`) supaya baris
   hasil render bisa dilacak balik ke baris fixture-nya.
2. `test_collapsing_rows_to_done_fails_exactly_the_other_shapes` meng-collapse
   **semua** baris menjadi `done`, padahal port hari ini benar memberi `empty`
   untuk sesi tanpa baris pesan → 4 pelanggaran, bukan 3. Diperbaiki pada
   simulasi port-nya: hanya baris yang punya pesan yang di-collapse. Gate
   tetap menuntut 3 pelanggaran (`intr`, `intr`, `err`) — tuntutan tidak
   diturunkan, yang dikoreksi adalah ketidakakuratan simulasi.
3. `tag_sgr()` menebak baris cursor dari **nilai tag** (`tag == LIFECYCLE_TAGS[0]`)
   alih-alih dari posisi cursor. Saat cursor digeser ke baris 4, baris 3 ikut
   dicat bold dan gate mengeluh "must not be bold". Diperbaiki: bold mengikuti
   posisi cursor, sesuai `_status_attr` referensi (`if i != cursor`).

Verifikasi yang benar-benar dijalankan (bukan prediksi):

- `python3 -m unittest discover` di `scripts/`: **202 tes, 13 error**.
- Baseline `a008e52` (worktree terpisah, tanpa patch): **192 tes, 13 error**.
  Jadi +10 tes baru semuanya hijau dan **nol regresi**.
- Ke-13 error identik di baseline maupun sesudahnya: `KeyError:
  'HERMES_PICKER_BINARY'` pada tes PTY hidup yang butuh binary Rust terkompilasi.
  `cargo`/`rustc` tidak tersedia di sandbox ini (bloker lingkungan yang sama
  dengan yang sudah tercatat di PROGRESS.md), sehingga sisi Rust harus
  diverifikasi lewat CI.
- `PICKER_LIFECYCLE_CASES` bersifat opt-in: `CASES` default tidak disentuh,
  jadi bundle lama mempertahankan daftar kasusnya persis seperti sebelumnya.

Gate ini **akan merah** terhadap Rust hari ini — itu memang tujuannya (RED
dulu): `collect_rows` menurunkan status dari `session.turns.is_empty()`, jadi
`intr`/`err` tidak pernah bisa muncul. Jangan "dihijaukan" dengan menurunkan
tuntutan gate.

## Resolution — opsi C diimplementasi dan terverifikasi

Dipilih: **opsi C** (port bertahap sesuai bentuk data), sesuai rekomendasi
tiket. Catatan kejujuran: keputusan ini diambil karena pengguna melewati
pertanyaan konfirmasi dan meminta pekerjaan dilanjutkan, bukan karena jawaban
eksplisit atas pertanyaan HITL di atas. **Konfirmasi pengguna masih
diperlukan untuk menutup tiket ini menjadi CLOSED.**

Implementasi: `2d36af9` (`hermes-core`: `SessionStatus` +
`classify_session_status` + `SessionStore::lifecycle_statuses`;
`hermes-cli`: `collect_rows` memakainya).

### Hasil verifikasi nyata

- **`make check` = exit 0.** Status commit `a4d96bb`:
  `vps-baremetal/fast-ci` = **success**, *"All fast checks passed via make
  check in 65s!"*. Ini sekaligus membuktikan daemon VPS sudah memakai
  `make check`, `Makefile` yang ditambahkan bekerja, dan workspace
  (`hermes-core` + `hermes-cli` + dependensi) terkompilasi bersih.
- **`tests/session_picker_e2e.rs`: 7 passed, 0 failed** (dilaporkan pengguna
  dari VPS; belum direproduksi mandiri karena sandbox tidak punya toolchain
  Rust). Tujuh skenario: frame verbatim + Enter, filter langsung + counter
  footer, panah bawah, Esc batal, hapus sesi, store kosong, dan buka picker
  dari REPL lalu resume.
- Dua asersi yang mengode kata lama bergeser `done` → `intr`
  (`collect_rows_sanitizes_and_single_lines_names` dan
  `session_picker_e2e::browse_renders_verbatim_frame...`). Keduanya men-seed
  satu pesan `user`, yang menurut kontrak referensi = `interrupted`. Lolos.

### Koreksi terhadap catatan sebelumnya

Bagian "Penerapan patch" di atas dan entri PROGRESS tertanggal sama dulu
menyimpulkan *"VPS/daemon tidak menjawab"* dari `curl` yang `connection reset
by peer`. Kesimpulan itu **salah**. Yang benar: sandbox ini tidak bisa
membuka koneksi TCP ke `203.145.35.218:9000` (reset seketika = pembatasan
egress), sedangkan daemon **berjalan** — dibuktikan oleh status
`vps-baremetal/fast-ci` = success pada `a4d96bb`. `curl` dari sandbox bukan
alat ukur yang sah untuk kesehatan VPS.

Demikian pula kegagalan lama *"Cargo Check failed (exit 101) (3s)"* pada
`4709543`/`5800741`/`2d36af9` kini terbaca sebagai status **basi** yang
diposting daemon sebelum dialihkan ke `make check`; status pada
commit lama tidak ditulis ulang. Jangan dibaca sebagai regresi kode.

### Yang belum tertutup

- `cargo fmt --all -- --check` dan `clippy --workspace --all-targets -- -D
  warnings` **belum** pernah dijalankan terhadap perubahan ini (`make check`
  hanya `cargo check`). Keduanya ada sebagai target `make fmt` / `make clippy`.
- Gate live `test_picker_status_tags.py` sudah terikat di CI dan tes
  decoder-nya lolos, tetapi **belum pernah dijalankan terhadap binary hasil
  build** — capture `picker-status-tags` belum diambil.
- Status tiket tetap **OPEN** sampai pengguna mengonfirmasi opsi C secara
  eksplisit.
