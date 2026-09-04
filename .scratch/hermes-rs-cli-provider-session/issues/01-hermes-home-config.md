# 01: Hermes home dan konfigurasi kompatibel

**What to build:** Hermes-RS menemukan Hermes home, membaca konfigurasi secara read-only, menerapkan override eksplisit, dan memberikan error aman untuk konfigurasi invalid atau provider yang belum dikonfigurasi.

**Blocked by:** None (can start immediately).

**Status:** done

- [x] Hermes home dapat ditemukan dari default dan override eksplisit.
- [x] Konfigurasi provider/model dapat dibaca tanpa mengubah instalasi Python Hermes.
- [x] Precedence override terdokumentasi dan teruji.
- [x] Missing, malformed, dan unknown provider menghasilkan error actionable tanpa secret.
- [x] Pengujian memakai Hermes home disposable.
