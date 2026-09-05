# 03: Self-reflection gate

**What to build:** Setelah tiap hasil tool, nilai apakah hasil sesuai harapan
terhadap plan (02) & goal (01); jika tidak → tandai langkah utk di-replan
(diserap 04), jika ya → lanjut. Batas refleksi per langkah agar tak infinite
reflect.

**Blocked by:** 02.

**Status:** done — commit di VM, 233 test hijau, clippy clean.

## Kondisi sekarang (terverifikasi)

- Setelah `ToolRegistry::execute`, `chat_agentic` mendorong `Turn::Tool` dan
  membiarkan iterasi berikutnya. Tidak ada penilaian apakah hasil cocok dgn
  harapan; model hanya melihat teks hasil (termasuk error) dan bisa saja
  terus mengulang langkah yang sama sampai `MaxIterations`.
- `ToolResponse { success: bool, content }` & `ToolExecutionStatus` sudah ada;
  status gagal (Error/Denied/Timeout) tersedia saat refleksi, tapi belum
  dipakai untuk keputusan struktural.

## Konsep

- `ReflectionGate`: setelah `Turn::Tool` dengan status non-success (atau sukses
  tapi belum selesai sesuai plan), runner menilai: (a) apakah masih on-plan, (b)
  apakah harus mengulang dgn pendekatan berbeda (→ 04), (c) apakah goal sudah
  tercapai (→ 01, status `Achieved`). Penilaian default **heuristik +
  round-trip LLM opsional**; putuskan di sini agar deterministik & teruji.
- Anti-loop: batas refleksi per langkah/plan step (mis. `MAX_REFLECTIONS`),
  setelah itu `MaxIterations` atau tandai `Blocked` — jangan reflect tanpa
  batas.
- Refleksi memakai iterasi dari budget 10 yang sama.

## Kriteria

- [x] Setelah tool result non-success, refleksi menilai on/off-plan dan
      menandai langkah untuk recovery (04) — tidak sekadar meneruskan teks.
- [x] Batas refleksi per langkah di-enforce (uji: loop refleksi tak berlanjut
      selamanya; `MaxIterations`/`Blocked` keluar).
- [~] Goal yang tercapai memicu status `Achieved` (tiket 01) dan menghentikan
      loop lebih awal.
- [x] Mode reaktif (tanpa plan) → refleksi tak mengubah perilaku (regresi nol).
- [x] Token refleksi dihitung dlm budget; window/pin Spec 008 tetap berlaku.
- [x] Test + clippy hijau.

## STRIDE

- Keputusan refleksi murni di layer conversation; tidak menambah eksekusi.
- Tidak ada fake User/role baru di db.

## Risiko

- Refleksi menambah latensi/iterasi — batas eksplisit & uji anti-loop wajib.

## Dependency

02 (membutuhkan rencana utk dinilai); goal tracking 01 untuk `Achieved`.

## Catatan cakupan implementasi (Ticket 03)
- Refleksi default = **heuristik deterministik** (`verdict`: OnPlan/OffPlan/Blocked
  berdasar status tool + sisa retry), bukan LLM. `Denied` selalu Blocked (tak
  pernah di-retry). Anti-loop `MAX_REFLECTIONS=2`.
- Wire: `chat_agentic` memanggil `reflect_tool_outcome(status, retries_remaining)`
  tiap hasil tool (no-op saat reflection off → reactive identik). Goal diblokir
  saat Denied/exhaust; status `Achieved` di-set manual (`/goal achieved`).
- **Belum diimplementasikan (menyusul Ticket 04 orchestration):**
  - *early-stop* loop saat goal `Achieved` di tengah eksekusi (orchestration).
  - round-trip LLM reflection utuh (instruksi `reflect_instruction` & helper
    `needs_llm_reflection` tersedia; method runner utuh ada di 04/05).
