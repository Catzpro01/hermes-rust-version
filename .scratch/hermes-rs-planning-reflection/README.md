# Spec 009 — Multi-Turn Planning & Reflection

Vertical slice: mengubah `chat_agentic()` yang saat ini **reaktif** menjadi
agent yang bisa **merencanakan sebelum bertindak**, **merefleksikan hasil tool**
setelah eksekusi, **memulihkan diri dari error tool** dengan pendekatan
berbeda, dan **melacak progres terhadap goal** user. Slice ini bekerja di
layer conversation/LLM saja — **tidak ada execution surface baru** (tidak ada
tool/shell baru), selaras dengan spec security boundary yang ada.

## Motivasi (terverifikasi dari kode)

- `ConversationRunner::chat_agentic` (`crates/hermes-core/src/conversation/mod.rs`)
  reaktif: dorong `Turn::User` → loop `1..=max_iters` (10, Spec 002) → tiap
  iterasi hitung ulang window (`turns_to_send`) → kirim ke provider → kumpulkan
  `Event::Chunk`/`ToolCall`. Jika tidak ada tool call → `push_assistant` +
  `Done`. Jika ada → eksekusi tiap call via `ToolRegistry::execute`, petakan
  `ToolError` ke `(content, status)`, dorong `Turn::Tool`, simpan
  `ToolCallRecord` (bila ada `store_ctx`). Tidak ada fase "pikir dulu",
  penilaian hasil, atau deteksi tujuan tercapai.
- `Turn` hanya punya `User | Assistant | Tool { name, content }`. Tidak ada
  `Plan`/`Reflection`/`System`. Invariant Spec 008: **tidak boleh ada fake
  `User` turn** dan ADR 0003 menegaskan representasi ringkasan tidak diinjek
  tanpa ADR terpisah. Plan/Reflection default **in-memory** (bukan role baru di
  `state.db`) agar tidak melanggar invariant itu.
- Error tool diwakili `ToolExecutionStatus` (`Success|Error|Denied|Timeout|Cancelled`)
  dan `ToolError` (`Unknown|Cancelled|Timeout|Denied|Failed|InvalidXml`) di
  `crates/hermes-core/src/tools/mod.rs`. Saat ini model hanya melihat hasil
  error sebagai teks; ia bisa saja mengulang parameter yang sama sampai `max_iters`
  habis (loop tak produktif).
- Akuntansi token siap: `estimate_tokens` (char/4), `turn_tokens`, `estimate_turns_tokens`,
  `check_context_limit` di `conversation/context.rs`; window + pin dari Spec 008
  memakai `turn_tokens`. Plan/Reflection harus dihitung terhadap `context_limit`.
- Belum ada wiring system-prompt (ADR 0003 mencatat aktivasi system prompt
  terpisah). Instruksi plan/reflect dikirim sebagai bagian dari permintaan ke
  provider (teks model), bukan sebagai pesan user palsu yang dipersist.

## Keputusan scope awal (untuk direview)

- **Opt-in / backward compatible.** Jika permintaan user tidak meminta
  perencanaan (atau feature belum diaktifkan), `chat_agentic` berperilaku
  persis seperti sekarang (reaktif). Planning/reflection adalah jalur yang
  diaktifkan (mis. `/plan on` atau heuristik deteksi tugas kompleks — keputusan
  di tiket 02).
- **Representasi Plan/Reflection = in-memory + teks model** pada iterasi
  terpisah, BUKAN varian `Turn` baru yang dipersist ke `state.db`, kecuali ADR
  analog 0003 disetujui dulu. Ini menjaga: tidak ada fake User turn, tidak ada
  role baru di db tanpa ADR, dan pembaca lama tetap kompatibel.
- **Iterasi tetap ≤ 10 (Spec 002).** Planning/reflection memakai iterasi yang
  sama; tidak menambah batas. Tiap round-trip LLM (plan, reflect) adalah satu
  iterasi — trade-off ini dieja eksplisit di tiket.
- **Token budget.** Teks plan & refleksi ditambahkan ke konteks dan dihitung
  via `estimate_turns_tokens`/`turn_tokens`; window tetap melindungi turn
  terbaru + pin (Spec 008). Bila plan menjadikan konteks melebihi limit,
  muncul warning (tidak di-drop plan yang sedang aktif).
- **Tidak ada tool/shell/execution baru.** Menghormati invariant ROADMAP dan
  STRIDE Spec 002/006.
- **`state.db` tetap canonical** hanya untuk turn yang benar-benar dikirim;
  state plan/reflection in-memory (opsional, sub-tiket terpisah bila ingin
  persist).

## Tiket

| # | Tiket | Blocked by |
|---|---|---|
| 01 | [Goal extraction & tracking](issues/01-goal-extraction-tracking.md) | — |
| 02 | [Plan-then-execute loop](issues/02-plan-then-execute-loop.md) | 01 |
| 03 | [Self-reflection gate](issues/03-self-reflection-gate.md) | 02 |
| 04 | [Error recovery dengan parameter mutation](issues/04-error-recovery-parameter-mutation.md) | 03 |
| 05 | [Parity, docs, penutupan](issues/05-parity-docs-closure.md) | 01–04 |

01 memodelkan goal sebagai fondasi yang tak tergantung; 02 memakai goal utk
membentuk plan; 03 menilai tiap hasil tool terhadap plan/goal; 04 membuat
recovery termutasi-param (bukan ulang parameter sama); 05 penutupan.

## Invariant yang tetap berlaku

Semua invariant `docs/ROADMAP.md` tetap: `state.db` canonical untuk turn yang
dikirim; SIGINT exit 130; credential terredaksi; turn yang dibatalkan tidak
dipersist parsial; tidak ada fake `User` turn; plan/reflection tidak menjadi
role baru di db tanpa ADR. Window Spec 008 (pinned ∪ terbaru selalu dikirim)
tetap berlaku.
