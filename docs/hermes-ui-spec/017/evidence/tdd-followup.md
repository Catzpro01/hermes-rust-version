# Hermes banner — tindak lanjut TDD, 2026-09-14

**Empat slice RED → GREEN terverifikasi. Review dan capture final masih pending.**

Seam yang dikonfirmasi user: public `write_banner`. Bukti Python independen
berasal dari `banner-a2a3d08/`, bukan snapshot yang dibuat dari output Rust.
Setiap slice diamati gagal terlebih dahulu, lalu hanya koreksi terkait diuji.

| Slice | RED (tes bernama benar-benar gagal) | GREEN (fmt/check, regresi, clippy, seluruh tes workspace) | Perbaikan |
|---|---|---|---|
| Layout session panjang | [34783950104](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34783950104) | [34784076114](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34784076114) | Pertahankan alokasi kolom setelah wrap; posisi Python 51/41/48/49 |
| Bold judul → border | [34784219747](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34784219747) | [34784316150](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34784316150) | Reset SGR aktif sebelum style berubah; bold tidak merembet |
| Dim teks sekunder | [34784445723](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34784445723) | [34784546009](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34784546009) | Dim pada label/cwd/session; ellipsis crop mempertahankan span style |
| Koma dan `...` tool | [34784685139](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34784685139) | [34784783352](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34784783352) | Nama berwarna body, koma default, marker default + dim; budget truncation tetap |

Keempat regresi mencakup 100/80/94/95 kolom dan truecolor/256-color. Tes style
mendekode stream SGR publik, tidak membaca buffer internal. Parameter RGB
1/2 dikonsumsi sebagai bagian warna, bukan dianggap modifier bold/dim.
Regresi lama juga tetap lulus dalam setiap pemeriksaan workspace GREEN.

## Integritas penerapan kode

Setiap delta berikut diambil dari runner yang telah menjalankan fmt/check dan
tes; hash/ukuran diverifikasi, `git apply --check` dijalankan, dan diff Rust
lokal dibandingkan byte-per-byte sebelum commit. Tidak ada klaim build lokal.

| Slice | Byte | SHA-256 delta yang diuji |
|---|---:|---|
| Layout | 4075 | `ffe59885824b13a0c6d73f54eff9b12691b5da93e93aca91e37b0f0425b244cd` |
| Bold | 6319 | `7859e9d849c6f5eec26f69e98570514d1751c05f87d92ab77dfee3962f9bfe2c` |
| Dim | 5244 | `8f04a4cb9efe748c10e2cc3fd49234a817a9194679ca1bf3c3bd10bf8b31057a` |
| Punctuation | 5232 | `69584e4065123dcb522cba2cf04be736bd5ed395574825dff0c866f56db9e573` |

Tidak ada perubahan pada shared theme atau instalasi/data Python. Perubahan
runtime dan regresi berada di `crates/hermes-cli/src/tui/welcome.rs`.

## Verifikasi commit aktual

Kode `3e7c89590b22881898b0f0b37e882a5d415d072b` lulus
[CI 34784967731](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34784967731)
(format, clippy, tes workspace, QA workflow).
[Run 34784967746](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34784967746)
juga memverifikasi bahwa build/capture/export final **skipped**, sesuai
`capture: false`. Keberhasilan run tersebut bukan hasil capture baru.

## Batas review dan capture

- Kandidat sudah dikonsumsi; request `phase: none, capture: false` menahan
  capture final sampai tahap review. Build/capture kandidat pada run GREEN
  hanyalah diagnostik, bukan bukti final dari sumber committed.
- Delapan QA workflow lokal lulus, termasuk exact-test gate dan defer-capture.
  Pengujian selector tidak menerima kegagalan tes lain atau nol tes sebagai RED;
  flag capture harus boolean. Semua gate Rust tetap aktif untuk kandidat.
- `/code-review` masih membutuhkan fixed point dari user. Skill aslinya juga
  membutuhkan dua subagent independen, yang tidak tersedia di sesi ini.
  Pemeriksaan langsung atau paket untuk reviewer manusia harus dibedakan dari
  review independen; tidak ada verdict yang direkayasa.
- Setelah batas review diselesaikan: aktifkan capture, verifikasi commit/hash
  sumber dan diff kosong, pasangkan ulang dengan Python, pertahankan raw/PNG
  baru, lalu lanjutkan wizard/picker/completion/summary sesuai T12.
- Bukti visual lama tidak diubah menjadi PASS. Tes yang hijau belum membuktikan
  seluruh kesetaraan visual. Closure tetap membutuhkan persetujuan user.

**Tidak ada merge atau persetujuan closure.**
