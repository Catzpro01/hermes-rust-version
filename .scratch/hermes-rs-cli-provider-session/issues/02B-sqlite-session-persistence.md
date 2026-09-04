# 02B: SQLite session persistence dan Hermes parity

**What to build:** SessionManager menyimpan dan memuat conversation turns di SQLite `state.db` dengan WAL, transaction atomic, UUID v7, dan schema yang dapat membaca database Hermes.

**Blocked by:** 02A — Conversation runner dan provider contract.

**Status:** done

- [x] SessionId UUID v7.
- [x] SQLite store dengan WAL, foreign keys, dan busy timeout.
- [x] Create, resume, list, dan save turn.
- [x] Atomic transaction untuk penyimpanan turn.
- [x] Roundtrip dan unknown-session tests.
- [x] Fixture test menggunakan `state.db` aktual Hermes Python.
- [x] Mapping metadata/message fields penuh sesuai schema Hermes upstream.
- [x] Concurrent resume/write integration test.

Catatan: VM belum memiliki `~/.hermes/state.db` karena belum ada sesi Hermes yang dibuat. Schema Hermes upstream sudah diinvestigasi dari `hermes_state_common.py` dan dokumentasi developer.
