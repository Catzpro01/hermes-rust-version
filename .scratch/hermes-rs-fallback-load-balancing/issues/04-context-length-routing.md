# 04: Kesadaran batas konteks

**What to build:** Deteksi apakah total permintaan (konteks percakapan) muat di
model/provider target, dan gunakan itu untuk memilih hop / memperingatkan —
mencegah kirim konteks yang melebihi `context_length` provider.

**Blocked by:** 01, 03.

**Status:** todo

## Kondisi sekarang (terverifikasi)

`ProviderConfig.context_length: Option<u64>` dan `ModelConfig.context_length:
Option<u64>` sudah diparse tetapi tidak dibaca untuk routing (lihat
`crates/hermes-core/src/config/schema.rs`). `models: HashMap<String,
serde_yaml::Value>` menyimpan nilai per-model bebas; tidak ada perhitungan token
dari `turns`.

## Kriteria

- [ ] Estimasi panjang konteks dari `&[Turn]` (mis. penghitung char/token
      deterministik; tidak perlu tokenizer pihak ketiga bila cukup deterministik
      dan terdokumentasi sebagai estimasi).
- [ ] `context_length` per provider/model dibaca dan dibandingkan dengan estimasi
      sebelum memilih hop dalam rantai fallback / sebelum memilih model default.
- [ ] Jika konteks diperkirakan melebihi `context_length`, provider tersebut
      dilewati (bila ada alternatif) atau menghasilkan error yang jelas lebih
      awal — bukan gagal misterius di tengah.
- [ ] Ambang perhitungan disebut eksplisit di test (ubah konstanta → test
      patah), sesuai pelajaran Spec 004/005.
- [ ] Tidak ada regresi pada `chat_completions`/`completions` (Spec 005) — mode
      tetap dipilih dari `api_mode`.

## Catatan review (anti over-engineering)

`models` adalah `HashMap<String, serde_yaml::Value>` bebas dan tidak ada
tokenizer di codebase; estimasi char-based kasar untuk konten multilingual.
Implementasi sebagai **warning/advisory**, bukan hard routing decision:

- Heuristik konservatif: `char_count / 4` sebagai estimasi token.
- Jika `context_length` absent (`None`), **skip** check (backward compatible).
- Jangan blokir request hanya karena estimasi — log warning dan lanjutkan.
- Jika ternyata terlalu kompleks, boleh di-descope ke Spec 008 (Memory &
  Context Management) dan tiket ini disesuaikan.

## STRIDE

- Tidak ada surface credential/eksekusi baru; ini murni keputusan routing +
  estimasi.
- **DoS (ringan):** mencegah kirim payload raksasa ke endpoint yang tak sanggup.

## Risiko

- Estimasi token deterministik ≠ tokenizer asli; harus jelas ini "estimasi
  konservatif" dan didokumentasikan, agar tidak over-claim parity dengan Python.
- Interaksi: apakah context-length memilih model/provider, atau hanya
  memperingatkan? Keputusan: ikut memilih hop di fallback, dan warning saat
  `--provider` eksplisit melebihi.

## Dependency

01 (error taxonomy untuk error "context too long" bila provider mengembalikan
400), 03 (routing hop).
