# 03: Provider LLM streaming

**What to build:** Satu provider HTTP-compatible nyata dapat dipilih melalui konfigurasi, menerima prompt dari conversation runner, mengirim streaming response, menangani error dan cancellation, serta meredaksi credential.

**Blocked by:** 02 — Conversation runner offline dan session persistence.

**Status:** ready-for-agent

- [ ] Provider dapat dipilih melalui konfigurasi tanpa perubahan pada conversation runner.
- [ ] Request, authentication, response decoding, dan provider-specific errors terisolasi di adapter.
- [ ] Streaming chunks dinormalisasi menjadi event runtime.
- [ ] Empty/malformed chunks dan stream termination ditangani secara aman.
- [ ] Cancellation menghentikan request dan tidak merusak session sebelumnya.
- [ ] Credential tidak muncul dalam error, log, atau output diagnostik.
- [ ] Test tidak membutuhkan credential provider nyata.
