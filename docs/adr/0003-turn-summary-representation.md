# ADR 0003: Representation of the dropped-context summary

- Status: proposed (for review, alongside Spec 008 closure)
- Date: 2026-09-05
- Scope: Spec 008 (memory/context) — turn summarization
- Related: ticket `03-turn-summarization`, `docs/FTS5_CONTRACT.md`, `docs/PARITY.md`

## Context

Spec 008 adds context management to Hermes-RS. When the sliding window drops
old turns from the send-side copy (`turns_to_send`), the dropped content is
"lost" to the model on the next request unless a summary is reintroduced.

Ticket 03 shipped **heuristic, display-only** visibility of what was dropped
(`/info`, `summarize_dropped`). That feature is intentionally never injected
into the model context. Before Spec 008 can close, we must decide — for the
**future** feature that would inject a summary into the model — *what* that
summary is, how it is stored, and how it maps onto a provider request. This
ADR records that decision now so the deferred implementation is fully
specified and does not reopen the design later.

The open questions (from ticket 03 / Matt's review):

1. Do we add `Turn::Summary` or reuse a `Turn::System`?
2. Is the new variant persisted to the `state.db` `role` column?
3. How does it map onto a provider request (`chat_completions` vs
   `completions`)?
4. How do the FTS5 search index and `/search` handle the new variant?
5. STRIDE: who may create or modify the summary?
6. Parity: does upstream Python Hermes have a comparable concept?

## Decision

**Adopt a dedicated `Turn::Summary { content: String }` variant.** Do not reuse
`Turn::System`. Persist it to `state.db` with a reserved `role = "summary"`.
Map it to the provider as a `system` message in `chat_completions` mode and as
a **prefix to the assembled prompt** in `completions` mode. Exclude summary
rows from FTS5 searchable content by default. Only Hermes-RS itself may create
or modify a summary.

### Rationale for `Turn::Summary` over `Turn::System`

`system` in the conversation carries a distinct meaning: the standing system
prompt / developer instructions that apply to the *whole* session and are
usually the first message. A context summary is (a) derived from conversation
content, (b) recomputed as the window slides, and (c) positioned where the
dropped span began, not necessarily at the head. Overloading `Turn::System`
would conflate "standing instructions" with "derived recollection", which have
different lifetimes, producers, redaction rules, and update cadence. A named
`Summary` keeps provenance explicit and lets policy target it precisely.

`Tool` turns already store an arbitrary tool *name* in the `role` column, so
the schema is not constrained to a fixed role enum. Adding `summary` is
therefore consistent with the existing free-form `role` design and does not
require a migration.

### State.db role column

Store as `role = "summary"`, content as the summary text. Persist **only the
generated summary turn**, not the individually dropped turns — the full
history already remains in `state.db`; the window drops only the send-side
copy, never the canonical rows (Spec 008 invariant).

- Backward compatibility: existing readers map `"user"`→`User`,
  `"assistant"`→`Assistant`, and every other string→`Tool { name }`. A stored
  `"summary"` row would therefore be misread as `Tool` by an old build. This
  is the one wrinkle the decision must accept. Mitigation: when injection is
  implemented in the same commit as persistence, `SessionStore::resume` maps
  `"summary"`→`Turn::Summary`; older binaries reading a newer db are not a
  supported combination (Rust-Hermes is a single moving binary, not a
  long-lived db server). Within one binary version the round-trip is lossless.

### Provider mapping

- `chat_completions` (`HttpProvider`, `ApiMode::ChatCompletions`): render the
  `Summary` turn as an `ApiMessage { role: "system", content }`. Because a
  `system` message is conventionally expected to lead, the summary is emitted
  at its chronological position only if the API accepts mid-conversation
  `system`; otherwise the implementation must hoist the summary to just after
  the real system prompt. This mapping is delegated to the injection feature's
  implementation; the ADR fixes the *role name* (`system`) and records that
  hoisting is the fallback.
- `completions` mode: there is no per-message role. Render the summary as a
  lead block (e.g. `Summary: ...`) at the head of `render_completions_prompt`,
  before the transcript, so the model receives the recollection without
  inventing a user author. Crucially, a summary must **never** be rendered as
  `User: ...`, preserving the invariant that no fake user turn is introduced.

Adding a variant requires updating every exhaustive `match` over `Turn`:
`session/store.rs` (save + resume), `provider/http.rs` (build_request
chat/completions + render_completions_prompt), `provider/fake.rs`, and the
token estimator in `conversation/context.rs`. The compiler enforces this — a
benefit of the closed enum.

### FTS5 parity

`docs/FTS5_CONTRACT.md` governs what is indexed. A summary is a derived
recollection of user/assistant/tool content; indexing it verbatim would
double-count tokens and could surface a paraphrase as if it were a primary
statement. **Default: summary rows are excluded from FTS5 searchable content.**
If a future ticket wants summaries searchable, it must define ranking/dedup and
be a separate deliberate change.

### STRIDE

- **Spoofing/Integrity:** only Hermes-RS generates summaries; they are not
  produced from arbitrary user text by a writer outside the conversation
  machinery. Creating a summary is gated behind the internal compression path,
  not a free-form input. This keeps tampering risk low.
- **Information Disclosure:** a summary condenses old turns; it must be treated
  as derived from user content and pass the same redaction/sanitization as the
  source turns before display or injection (`redact_credentials`,
  `sanitize_untrusted_output`).
- **Denial of Service:** a summary that is unboundedly large could itself blow
  the token budget. Bound summary length (ticket 03 caps at
  `SUMMARY_MAX_CHARS`), and count it against `context_limit` like any turn.
- **Elevation:** no privilege change; summary stays in-band conversation data.

### Parity (Python Hermes)

At the time of writing, upstream Python Hermes has no first-class, persisted
"summary turn" row; context compaction in the reference is a later capability
this Rust slice is defining ahead of parity. `docs/PARITY.md` Spec 008 should
note the divergence and that summary injection remains gated on the Rust side
until parity semantics are agreed. If upstream later defines its own
representation, this ADR is the seam where it reconciles.

## Non-goals (this ADR does not decide)

- **Whether/when to inject** the summary at all, or the exact compression
  trigger. Ticket 03 ships display-only; the injection feature and its
  on/off wiring are separate future work.
- **System-prompt activation.** The standing system prompt is not yet wired to
  the provider (Matt's Ticket-02 note). `Turn::System` semantics are orthogonal
  and tracked separately.

## Consequences

- Downstream must handle `Turn::Summary` in every `match` (compiler-driven).
- Old binaries cannot read a db containing `role="summary"` correctly; accepted
  because Hermes-RS is a single moving binary.
- Search does not index summaries unless a future ticket explicitly opts in.
- Closing Spec 008 is now unblocked on the summary-representation question; the
  display-only heuristic (ticket 03) is unaffected by this ADR.
