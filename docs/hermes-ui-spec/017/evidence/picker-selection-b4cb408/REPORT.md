# Baris terpilih picker (fix `ee11541`, capture `b4cb408`)

RED [34866264371](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34866264371)
→ GREEN resmi [34866921566](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34866921566)
→ capture sumber ter-commit
[34868306211](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34868306211)
→ CI biasa
[34867949911](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34867949911) /
[34868819214](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34868819214) SUCCESS.

Scope: **gaya baris terpilih** — seluruh baris kursor, termasuk penanda ` → `,
memakai palette slot 2 + bold tanpa reverse video. Kolom status, `Active`, `ID`,
geometri dan teks tidak diubah.

## Kontrak yang dipatok referensi

| Fakta | 100 kolom | 80 kolom |
|---|---|---|
| Baris kursor | baris 4, penanda ` → `, span 0…93 | baris 4, ` → `, span 0…75 |
| Gaya | satu run `palette2 + bold`, **tanpa** reverse video | sama |
| Baris tak terpilih | default, tidak bold, tanpa reverse, tanpa hijau | sama |

Bagian yang dipatok berhenti di kolom status (43 / 25): isi kolom status dan
setelahnya adalah adaptasi terdokumentasi, jadi bukan bagian gate ini.

## Perubahan (patch teruji `8eed758f…`, 1.171 byte)

`crates/hermes-cli/src/session_picker.rs`: cabang `is_cursor_row` mengganti
`SetAttribute(Attribute::Reverse)` dengan
`SetForegroundColor(Color::DarkGreen)` + `SetAttribute(Attribute::Bold)` + reset.

## Verifikasi

- RED: gate live gagal 3× dengan nama tes benar dan **tanpa** error setup; dua
  masalah per kasus (reverse masih aktif; gaya default bukan palette2+bold).
- GREEN: patch hasil ekspor identik byte dengan proposal (`8eed758f…`, 1.171 byte);
  gate live 3×, fmt/check, `clippy --workspace --all-targets -D warnings` dan
  seluruh suite PASS.
- Capture: 10 kasus; **sembilan** checker (hint, counter, posisi, warna, header
  normal, header filter, tata letak kolom, prompt/message, baris terpilih)
  semuanya PASS pada sumber ter-commit. Bundle `fe8bc2c2…`, 423.798 byte.
- `verify.py` → `audit.json` `SELECTION_ROW_FIXED_ALL_PINNED_REGIONS_EQUAL`:
  20 round trip raw/cast, 10 hash PNG, 12 jalan checker, dan **34 dari 34 region
  piksel identik** — termasuk dua region baru untuk baris terpilih di kedua lebar
  dan baris kontrol tak terpilih — serta 4 PNG kasus empty byte-identik.
- Delta sel terhadap paket `picker-message-style-fd674dc`: **hanya baris 4** pada
  ketiga skenario ber-kursor, di kedua lebar (83 sel @100, 65 sel @80). Tiap sel:
  `inverse True→False`, `fg default→2`, `bold False→True`, dan mode palet256
  `0→33554432`; **teks baris tidak berubah**. Sel dan rekaman Python tidak berubah.

## Percobaan yang gagal (disimpan, bukan disembunyikan)

Percobaan GREEN pertama ([34866468131](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34866468131))
mengekspor log yang menunjukkan gate lulus, tetapi loop tiga kali tetap berakhir
non-zero; artefak iterasi yang gagal ada di upload yang diblokir, jadi penyebabnya
tidak dapat direproduksi. Permintaan ulang dengan sumber dan patch yang sama
berhasil 3×. Selain itu: dua capture gagal tanpa sebab terlihat (memunculkan
`capture-verify.log` + anotasi), satu capture gagal karena **bug di checker kami
sendiri** (`38;5;2` dibaca sebagai dim — fix `1c40eea`), dan satu flake start-up
PTY (izin 45 s + pesan diagnostik `400f5e2`, `064e92d`). Semuanya tercatat di
`attempts-result.txt`; tidak ada kegagalan yang diam-diam diubah jadi sukses.

## Batasan

- Bukan PASS seluruh picker: warna kolom status (belum ada bukti terpinn untuk
  nilai Rust `done`), konten `Active`/`ID` (adaptasi), resize/daftar
  panjang/clear-filter masih terbuka.
- Capture commit `b4cb408` memuat perubahan **tooling saja** dibanding commit fix;
  `verify.py` membuktikan tidak ada berkas Rust yang berbeda.

Reproduksi: `verify.py`; `SHA256SUMS` memuat seluruh berkas; `verify-regions.cjs`
+ `region-spec.json` mereproduksi perbandingan piksel; `pair-bundle.py`
mereproduksi pasangan dengan rekaman Python yang dipakai ulang.
