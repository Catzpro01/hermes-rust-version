# 04: Kesadaran batas konteks

**What to build:** Deteksi apakah total permintaan (konteks percakapan) muat di
model/provider target, dan gunakan itu untuk memilih hop / memperingatkan —
mencegah kirim konteks yang melebihi `context_length` provider.

**Blocked by:** 01, 03.

**Status:** done — commit di VM, 155/155 test hijau (`cargo test --workspace`),
`clippy --workspace --all-targets -D warnings` bersih.

## Kondisi sekarang (terverifikasi)

`ProviderConfig.context_length: Option<u64>` dan `ModelConfig.context_length:
Option<u64>` sudah diparse tetapi tidak dibaca untuk routing (lihat
`crates/hermes-core/src/config/schema.rs`). `models: HashMap<String,
serde_yaml::Value>` menyimpan nilai per-model bebas; tidak ada perhitungan token
dari `turns`.

## Keputusan desain (dikunci /ask-matt — Opsi 2)

Implementasi sebagai **helper murni + unit test di `hermes-core`**, BELUM di-wire
ke alur kirim. Estimasi char/4 terlalu kasar untuk dipakai sebagai warning REPL
yang aktif (risiko false-positive mengganggu user), dan descope penuh terlalu
pasif karena `context_length` sudah tersedia. Fondasi deterministik dibangun
sekarang agar siap saat Spec 008 (Memory & context). `context_length` tidak
memicu pemilihan model/provider; murni advisory untuk konsumsi nanti.

## Kriteria

- [x] Estimasi panjang konteks dari `&[Turn]` (penghitung token konservatif).
- [x] `context_length` dibaca dan dibandingkan dengan estimasi lewat fungsi
      advisory `check_context_limit(&[Turn], Option<u64>)`.
- [x] Jika konteks diperkirakan melebihi `context_length`, fungsi mengembalikan
      pesan peringatan (non-blocking). Pemanggil bebas memutuskan apakah
      memperingatkan/menindak — helper tidak pernah menolak/memblokir.
- [x] Ambang pembagian (`text.len() / 4`) diekspos eksplisit & di-pin test
      (ubah konstanta → test berubah).
- [x] Tidak ada regresi pada `chat_completions`/`completions` (Spec 005).

## Catatan review (anti over-engineering) — dipatuhi

- Heuristik konservatif: `text.len() / 4` sebagai estimasi token.
- Jika `context_length` absent (`None`), **skip** check (backward compatible).
- Tidak memblokir request; murni advisory.
- Tidak di-descope: fondasi dibangun (lihat keputusan desain di atas).

## STRIDE

- Tidak ada surface credential/eksekusi baru; murni helper + unit test.
- **DoS (ringan):** helper menjadi dasar untuk mencegah kirim payload raksasa
  ke endpoint yang tak sanggup (di-wire pada tiket lanjutan bila diperlukan).

## Pengujian (di atas 147 → 155, +8)

- Unit `conversation/context.rs`:
  - `divisor_is_exactly_four`: pin konstanta `len/4` (40→10), fragment < 1 tetap
    `max(1)`.
  - `turns_sum_content_and_tool_name`, `tool_turn_counts_name_and_content`,
    `empty_turns_are_zero_tokens`.
  - `none_limit_skips_the_check_entirely` (backward compatible).
  - `over_limit_yields_a_warning_naming_both_numbers`, `within_limit_yields_no_warning`,
    `equal_to_limit_is_not_over`.

## Perubahan

- `crates/hermes-core/src/conversation/context.rs` (baru): `estimate_tokens`
  (`text.len()/4`, min 1), `estimate_turns_tokens`, `check_context_limit`.
- `crates/hermes-core/src/conversation/mod.rs`: deklarasi `pub mod context;`.

## Catatan desain

- Estimasi token deterministik ≠ tokenizer asli; hanya penasihat. Divisor `4`
  adalah konstanta eksplisit yang di-pin test.
- Belum ada wiring ke REPL/runner (`model.context_length`/`providers[*]` dipakai
  saat Spec 008 memutuskan UX-nya) — ini sengaja, mengikuti keputusan /ask-matt.
- Error "context too long" (400 dari provider, Ticket 01) tetap jadi jalur
  terpisah; helper ini tidak menanganinya.

## Dependency

01, 03.
