# 04: `/provider <name>` — switch mid-session

**What to build:** Perintah REPL untuk mengganti provider tanpa kehilangan session yang sedang berjalan.

**Blocked by:** 01 — Provider registry.

**Status:** todo

## Kondisi sekarang (terverifikasi)

`repl::run_repl(&home, provider, args.resume)` menerima **satu**
`Box<dyn Provider>` yang diikat saat startup. Tidak ada mekanisme penggantian.
Perintah yang ada sekarang:

```
/new, /sessions, /inspect <id>, /messages <id>, /tool-calls <id>, /search <query>, /resume <id>, /exit
```

## Kriteria

- [ ] `/provider` tanpa argumen mendaftar provider yang tersedia dan menandai yang aktif.
- [ ] `/provider <name>` mengganti provider aktif; nama tak dikenal menghasilkan error tanpa mengubah provider aktif.
- [ ] Session dan riwayat `Turn` **tidak** terpengaruh oleh penggantian provider.
- [ ] Provider baru tidak menyimpan state apa pun yang diwarisi dari provider lama.
- [ ] SIGINT di tengah stream tetap keluar 130 setelah penggantian provider.
- [ ] Gagal inisialisasi provider (mis. `key_env` kosong) **tidak** merusak provider yang sedang aktif — rollback, bukan setengah jalan.

## Batas yang harus ditegakkan

Invariant project: *partial turn tidak pernah disimpan saat cancellation*.
Penggantian provider hanya boleh terjadi **di batas turn**. Kalau pengguna
menjalankan `/provider` saat stream aktif, putuskan dan dokumentasikan:
tolak perintahnya, atau batalkan turn lebih dulu lalu ganti. Jangan biarkan
satu turn tercatat berasal dari dua provider.

`state.db` tetap satu-satunya canonical storage — jangan simpan provider aktif
di tempat lain.
