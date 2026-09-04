# 01: Hermes home dan konfigurasi kompatibel

**What to build:** Hermes-RS menemukan Hermes home, membaca konfigurasi secara read-only, menerapkan override eksplisit, dan memberikan error aman untuk konfigurasi invalid atau provider yang belum dikonfigurasi.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] Hermes home dapat ditemukan dari default dan override eksplisit.
- [ ] Konfigurasi provider/model dapat dibaca tanpa mengubah instalasi Python Hermes.
- [ ] Precedence override terdokumentasi dan teruji.
- [ ] Missing, malformed, dan unknown provider menghasilkan error actionable tanpa secret.
- [ ] Pengujian memakai Hermes home disposable.
