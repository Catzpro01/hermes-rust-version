# 04: Pinned messages (/pin)

**What to build:** Pengguna bisa menandai turn penting agar **tidak pernah**
terdorong keluar oleh sliding window (ticket 02) — misalnya fakta kunci, batasan,
atau instruksi lintas-percakapan panjang.

**Blocked by:** 02 (window harus hormati kumpulan pin).

**Status:** todo

## Kondisi sekarang (terverifikasi)

- `ConversationRunner` tidak punya konsep pin; window (02) akan memangkas dari
  tertua tanpa pandang bulu.
- REPL punya dispatch `/...` (mis. `/new`, `/resume`, `/provider`, `/sessions`).
  Tidak ada `/pin`.

## Konsep

Simpan set indeks/penanda turn yang di-pin di dalam `ConversationRunner`
(in-memory, per-runner). Sliding window (02) tidak boleh menjatuhkan turn
berpin. Representasi pin: indeks stabil ke `turns`, atau id — pilih yang tak
mudah basi saat `replace_turns`/`resume`.

## Kriteria

- [ ] `ConversationRunner` mengekspos `pin(index)` / `unpin(index)` /
      `pinned(&self)` (atau id-based), dan window (02) meng-exempt turn berpin.
- [ ] Pin bertahan saat turn lain di-drop (indeks dipetakan ulang dengan benar
      bila window menghapus turn sebelum turn berpin).
- [ ] REPL: perintah `/pin <turn_index>` dan `/unpin <turn_index>` (+ `/pin`
      tanpa arg untuk daftar). Indeks yang tak valid → pesan jelas, tak panik.
- [ ] Test: pin turn tengah → window menjatuhkan turn lain tapi turn berpin
      tetap hadir di `turns_to_send`; unpin → bisa di-drop lagi.
- [ ] Ambang: jumlah maks pin waras (opsional) dibatasi & didokumentasikan.

## STRIDE

- **Prompt-injection/kepercayaan:** pin memperbesar bobot isi turn itu; jelaskan
  di output bahwa itu pilihan pengguna. Tidak ada jalur credential baru.
- Tidak ada surface eksekusi baru.

## Risiko

- Indeks berubah saat turn di-drop/resume — harus definisikan semantik yang
  jelas (mis. pin mengikuti turn asli, bukan posisi) agar tidak salah-pin.
- Interaksi pin + resume dari `state.db` (turn id perlu eksis di kedua sisi).

## Dependency

02.
