# 03: Summarization turn yang di-drop

**What to build:** Alih-alih membuang turn tertua mentah-mentah saat window
menyempit, kompres turn yang dikeluarkan menjadi ringkasan singkat yang
disisipkan sebagai konteks prefix, sehingga thread panjang tetap "mengingat"
inti percakapan lama tanpa melampaui batas token.

**Blocked by:** 02 (sliding window menentukan turn mana yang dikeluarkan).

**Status:** todo

## Kondisi sekarang (terverifikasi)

- Ticket 02 akan memilih turn tertua untuk dikeluarkan dari salinan kirim.
- Tidak ada representasi ringkasan; turn yang di-drop hilang dari konteks aktif
  (walau tetap utuh di `state.db`).

## Konsep

Saat window memangkas N turn tertua, buat satu `Turn::User`/pesan ringkas (atau
sistem note) berisi inti dari turn yang dikeluarkan, lalu simpan sebagai elemen
awal salinan yang dikirim. Dua strategi dipertimbangkan, yang **pilih dipecah
menjadi keputusan eksplisit**:

- **Heuristik (default, tanpa dependency):** ekstrak pesan User & hasil
  Tool terakhir dari blok yang dikeluarkan menjadi ringkasan 1–3 kalimat
  deterministik (mis. role + dua/tiga pesan terakhir), cukup untuk kontinuitas.
- **LLM-recursive (opt-in):** panggil provider untuk meringkas blok yang
  dikeluarkan. Berisiko (recursive call, biaya, latensi) — jadikan opsi yang
  di-disable default.

Ringkasan harus disimpan sedemikian rupa sehingga state.db tetap canonical dan
ringkasan tidak disalahartikan sebagai pesan asli pengguna.

## Kriteria

- [ ] Fungsi `summarize_dropped(dropped: &[Turn]) -> Turn` (heuristik) tersedia
      & deterministik; dipakai saat window memangkas.
- [ ] Ringkasan disisipkan sebagai elemen prefix pada salinan kirim, tidak masuk
      `self.turns` canonical dan tidak disimpan sebagai pesan User palsu.
- [ ] Ukuran ringkasan token dibatasi eksplisit (pastikan tidak meniadakan
      penghematan window).
- [ ] Mode LLM-recursive bila ada di-disable-default & dibatasi (max 1 call,
      error tidak memblokir — jatuh ke heuristik).
- [ ] Test: blok yang di-drop → ringkasan mengandung inti (User terakhir/role);
      token ringkasan << token blok asli.
- [ ] Tidak regresi: window + summarization tetap <= limit.

## STRIDE

- **Integrity/prompt-injection:** ringkasan harus diberi peran eksplisit
  ("ringkasan percakapan lama"), bukan disuntik sebagai User asli, agar model
  tak mengira instruksi di dalamnya berasal dari pengguna.
- Tidak ada surface eksekusi baru (heuristik); mode LLM = jalur network yang
  sudah ada.

## Risiko

- Ringkasan heuristik kehilangan nuansa; dokumentasikan sebagai kompromi.
- Recursive LLM summarization = loop tak terbatas bila tak di-batas; hard-cap.

## Dependency

02.
