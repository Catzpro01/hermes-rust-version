# 01: Integrasi token counting ke runner

**What to build:** Wire helper `estimate_turns_tokens` (Spec 006 #04) ke
`ConversationRunner` sehingga total estimasi token per-kontek tersedia & bisa
di-query, dan REPL bisa menampilkannya (token accounting sebagai fondasi semua
tiket Spec 008).

**Blocked by:** — (Spec 006 #04 sudah landed sebagai fondasi).

**Status:** todo

## Kondisi sekarang (terverifikasi)

- `crates/hermes-core/src/conversation/context.rs` punya `estimate_turns_tokens`.
- `ConversationRunner<P: Provider>` (`conversation/mod.rs`) memegang
  `turns: Vec<Turn>`; tidak ada penghitung token yang terekspos.
- REPL (`crates/hermes-cli/src/repl.rs`) mencetak baris sambutan
  `Hermes-RS session {id} (provider {name})`; tidak menampilkan ukuran konteks.

## Kriteria

- [ ] `ConversationRunner` mengekspos `estimated_tokens(&self) -> usize` yang
      memakai `estimate_turns_tokens(self.turns())` (terus konsisten dengan
      helper yang sama, jangan duplikasi).
- [ ] Diupdate otomatis setiap turn masuk/keluar (pakai metode existing, tak
      perlu cache yang bisa basi).
- [ ] REPL menampilkan estimasi konteks (mis. baris sambutan atau `/info`)
      tanpa mengubah arsitektur REPL; angka = dari helper yang sama.
- [ ] Test: tambah turn → angka naik sesuai `estimate_turns_tokens`; runner
      kosong → 0.
- [ ] Tidak mengubah perilaku kirim (murni read-only accounting).

## STRIDE

- Tidak ada surface credential/eksekusi baru; murni read-only penghitungan.
- Angka estimasi tidak pernah bocor isi turn (hanya angka).

## Risiko

- Duplikasi heuristik: harus selalu delegasi ke `estimate_turns_tokens`, jangan
  definisi ulang `char/4` di runner.
- Over-engineering: cukup expose method + tampil; jangan bangun "token
  dashboard" sebelum dibutuhkan.

## Dependency

Spec 006 #04 (context helper) — sudah done.
