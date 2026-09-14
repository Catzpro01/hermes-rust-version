# Agent support — mulai di sini

Folder ini menyimpan **materi pendukung agent**, terpisah dari kode Hermes dan
bukti produk. Ini adalah paket kesinambungan kerja, bukan bagian runtime Hermes.

## Urutan baca tercepat

1. **[CURRENT.md](handoff/CURRENT.md)** — sedang mengerjakan apa dan langkah berikutnya.
2. **[RULES.md](RULES.md)** — aturan keselamatan, branch, verifikasi, commit/push.
3. **[START-NEXT-SESSION.md](handoff/START-NEXT-SESSION.md)** — panduan pindah sesi.
4. [Memory](memory/MEMORY.md) dan bagian terakhir [progress](memory/PROGRESS.md).
5. Skill/tiket/laporan yang ditunjuk, bukan seluruh arsip sekaligus.

## Peta folder

| Lokasi | Isi |
|---|---|
| `handoff/` | Status aktif, keputusan penting, cara melanjutkan, integrasi main |
| `memory/` | Ingatan kerja dan log progres kronologis lengkap yang sudah dicatat |
| `context/` | Konteks/glosarium proyek yang diwarisi |
| `guidance/` | Panduan tracker/domain, integrasi skill, kendala Actions, skill lokal |
| `guidance/vendor/` | Sumber Matt Pocock asli + lock/hash; jangan diedit |
| `skills/` | 37 tautan ke skill asli sesuai kategori upstream |
| `planning/` | Tiket setup skill/dukungan agent, bukan implementasi runtime |
| `automation/` | Verifikasi paket, auto-push setelah commit, tes dan bootstrap |

**Tetap pada lokasi proyek:** `crates/`, `specs/`, `docs/adr/`, `docs/PARITY.md`,
`.scratch/hermes-rs-total-parity/`, `scripts/`, dan bukti visual
`docs/hermes-ui-spec/017/evidence/`. Workflow dan tes masih memakai lokasi ini.
[Milestone visual](../docs/hermes-ui-spec/017/MILESTONES.md) tetap dokumen produk.

## Kompatibilitas penemuan agent

`AGENTS.md` di root adalah file masuk biasa. Path lama MEMORY/PROGRESS/CONTEXT,
`docs/agents`, `.agents/skills` dan `.scratch/hermes-rs-agent-skills` adalah tautan
ke sumber tunggal di folder ini. Jangan membuat salinan yang kemudian berbeda.
Pada clone tanpa dukungan symlink, baca path kanonik di sini; verifikasi akan
melaporkan alias yang tidak berfungsi. Linux/Git checkout normal mendukungnya.

## Apa yang disimpan / tidak disimpan

Disimpan: keputusan pengguna, scope, kemajuan, hasil verifikasi, hash/run/commit,
skill asli, batasan, masalah yang pernah ditemui, dan petunjuk melanjutkan.
Bukti produk lama tetap utuh; log panjang dirujuk, bukan disalin berkali-kali.

Ini **bukan ekspor mentah seluruh chat/context window**. Tidak ada credential,
transkrip pribadi, state internal yang tidak tersedia, cache, binary build,
node_modules, atau proses hidup yang dijanjikan akan berpindah ke sesi berikutnya.

Status source/bukti tidak sama dengan status acceptance. Spec017 masih terbuka.
Permintaan simpan di main belum dapat dijalankan dari sesi yang branch-nya dikunci;
lihat [integrasi main](handoff/MAIN-INTEGRATION.md).
