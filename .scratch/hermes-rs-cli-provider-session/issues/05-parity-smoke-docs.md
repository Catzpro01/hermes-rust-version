# 05: Parity smoke test dan dokumentasi operasional

**What to build:** Hermes-RS memiliki smoke test yang menjalankan binary dalam Hermes home disposable, memverifikasi konfigurasi, session persistence, provider flow, dan perilaku CLI, serta mencatat parity gap tanpa mengubah instalasi Python Hermes.

**Blocked by:** 03 — Provider LLM streaming; 04 — CLI interaktif end-to-end.

**Status:** ready-for-agent

- [ ] Smoke test dapat dijalankan tanpa credential nyata menggunakan fake provider.
- [ ] Test memverifikasi alur CLI, streaming event, session persistence, resume, dan error path.
- [ ] Test memverifikasi Hermes home disposable tidak mengubah data upstream.
- [ ] Prosedur manual untuk provider nyata terdokumentasi tanpa menulis secret ke repository.
- [ ] Perbedaan perilaku terhadap Hermes upstream dicatat sebagai parity gap yang dapat ditindaklanjuti.
- [ ] Cargo formatting, check, dan test menjadi langkah verifikasi standar.
