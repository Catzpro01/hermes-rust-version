# 02A: Conversation runner dan provider contract

**What to build:** Runtime in-memory menerima prompt melalui provider-neutral contract, menghasilkan event sequence, dan menyediakan fake provider deterministik tanpa I/O atau API key.

**Blocked by:** 01 — Hermes home dan konfigurasi kompatibel.

**Status:** done

- [x] Provider trait async dan EventStream.
- [x] FakeProvider untuk echo, error, dan offline test.
- [x] ConversationRunner menyimpan user/assistant turns in-memory.
- [x] Event sequence dan error behavior teruji.
