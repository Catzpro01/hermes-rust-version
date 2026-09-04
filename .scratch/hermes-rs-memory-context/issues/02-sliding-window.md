# 02: Sliding window (drop turns tertua)

**What to build:** Ketika estimasi konteks melebihi `context_length`, sebelum
mengirim turn berikutnya, jatuhkan turn tertua (selain yang diproteksi: pesan
yang di-pin, lihat 04) agar permintaan ke provider tetap muat. Drop hanya
berlaku pada **salinan yang dikirim ke provider**, bukan menghapus turn dari
`state.db`/sesi.

**Blocked by:** 01 (token accounting).

**Status:** todo

## Kondisi sekarang (terverifikasi)

- `ConversationRunner::chat_agentic`/`chat` mengirim `self.turns` penuh ke
  `provider.chat_with_cancel(&self.turns, ...)`.
- Tidak ada pemangkasan: percakapan panjang mengirim seluruh sejarah tanpa batas.
- `context_length: Option<u64>` ada di `ModelConfig` dan `ProviderConfig`,
  diparse tapi tidak dibaca untuk perilaku.

## Konsep

Sliding window harus menjaga **integritas turn yang sudah disimpan**:
`state.db` tetap menyimpan seluruh turns sesi (canonical). Yang dipangkas hanya
`Vec<Turn>` yang dikirim ke provider pada langkah berikutnya. Ini menjaga
invariant "turn yang benar-benar dikirim utuh di state.db" dan "satu turn tidak
pernah terpecah".

## Kriteria

- [ ] Runner menyiapkan `turns_to_send(&self, context_length: Option<u64>) ->
      Vec<Turn>` yang menjatuhkan turn tertua (User/Assistant/Tool) dari
      salinan sampai `estimate_turns_tokens` <= limit, dengan aturan konservatif:
      jangan jatuhkan turn yang sedang jadi pertanyaan aktif terakhir, dan
      hormati kumpulan turn yang di-pin (04) sebagai tidak-bisa-didrop.
- [ ] Sumber limit jelas: dipakai `model.context_length`, atau `target_max_tokens`
      dari `compression` bila lebih relevan (lihat 05). Absen (`None`) => tanpa
      window (kirim penuh, backward compatible).
- [ ] Turn yang di-drop TIDAK dihapus dari `self.turns`/`state.db`; hanya tidak
      ikut dalam kiriman.
- [ ] Test: percakapan panjang (mis. ~100 turn) tetap terkirim dalam batas
      `estimate_turns_tokens <= limit`; turn terakhir + system tetap hadir.
- [ ] Ambang (berapa turn / berapa token dipertahankan) dieja eksplisit di test.

## STRIDE

- **Data integrity:** tidak ada penghapusan turn canonical; hanya pemangkasan
  salinan. Test memastikan state.db utuh setelah window aktif.
- Tidak ada surface credential/eksekusi baru.

## Risiko

- Menjatuhkan turn User tanpa Assistant berpasangan bisa membuat konteks tak
  koheren; window harus drop berpasangan (User+Assistant/Tool) sejauh mungkin.
- Interaksi pin (04): turn di-pin tidak boleh terdorong keluar.

## Dependency

01.
