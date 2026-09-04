# 03: Rantai fallback antar provider

**What to build:** Ketika provider aktif gagal permanen (atau kehabisan retry),
Hermes-RS otomatis mencoba provider berikutnya dalam rantai fallback yang
terkonfigurasi — tanpa mengubah `ConversationRunner`/`repl`.

**Blocked by:** 01, 02.

**Status:** todo

## Kondisi sekarang (terverifikasi)

`ConversationRunner<P: Provider>` memegang satu `provider: P` (spec 005).
`Provider` adalah trait async. `ProviderRegistry::build/select` membangun satu
provider. Tidak ada konsep "daftar provider dicoba berurutan".

## Konsep

Fallback paling bersih dibangun sebagai **wrapper `Provider`** baru (mis.
`FallbackProvider`) yang memegang daftar provider (berurutan) dan men-delegasi
ke masing-masing: coba #1 (dengan retry tiket 02), kalau gagal non-retry-able /
kehabisan retry, coba #2, dst. `ConversationRunner` dan REPL tidak perlu tahu —
mereka tetap melihat satu `Box<dyn Provider>`.

## Kriteria

- [ ] `config.yaml` mendukung mendefinisikan rantai fallback (mis. field
      `fallback_chain: [b, c]` pada provider, atau sebuah aggregator `provider`
      dengan daftar). Skema ketat seperti `api_mode`; nilai tak dikenal ditolak
      saat load, bukan saat request.
- [ ] Wrapper fallback mengimplementasikan `Provider`, jadi transparan bagi
      `ConversationRunner`/REPL `/provider`.
- [ ] Setiap provider dalam rantai memakai credentialnya sendiri
      (`key_env`/`model.api_key`) — **tidak ada** credential silang (sesuai
      STRIDE Spec 005).
- [ ] Provider yang gagal diinisialisasi dilewati dengan bersih; rantai terus
      ke yang berikutnya.
- [ ] Jika seluruh rantai gagal, error terakhir (atau agregat yang jelas)
      diteruskan; tidak ada partial state.
- [ ] Fallback mencoba provider berikutnya dengan **turns yang sama dari awal**
      (belum ada output yang dihasilkan karena kita hanya fallback pada error
      pra-stream). Keputusan di-dokumentasikan.
- [ ] Semua anggota rantai memakai `SecretString`/redaction yang sama.

## STRIDE

- **Spoofing/credential confusion:** rantai fallback adalah area rawan baru.
  Setiap hop WAJIB memakai key provider hop itu sendiri. Diuji dengan wiremock
  dua provider (pola `provider_routing_e2e.rs` dari Spec 005).
- **Information disclosure:** error agregat harus menyebut nama provider, bukan
  nilai key.
- Tidak ada surface eksekusi baru.

## Risiko

- Menentukan "kegagalan hop" harus konsisten: hanya error permanen / habis
  retry yang memicu hop, bukan `Cancelled`.
- Interaksi dengan `/provider` (spec 005): fallback dan switch manual harus
  jelas; switching manual mengganti "aktif", fallback adalah lapisan di bawahnya.

## Dependency

01, 02.
