# 02: Plan-then-execute loop

**What to build:** Fase perencanaan sebelum tool pertama: pada mode terencana,
agent membuat rencana langkah terstruktur (teks model) terhadap goal (tiket 01),
lalu mengeksekusinya — bukan langsung reaktif. `/plan` menampilkan rencana aktif.

**Blocked by:** 01.

**Status:** done — commit di VM, 223 test hijau, clippy clean.

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

- [x] `/plan on`/`/plan` mengaktifkan & menampilkan rencana; `/plan off`
      kembali reaktif (regresi nol saat off).
- [x] Rencana disimpan in-memory (bukan varian `Turn`/role db baru); tidak ada
      fake `User` turn; pembaca `state.db` lama tak terpengaruh.
- [x] Parse rencana berformat delimited deterministik & char-safe (uji dengan
      CJK).
- [x] Rencana aktif dihitung terhadap `context_limit`; window tetap berlaku.
- [x] Iterasi tetap ≤ 10; plan + eksekusi berbagi budget itu.
- [x] Test unit + integration (mode on/off, parse, token); suite hijau;
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

## Keputusan implementasi (Opsi A — kanal instruksi ephemeral)

Implementasi mengikuti keputusan /ask-matt Opsi A (scope ketat):

- `Provider::chat_with_instruction(&self, turns, instruction: Option<&str>, cancel)`
  ditambah **dengan default impl** yang mendelegasi ke `chat_with_cancel`
  (backward compatible; `None` identik). `Box<T>` meneruskan.
- `HttpProvider` override: di `chat_completions` instruksi disisipkan sbg pesan
  `system` di depan (via `build_chat_messages`); di `completions` sbg header
  `[Instruction] …` (via `render_completions_prompt_with_instruction`). Kanal
  tetap tool-aware (plan/eksekusi boleh memanggil tool).
- `FallbackProvider` meneruskan instruksi ke tiap hop.
- Instruksi **internal-only** (konstanta `PLAN_INSTRUCTION`), tidak pernah dari
  input user, tidak dipersist, tidak jadi `Turn`/role db.
- Parser `[[plan]]…[[/plan]]` di `conversation/plan.rs` (bracket ganda, bebas
  konflik dgn XML `<tool_call>`), char-safe (uji CJK).
- Runner: field `plan_mode` + `plan: Option<Plan>` in-memory; `ensure_plan`
  melakukan 1 round-trip instruksi (sharing budget: plan round memakai 1 dari
  `max_iters`); `estimated_tokens` menambah `plan.tokens()`.
- REPL: `/plan [on|off|reset]`, `/plan` menampilkan mode + langkah (disanitasi).
- ADR 0004 mengunci scope kanal instruksi (bukan aktivasi system-prompt penuh).

### Batasan yang didokumentasikan
- Window trimming Spec 008 tetap berbasis turn (`self.turns`); plan adalah
  beban ephemeral yg ditambahkan ke estimasi/warning, bukan subset window.
- Eksekusi menyuplai ulang plan sbg instruksi ephemeral tiap kirim; adherence
  penuh (reflection/recovery) ada di Ticket 03/04.
