# 02: Conversation runner offline dan session persistence

**What to build:** Dengan fake provider deterministik, Hermes-RS menerima prompt, menghasilkan event response berurutan, membuat session baru, melakukan resume, dan menyimpan session secara atomic tanpa jaringan atau API key.

**Blocked by:** 01 — Hermes home dan konfigurasi kompatibel.

**Status:** superseded — split into 02A and 02B

- [ ] Provider-neutral conversation runner memiliki kontrak event yang dapat dipakai CLI dan adapter lain.
- [ ] Fake provider menghasilkan chunk, completion, error, dan cancellation secara deterministik.
- [ ] User turn dan assistant turn dipertahankan dalam urutan yang benar.
- [ ] Session baru, resume, version metadata, dan ordering bekerja.
- [ ] Session write atomic; kegagalan write mempertahankan session valid sebelumnya.
- [ ] Secret tidak masuk ke session metadata atau diagnostic output.
- [ ] Jika format session upstream ternyata kompleks, batas parity dan pemecahan kerja didokumentasikan sebelum implementasi berlanjut.


Split tickets: [[02A-conversation-runner-provider-contract]] and [[02B-sqlite-session-persistence]].
