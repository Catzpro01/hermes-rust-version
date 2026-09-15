# W5 — Kontrak status lifecycle kolom `Stat` picker (intr/err vs adaptasi T09)

- Status: CLOSED (2026-09-16) — keputusan HITL dijawab pengguna, dicatat di ADR 0007
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

## Temuan review PR #11 (sesi `arena/01a0a65a`, `main` = `8ad14a7`)

Review dua sumbu (Standards + Spec) terhadap diff `a008e52...8ad14a7`.
**Status tiket tidak diubah oleh bagian ini** — tetap OPEN sampai pengguna
mengonfirmasi opsi C.

### Spesifikasi

1. **Kontrak butir 4 tidak terjangkau di DB bentukan Rust.** Rust menulis baris
   hasil tool dengan `role` = *nama tool* (`store.rs`, `save_turn`), bukan
   `tool`; `resume` memetakan balik setiap role non-`user`/`assistant`. Jadi
   sesi yang terputus setelah tool berjalan (sebelum jawaban asisten) tetap
   `done`, padahal referensi memberi `intr` — padahal inilah bentuk
   `interrupted` yang dijadikan alasan memilih opsi C. DB warisan Python tidak
   terdampak. **SELESAI 2026-09-16** — pengguna memilih "beri classifier
   aturan eksplisit untuk bentuk Rust": baris terakhir yang role-nya bukan
   `user`/`assistant`/`system` diklasifikasikan `intr`. Dicatat di
   **ADR 0007** dan diimplementasikan (`classify_session_status`); adaptasi
   ini sekarang tercatat di `docs/PARITY.md`, bukan lagi keterbatasan.
2. **Kontrak butir 7 tidak diport.** Referensi menelan semua error query status
   (`_annotate_session_statuses`) lalu merender `-`; port mempropagasi
   (`lifecycle_statuses()?` → `collect_rows` → `browse`), sehingga kegagalan
   query menggagalkan perintah picker. Padahal `status_ink("-")` sudah ada.
   *Belum diperbaiki* — menyentuhnya berarti mengubah semantik error, bukan
   sekadar komentar.
3. **Properti biaya butir 1 tidak ikut terport.** Referensi membatasi grouping
   dengan `WHERE session_id IN (...)`; port melakukan `GROUP BY` atas seluruh
   tabel `messages`. Hasil sama, biaya O(semua baris). **SELESAI** —
   `lifecycle_statuses(&ids)` kini menerima daftar id, membatasi grouping
   seperti referensi, dan men-seed `{id -> empty}` untuk sesi tanpa baris
   pesan (persis `{sid: "empty" for sid in ids}` referensi). Perubahan API
   publik `hermes-core` ini disengaja dalam slice tersendiri; pemanggil
   satu-satunya adalah `collect_rows`.

### Verifikasi

4. `8ad14a7` (commit merge, ujung `main` saat ini): `vps-baremetal/fast-ci` =
   **failure**, *"Cargo Check failed (exit 101) (1s)"*, diposting **18:33:27** —
   setelah daemon beralih ke `make check` (bukti: `d150470` 18:17 sukses,
   *"All fast checks passed via make check in 88s!"*). Penjelasan "status basi"
   di atas **tidak berlaku** untuk posting 18:33:27; `285760e` (docs-only)
   juga failure pada 18:33:24. Status `main` saat ini merah dan belum
   direproduksi/diagnosis.
5. Klaim PR "+10 tes baru": delta sesungguhnya **+11** (192→203 tes, 13→14
   error; error ke-14 = tes PTY hidup baru yang butuh binary). Diverifikasi
   ulang secara independen (venv `pyte`+`pyyaml`): base `a008e52` = 192/13,
   HEAD = 203/14, **nol regresi**.
6. Gate live `test_picker_status_tags.py` tetap belum pernah berjalan terhadap
   binary mana pun: `make check` hanya `cargo check`, dan run Actions untuk
   `8ad14a7` masih *queued*.

### Perbaikan yang sudah dilakukan sesi ini (source only)

- `Makefile:7` mengutip `ci.yml:307` untuk `cargo check`; PR #11 sendiri
  menambah baris di `ci.yml:205`, jadi perintah itu kini di **308**. Diperbaiki
  — pelanggaran invarian anti-drift yang dinyatakan `Makefile` sendiri.
