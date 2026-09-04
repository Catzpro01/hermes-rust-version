# 04: CLI interaktif end-to-end

**What to build:** User dapat menjalankan Hermes-RS dari terminal, memilih atau melanjutkan session, mengetik prompt, melihat response streaming, menangani Ctrl-C/EOF, dan menerima exit status yang benar.

**Blocked by:** 02 — Conversation runner offline dan session persistence. Provider nyata pada 03 tidak memblokir pengembangan atau pengujian CLI dengan fake provider; integrasi penuh dengan provider nyata diverifikasi setelah 03 selesai.

**Status:** ready-for-agent

- [ ] CLI dapat memulai session baru dan resume session.
- [ ] Input prompt dan output event runtime dirender dengan benar.
- [ ] Fake provider dapat digunakan untuk smoke test offline.
- [ ] Ctrl-C menghentikan turn secara bersih dan EOF keluar tanpa merusak session.
- [ ] Configuration, runtime, dan provider errors menghasilkan exit status nonzero yang tepat.
- [ ] CLI tidak mencetak credential.
- [ ] CLI dapat dijalankan melalui SSH pada Ubuntu tanpa graphical environment.
