# Session Inspection Output Contract

This document defines the stable, read-only output contract for the Hermes-RS inspection extension. Canonical session/message data remains in SQLite; inspection commands never execute tools, call the network, or mutate canonical records.

## `/sessions`

One session per line, newest first:

```text
Sessions:
<uuid>  started=<unix-seconds>  <first-user-message-preview>
```

Empty state:

```text
No sessions.
```

The preview is display-only and is not executed. Full content is available through `/messages <id>`.

## `/inspect <id>`

```text
Session: <uuid>
Source: <source>
Started: <unix-seconds>
Turns: <count>
Tool calls: <count>
```

## `/messages <id>`

Messages are emitted in SQLite insertion order:

```text
[<sequence>] <role>: <content>
```

Roles are `user`, `assistant`, or the stored tool name. Empty sessions print `No messages.`.

## `/tool-calls <id>`

Tool calls are emitted by `turn_index`:

```text
<id> [<status>] <tool-name> args=<arguments> result=<result>
```

Statuses are `success`, `error`, `denied`, `timeout`, or `cancelled`. Empty results print `No tool calls.`.

## Compatibility notes

- IDs are UUID strings stored in SQLite `TEXT`; Hermes-RS uses UUID v7 while the Python implementation commonly uses UUID v4.
- Timestamps are currently exposed as Unix seconds to preserve the existing database representation.
- `search` remains a placeholder until a separately versioned FTS5 derived index is implemented.
- Arguments and results are untrusted display data. Inspection never invokes commands contained in them.
- Secret redaction remains mandatory before displaying provider credentials or tokens.