- Probe kolom `messages` (`lifecycle_column_expressions`, menggantikan
  `messages_have_lifecycle_columns`) kini memeriksa `tool_calls` **dan**
  `finish_reason` secara terpisah dan menyusun SQL dari ekspresi yang tersedia
  (`NULL` bila kolom tidak ada). Sebelumnya hanya `tool_calls` yang diprobe
  padahal SQL membaca dua kolom: skema parsial akan gagal di `prepare` dan
  menggagalkan seluruh picker (lihat temuan 2). Duplikasi dua literal SQL
  ikut hilang.
- Komentar yang menjanjikan "direct port" / "O(1) per session" dikoreksi agar
  sesuai kode (lihat temuan 3).

**Belum ada `cargo fmt` / `clippy` / `cargo test` untuk perubahan sesi ini** —
toolchain Rust tidak tersedia di sandbox ini. Verifikasi harus datang dari
runner resmi; jangan dibaca sebagai PASS lokal.

## Keputusan P1 (HITL) — 2026-09-16, tiket DITUTUP

Pengguna menjawab pertanyaan yang menahan tiket ini:

> Pilih: **Beri classifier aturan eksplisit untuk bentuk Rust**. Aturannya:
> Jika `role` bukan `user`, `assistant`, atau `system` (misalnya baris hasil
> tool / role `tool`), maka klasifikasikan sebagai `intr` (Interrupted).

Direkam sebagai **ADR 0007 — A session's lifecycle status comes from its last
message row, and any non-speaker role counts as interrupted**
(`docs/adr/0007-session-lifecycle-status-classification.md`), dan
diimplementasikan di `classify_session_status` beserta tes unitnya:

- urutan referensi tetap dipatok (`finish_reason` error menang sebelum role);
- `assistant` membawa `tool_calls` → `intr`; `assistant` tanpa `tool_calls`
  dan `system` → `done`;
- **selain itu → `intr`** (mencakup role `tool` pada DB Python *dan* nama tool
  pada DB bentukan Rust);
- role kosong/tidak dikenal ikut terbaca sebagai baris hasil tool → `intr`,
  menyimpang dari default benign referensi (`complete`) dan **dideklarasikan**
  sebagai adaptasi di `docs/PARITY.md`, bukan diklaim sebagai parity.

Kosakata yang dipakai keputusan ini (**lifecycle status**, **tool-result row**)
ditambahkan ke `CONTEXT.md`.

Verifikasi yang sudah ada untuk pekerjaan ini (dijalankan pengguna di VPS
bare-metal 6-core, terhadap `eea45ee`, sebelum perubahan P1):
`make fmt` EXIT 0, `make clippy` EXIT 0, `make test` EXIT 0 (picker e2e 7/7,
subcommands 33/33, wizard 10/10, streaming 3/3, smoke 7/7). Perubahan P1
sendiri **belum** diverifikasi — butuh `make fmt && make clippy && make test`
ulang setelah commit ini (daemon VPS sudah diperbaiki dengan
`RUSTUP_TOOLCHAIN=stable`, rustc 1.98.1).

### Yang terbawa ke tiket lain (bukan lagi keputusan tiket ini)

- Capture `picker-status-tags` terhadap binary hasil build — gate live
  `scripts/test_picker_status_tags.py` masih belum pernah dijalankan nyata.
  Fixture kini **enam** bentuk: bentuk ke-6 (`tool-result-last`, baris terakhir
  ber-role `shell`) menuntut `intr` dan menjadi satu-satunya bentuk tempat
  Rust dan referensi Python berbeda secara terdeklarasi (Python: `done`) —
  bedah ADR 0007 yang akhirnya punya bukti hidup. Tuntutan gate naik 3 → 4
  pelanggaran; 10 tes decoder tetap hijau, suite Python tetap 203/14.
- P2: error query status seharusnya ditelan dan dirender `-`, bukan
  menggagalkan picker (kontrak butir 7) — **SELESAI** (`status_tag`,
  `SessionStatus::Unknown` → `-`).
- P3: `lifecycle_statuses(&ids)` agar grouping dibatasi seperti referensi
  (`WHERE session_id IN (...)`) — **SELESAI**.
- Status merah pada ujung `main` (`8ad14a7`) adalah kegagalan environment
  daemon yang sudah diperbaiki di VPS, bukan regresi kode.
