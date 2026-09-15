# ADR 0007: A session's lifecycle status comes from its last message row, and any non-speaker role counts as interrupted

- Status: accepted
- Date: 2026-09-16
- Scope: Spec 017 (visual parity), the session picker's `Stat` column
- Related: `.scratch/hermes-rs-total-parity/issues/W5-picker-status-lifecycle.md`
  (the decision ticket this closes), `docs/hermes-ui-spec/017/evidence/upstream-lifecycle-status/`
  (the pinned reference), `docs/PARITY.md`, ADR 0001 (compatibility-first)

## Context

The picker's `Stat` column reports one of four lifecycle tags. The pinned
reference (`hermes_state.classify_session_status`, retained verbatim under
`docs/hermes-ui-spec/017/evidence/upstream-lifecycle-status/`) derives the tag
from a session's **last message row only**, in this order:

1. `finish_reason` in `{error, agent_error, content_filter}` → `err` (checked
   *before* the role);
2. last row is `user` or `tool` → `intr`;
3. last row is `assistant` carrying `tool_calls` → `intr`;
4. anything else → `done`;
5. no message row → `empty`.

The port inherited that order, but not its vocabulary. `messages.role` is an
overloaded column: it holds a **speaker** (`user`, `assistant`, `system`) for
ordinary rows and an **instrument** — the tool's own name (`shell`,
`read_file`, …) — for tool-result rows, because `Turn::Tool { name, .. }` has
no role of its own and `save_turn` persists `name` as the role. So a database
written by Hermes-RS never contains the role `tool`, rule 2 could never fire
for the Rust shape, and a session interrupted *after* a tool ran but *before*
the assistant replied rendered `done` where the reference renders `intr` —
half of the `interrupted` shape was unreachable in the databases this project
creates itself. Databases written by Hermes Python were never affected: they
store `tool` literally.

Separately, the two columns the reference reads beyond the role
(`tool_calls`, `finish_reason`) exist only in Python-written databases, so on
a database this crate creates `err` cannot arise — equally true of the
reference, which has no other source for it.

## Decision

1. **The reference's order is pinned.** An error `finish_reason` is checked
   first and wins over the role.
2. **A last row whose role is not `user`, `assistant` or `system` is a
   tool-result row and is classified `intr`.** This covers the reference's
   literal `tool` role and the Rust shape where the role is the tool's own
   name, with one rule instead of two vocabularies.
3. **`system` is a speaker and stays `done`.** It is listed explicitly rather
   than left to fall through, because rule 2 makes the fall-through branch
   mean something else.
4. **An empty or unrecognised role is read as a tool-result row → `intr`.** A
   deliberate consequence of rule 2: a session whose last row has no
   recognisable speaker did not get its answer.
5. **No schema change.** `messages.role` keeps carrying the tool name for
   tool-result rows, and the rule lives in exactly one function,
   `classify_session_status`.

Rule 2 is a recorded **deviation** from the reference: its benign default for
an unknown shape is `complete`, this one is `intr`. It is listed as an
adaptation in `docs/PARITY.md`, not presented as parity.

## Considered options

- **Keep the reference's vocabulary and declare the gap** (the T09 pattern:
  `intr` unreachable in Rust, recorded as a stated limitation). Rejected: it
  was the status quo, and it leaves the "interrupted tool turn" shape — the
  one the W5 ticket set out to make capturable — permanently unobservable in
  the databases this project actually creates.
- **Change persistence so tool rows are written with role `tool`** and keep the
  classifier byte-faithful. Rejected: it changes the stored transcript, which
  is a bigger and harder-to-reverse decision than a display rule, and it does
  not by itself make the two spellings agree for databases already on disk.
- **Add a `kind` column** (or a separate tool-result table) so the row's nature
  stops being inferred. Rejected for now as out of Spec 017's scope (display
  parity, not storage); noted below as the change that would obsolete rule 2.

## Consequences

- Databases created by Hermes-RS can now render `intr` for two distinct
  reasons — an unanswered user turn and an unanswered tool result — so the
  live gate `scripts/check_picker_status_tags.py` has something to capture on
  the Rust side. `err` remains unreachable there, as in the reference.
- Existing evidence moves: a fixture row whose last row is a tool result flips
  `done` → `intr`, on top of the `done` → `intr` flip for user-last rows that
  PR #11 already caused. Nothing in the five-row `picker-status-tags` fixture
  changes, because none of its shapes ends in a tool row.
- The classifier is no longer byte-faithful for exotic roles. A
  Python-written database containing a role this project does not know (say
  `developer`) now renders `intr` where Python renders `done`. Accepted: the
  Rust transcript vocabulary has exactly four shapes, and for a *status*
  column, reporting an interruption that did not happen is the safer error
  than hiding one that did.
- Rule 2 infers "tool-result row" from the *absence* of a known speaker. If a
  future ADR gives tool rows their own role or column, this ADR must be
  revisited: `classify_session_status` is the only place that inference lives.
