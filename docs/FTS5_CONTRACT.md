# Spec 004 — FTS5 Search Contract

Status: **Planning approved / Ticket 01 complete**

FTS5 is a rebuildable derived index. `state.db` canonical session and message records remain authoritative.

## 1. Index model

Use an external-content FTS5 table backed by `messages`:

```sql
CREATE VIRTUAL TABLE message_search USING fts5(
    content,
    role,
    session_id UNINDEXED,
    message_id UNINDEXED,
    content='messages',
    content_rowid='id'
);
```

The index is not canonical data and may be deleted and rebuilt from `messages`.

## 2. Schema

| Column | Indexed | Purpose |
|---|---:|---|
| `content` | yes | Message full-text content |
| `role` | yes | Role filtering/search |
| `session_id` | no | Result association and optional scope filter |
| `message_id` | no | Canonical message row association |

The synchronization strategy (explicit rebuild versus triggers) is deferred to the migration ticket and must not alter canonical table semantics.

## 3. Write boundary

Allowed writes are limited to FTS virtual table creation/migration, FTS metadata/version records, and FTS rebuild/maintenance. Search/index code must never insert, update, or delete rows in `sessions`, `messages`, or `tool_calls`.

## 4. Migration

Migrations are versioned, idempotent, transactional, retry-safe, and compatible with existing Hermes databases. Failure must not partially modify canonical tables. FTS5-unavailable builds return a safe actionable error; they do not silently fall back to a mutating implementation.

## 5. Rebuild

Canonical rebuild operation:

```sql
INSERT INTO message_search(message_search) VALUES('rebuild');
```

Rebuild reads `messages` only, is deterministic and repeatable, supports empty databases, and can recover a corrupt derived index by recreating it. Canonical rows remain unchanged.

## 6. Query policy

The default `/search <query>` mode treats input as literal text, not raw FTS5 syntax. FTS5 operators and metacharacters must not unexpectedly broaden the query. All values use parameter binding; SQL and FTS expressions must not be built by concatenating user input.

Advanced FTS syntax, if later supported, requires an explicit opt-in interface such as `/search --query-syntax <query>` and is a separate security surface.

## 7. Result and limits

```rust
pub struct SearchResult {
    pub session_id: SessionId,
    pub message_id: i64,
    pub role: String,
    pub snippet: String,
    pub rank: f64,
}
```

Defaults:

```text
maximum query length: 4 KiB
maximum results:      50
maximum snippet:      4 KiB
```

Empty, over-limit, malformed, unavailable-FTS5, and missing-index behavior must be typed and documented.

## 8. Execution boundary

Results are untrusted display data. They must never be parsed as tool XML, dispatched to `ToolRegistry`, sent to a provider as an automatic follow-up, executed as shell commands, interpreted as file operations, used for network requests, or routed through confirmation as an executable action.

A result containing `rm -rf /`, a URL, XML tool-call syntax, or a write command remains plain text.

## 9. Redaction and rendering

Canonical content is untouched. Redaction and terminal sanitization happen only at the output boundary and reuse the Spec 003 policy:

```text
canonical messages → FTS derived index → SearchResult
→ credential redaction + ANSI sanitization → terminal
```

Actual CSI/OSC/DCS/control sequences must not reach the terminal; literal escaped source code remains visible.

## 10. Non-goals

This contract introduces no tool execution, arbitrary shell access, network access, automatic action from results, canonical history mutation, default advanced syntax, or canonical-message retention policy.

## 11. Required proof

Spec 004 cannot be called `DONE` or `production-polished` until tests prove migration idempotency, rebuild correctness/repeatability, canonical immutability, literal query behavior, SQL/FTS injection resistance, session isolation, ANSI-safe rendering, bounded limits, no execution from results, and green Spec 001–003 regressions.
