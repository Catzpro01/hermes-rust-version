# Verbatim excerpt of hermes_state.py at 63279301bcbdc185c1b07b98a9312eb0c862f26d
# Lines 5105-5146 (lifecycle constants + classify_session_status) and
# lines 12396-12446 (SessionDB.session_lifecycle_statuses).
# Nothing edited; see provenance.json for blob/hash verification.


# Lifecycle statuses surfaced by session pickers. Classification looks ONLY at
# a session's final message row — role, whether it carries tool_calls, and its
# finish_reason — so it stays O(1) per session (see
# SessionDB.session_lifecycle_statuses).
SESSION_STATUS_COMPLETE = "complete"
SESSION_STATUS_INTERRUPTED = "interrupted"
SESSION_STATUS_ERROR = "error"
SESSION_STATUS_EMPTY = "empty"

# finish_reason values that mark the turn as having ended in a provider or
# agent error (vs. a normal 'stop'/'length'/'tool_calls' completion).
_ERROR_FINISH_REASONS = frozenset({"error", "agent_error", "content_filter"})


def classify_session_status(
    role: Optional[str],
    has_tool_calls: bool,
    finish_reason: Optional[str],
) -> str:
    """Classify a session's lifecycle from the shape of its final message.

    - assistant with a normal finish → ``complete``
    - assistant that still has pending tool_calls (no tool result row ever
      followed, or it would be the last row instead) → ``interrupted``
    - user or tool as the last row → ``interrupted`` (the agent never got to
      answer / never consumed the tool result)
    - an error finish_reason on the last row → ``error``
    - anything unrecognized → ``complete`` (benign default; pickers must not
      alarm on unknown shapes)
    """
    if (finish_reason or "").strip().lower() in _ERROR_FINISH_REASONS:
        return SESSION_STATUS_ERROR
    r = (role or "").strip().lower()
    if r == "assistant":
        # The last row being an assistant message WITH tool_calls means the
        # matching tool result never landed — an interrupted tool turn.
        return SESSION_STATUS_INTERRUPTED if has_tool_calls else SESSION_STATUS_COMPLETE
    if r in {"user", "tool"}:
        return SESSION_STATUS_INTERRUPTED
    return SESSION_STATUS_COMPLETE


# ---- lines 12396-12446 ----
            s["unread"] = self.session_unread(s)

        return sessions

    def session_lifecycle_statuses(
        self, session_ids: List[str]
    ) -> Dict[str, str]:
        """Classify each session's lifecycle state from its LAST message row.

        Returns ``{session_id: status}`` where status is one of:

        - ``'complete'``    — last message is a normal assistant reply
        - ``'interrupted'`` — last message is a user turn, a pending assistant
          tool call (no tool result followed), or a tool result the assistant
          never responded to
        - ``'error'``       — last message carries an error finish_reason
        - ``'empty'``       — session has no messages

        Cost-bounded by design: one query that resolves each listed session's
        newest message id via ``MAX(id)`` (an index seek on
        ``idx_messages_session_id``) and joins back for that single row's
        role/tool_calls/finish_reason. Never scans transcripts, so it stays
        cheap on large databases regardless of total message volume.
        """
        ids = [sid for sid in (session_ids or []) if sid]
        if not ids:
            return {}
        statuses: Dict[str, str] = {sid: "empty" for sid in ids}
        placeholders = ",".join("?" for _ in ids)
        query = f"""
            SELECT m.session_id, m.role,
                   m.tool_calls IS NOT NULL AS has_tool_calls,
                   m.finish_reason
            FROM messages m
            JOIN (
                SELECT session_id, MAX(id) AS max_id
                FROM messages
                WHERE session_id IN ({placeholders})
                GROUP BY session_id
            ) latest ON m.id = latest.max_id
        """
        with self._read_ctx() as conn:
            rows = conn.execute(query, ids).fetchall()
        for row in rows:
            statuses[row["session_id"]] = classify_session_status(
                role=row["role"],
                has_tool_calls=bool(row["has_tool_calls"]),
                finish_reason=row["finish_reason"],
            )
        return statuses

