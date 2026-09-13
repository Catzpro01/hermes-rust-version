# 014-05: `hermes search <query>`
**Status:** DONE — menunggu review Matt.

## Cakupan (Ticket 05)
`hermes search <query>` memakai `session_menu::search_sessions` — fungsi
yang sama dengan `/search` REPL (Spec 004 FTS5 + redaksi kredensial).

- **Output** identik dengan REPL:
  ```
  Search results for: <query (disanitasi + diredaksi)>
  [N] session=<uuid> message=<id> role=<role> rank=<f.3>
    <snippet (disanitasi + diredaksi)>
  ```
  Tidak ada hit → `No search results.`; `state.db` tidak ada → `No search
  results.` tanpa membuat file apa pun.
- **Redaksi:** query dan snippet melewati
  `hermes_core::search::redact::redact_credentials` setelah
  `sanitize_untrusted_output` — kredensial mentah tidak pernah sampai ke
  stdout (invariant Spec 004).
- **State:** tabel kanonik (`sessions`/`messages`/`tool_calls`) tidak
  disentuh; `repair_search_index` hanya menulis state FTS turunan (kontrak
  `docs/FTS5_CONTRACT.md`, sama seperti REPL).

## Bukti
- **E2E** `tests/subcommands_e2e.rs`:
  - `search_hits_match_live_repl_and_redact_credentials` — pesan fixture
    berisi `API_KEY=super-secret-fixture-xyz`; REPL `/search parity` dan
    `hermes search parity` pada store yang sama menghasilkan blok hasil
    **byte-identik**; stdout memuat `***REDACTED***`, tidak memuat nilai
    mentah, tidak ada ESC, snapshot baris kanonik sebelum/sesudah sama.
  - `search_without_store_says_no_results_and_creates_nothing`.
- Ditambah `tests/search_credential_safety.rs` (Spec 004) untuk jalur REPL.
