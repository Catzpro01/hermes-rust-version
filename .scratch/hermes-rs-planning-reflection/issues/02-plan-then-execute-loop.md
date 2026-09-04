# 02: Plan-then-execute loop

**What to build:** Fase perencanaan sebelum tool pertama: pada mode terencana,
agent membuat rencana langkah terstruktur (teks model) terhadap goal (tiket 01),
lalu mengeksekusinya — bukan langsung reaktif. `/plan` menampilkan rencana aktif.

**Blocked by:** 01.

**Status:** breakdown (belum implementasi).

## Kondisi sekarang (terverifikasi)

- `chat_agentic` memanggil provider dengan seluruh konteks (window) dan langsung
  menafsirkan tool calls. Ia tidak pernah meminta model menyusun langkah dulu;
  tidak ada representasi "rencana".
- `Turn` hanya `User|Assistant|Tool`; tidak ada `Turn::Plan`. Menambah varian
  `Turn` memaksa update tiap `match` (`session/store.rs`, `provider/http.rs`,
  `provider/fake.rs`, `conversation/context.rs::turn_tokens`) dan berisiko
  menabrak ADR 0003 (role baru perlu ADR). Karena itu Plan default **in-memory**:
  teks model pada sebuah iterasi `Assistant` disimpan ke field runner
  (bukan role db baru), opsional ditampilkan via `/plan`.
- Provider streaming tak membedakan "ini rencana" vs "ini jawaban"; kita perlu
  meminta format terdelimitasi (mis. tag `[[plan]]…[[/plan]]`) dan mem-parse
  sebelum tool call, menyerupai `parser::parse_tool_events` (Spec 002).

## Konsep

- Aktivasi mode terencana: `/plan on` (atau deteksi tugas kompleks — putuskan di
  sini). Saat aktif, sebelum tool call pertama runner melakukan **satu
  round-trip** meminta model menghasilkan rencana (konsumsi 1 iterasi dari
  batas 10), parse langkah, simpan in-memory, tampilkan via `/plan`.
- Jalur eksekusi tetap memakai iterasi tersisa; `MaxIterations` bila rencana
  menyisakan langkah dan budget habis.
- Rencana **dihitung dalam token budget** (`turn_tokens`/`estimate_turns_tokens`);
  window Spec 008 tetap melindungi turn terbaru + pin. Bila plan membuat konteks
  melebihi `context_limit` → warning, tidak drop.
- Backward compatible: mode off → perilaku persis sekarang.

## Kriteria

- [ ] `/plan on`/`/plan` mengaktifkan & menampilkan rencana; `/plan off`
      kembali reaktif (regresi nol saat off).
- [ ] Rencana disimpan in-memory (bukan varian `Turn`/role db baru); tidak ada
      fake `User` turn; pembaca `state.db` lama tak terpengaruh.
- [ ] Parse rencana berformat delimited deterministik & char-safe (uji dengan
      CJK).
- [ ] Rencana aktif dihitung terhadap `context_limit`; window tetap berlaku.
- [ ] Iterasi tetap ≤ 10; plan + eksekusi berbagi budget itu.
- [ ] Test unit + integration (mode on/off, parse, token); suite hijau;
      clippy bersih.

## STRIDE

- **Prompt-injection:** teks rencana adalah output model (bukan input user
  mentah yang di-execute). Plan tidak dieksekusi sebagai perintah — hanya
  langkah tujuan yang memicu tool via jalur eksekusi normal Spec 002.
- Tidak ada surface baru.

## Risiko

- Plan menghabiskan iterasi — batasi ukuran/format; test mem-pin jumlah
  langkah.
- Ambigu representasi bila suatu hari plan perlu persist → ADR analog 0003
  sebelum dipersist.

## Dependency

01.
