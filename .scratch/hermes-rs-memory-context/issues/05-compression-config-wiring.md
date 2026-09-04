# 05: Wiring config compression + context

**What to build:** Baca `compression.enabled` / `compression.target_max_tokens`
(dan `context_length`) dari `config.yaml`, lalu aktifkan sliding window &
summarization (02/03) hanya bila diaktifkan konfigurasi. Backward compatible:
default off sehingga konfigurasi lama yang tak punya `compression` berperilaku
seperti sekarang.

**Blocked by:** 01 (accounting), 02 (window shape).

**Status:** todo

## Kondisi sekarang (terverifikasi)

- `HermesConfig.compression: Option<CompressionConfig>` sudah diparse, dengan
  `enabled: Option<bool>` & `target_max_tokens: Option<u64>` — tapi tidak
  dibaca untuk perilaku.
- `ModelConfig.context_length: Option<u64>` & `ProviderConfig.context_length`
  diparse, tidak dibaca.
- Runner dibuat di `crates/hermes-cli/src/repl.rs` (`ConversationRunner::from_turns`),
  yang punya akses `config: Option<HermesConfig>`.

## Konsep

Resolusi batas yang jelas, terdokumentasi (prioritas diputuskan di sini):
provider aktif `context_length` > `model.context_length` >
`compression.target_max_tokens`. `compression.enabled == Some(false)` atau
ketiadaan semua batas => matikan window (kirim penuh). REPL meneruskan
`Option<u64>` batas (dan switch window on/off) ke runner saat konstruksi.

## Kriteria

- [ ] REPL membaca `config.compression`/`context_length` dan meneruskan
      batas + flag aktif ke `ConversationRunner` (via builder/setter, bukan
      global).
- [ ] Precedence batas eksplisit & di-test (provider.context_length >
      model.context_length > compression.target_max_tokens).
- [ ] Default: tak ada `compression`/`context_length` => window off, perilaku
      identik dgn sekarang (regresi nol).
- [ ] `enabled:false` memaksa window off meski `target_max_tokens` ada.
- [ ] Test parsing + perilaku aktif/off + precedence (angka di-pin).
- [ ] Tidak menambah dependency; seluruh perilaku di jalankan lewat helper
      Spec 006 #04.

## STRIDE

- Tidak ada surface credential/eksekusi baru. Membaca config sudah lewat jalur
  yang ada.
- Kejelasan: keputusan "off karena tak dikonfigurasi" vs "off karena
  enabled:false" bisa dibedakan (untuk test), bukan disatukan.

## Risiko

- Precedence yang ambigu bila `compression` dan `context_length` sama-sama ada —
  selesaikan & dokumentasikan di tiket ini (test memin-nya).

## Dependency

01, 02.
