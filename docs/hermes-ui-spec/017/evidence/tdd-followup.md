# Hermes banner — tindak lanjut TDD, 2026-09-14

**Empat slice serta centering follow-up RED → GREEN terverifikasi; review langsung terbatas dan recapture 5a8e12c selesai. Bukti UI keseluruhan belum lengkap.**

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

## Hasil review, residual defect, dan recapture aktual

User memilih baseline 059cd65/all ANSI dan mendelegasikan mode terbaik.
Review langsung terbatas Standards/Spec selesai, bukan subagent independen.
Duplikasi decoder tes dibersihkan lewat GREEN 34785303446, diterapkan tepat
pada b56c9a3. Capture aktual b56c9a3 menemukan Session kolom 21/20 di 94×30;
paket FAIL itu tetap immutable.

Siklus kelima: `banner_ansi_wrapped_session_label_is_centered` di public writer,
100/80/94/95 dan truecolor/256-color. RED 34785680910 (5a2207d), GREEN
34785789939 (97ce7b4). Regresi Python mengharapkan Session 4/17/21/21;
centering kini mengukur teks terlihat, bukan whitespace separator akhir wrap.
Patch GREEN 3644 byte, SHA-256
`d306f7bb2d8861e1c1a50dc6cc1e2f2871f5b4a9c11539058e4c6119652095c8`,
terverifikasi dan diterapkan byte-identik. Regresi lama tidak dilemahkan.

Sumber aktual 5a8e12c97dd760bf05f411a5d59585a97f0b5ad6: CI 34785921216 dan
capture 34785921221 SUCCESS. [Paket baru](banner-5a8e12c/REPORT.md) diperiksa
langsung seluruh delapan PNG dan raw/event/cast. Tidak ada perbedaan glyph
non-judul atau atribut glyph sama; label branding adalah adaptasi terdokumentasi.
Tidak ada normalisasi raw/PNG. Hash sumber dan diff Rust kosong diverifikasi.

Candidate dikonsumsi. Request none/false setelah bukti disimpan mencegah
recapture banner berulang pada push dokumentasi. Delapan QA workflow lokal
lulus; Rust diperiksa di runner, bukan klaim build lokal. Berikutnya bukti
wizard/picker/completion/summary, bukan mengulang siklus banner yang selesai.

**Tidak ada review independen, penerimaan keseluruhan, closure, atau merge.**
