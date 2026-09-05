# ADR 0005: Error recovery via parameter mutation — never retry a denied tool

- Status: accepted
- Date: 2026-09-05
- Scope: Spec 009 (planning & reflection) — Ticket 04
- Related: ADR 0003 (`Turn::Summary`), ADR 0004 (ephemeral instruction channel),
  `docs/SECURITY.md` (Spec 002 confirmation boundary)

## Context

When a tool call fails, `chat_agentic` currently pushes a `Turn::Tool` with the
error text and the next iteration re-sends the whole window. The model may then
re-issue the **identical** tool call with the **same arguments** until it hits
`MaxIterations`. Spec 009 (Ticket 04) asks for error recovery by parameter
mutation: after a retryable failure, the model should pick *different*
parameters instead of repeating the exact request, bounded so it cannot loop
unproductively or hammer an expensive tool.

Two earlier tickets set invariants Ticket 04 must respect:

- Spec 002 (confirmation boundary): a **user denial is a human decision** and
  must never be bypassed by retrying.
- Ticket 03: the heuristic verdict can mark a goal `Blocked` and the runner can
  early-stop on it; `Blocked` is semantically distinct from running out of
  iterations.

## Decision

Add an internal, in-memory `RetryTracker` (`crates/hermes-core/src/conversation/recovery.rs`)
that records a **deterministic fingerprint of the arguments** already attempted
per tool, and drive recovery through the **existing tool-result channel** (no new
tool, no new shell, no new instruction channel beyond the already-existing
`chat_with_instruction`).

Concretely:

1. **Fingerprint.** `arg_fingerprint` = FNV-1a 64-bit over the argument string
   with surrounding whitespace trimmed (canonicalising superficial
   differences). This is a deliberate deviation from a sha256 recommendation:
   there is no hashing crate in the workspace and none is added for a single
   call site. Its only job is to detect an *identical repeated* argument set
   within one run, not to resist adversarial collisions. If collision
   resistance is ever needed it is a one-line swap behind pinned tests (noted
   in the module doc).

2. **Bound.** `MAX_RETRIES = 3` distinct recorded failures per tool, aligned
   with the bounded `RetryPolicy` default of Spec 006. `RetryTracker::can_retry`
   is `false` once that many distinct failing argument sets have been recorded.

3. **Classify.** Only `Error`/`Timeout` are retryable and are recorded.
   `Denied` is **never recorded and never retried** — it preserves the
   Spec 002 invariant and surfaces through Ticket 03's `Verdict::Blocked`
   (`Denied → Blocked`). `Cancelled` is handled by the runner teardown path
   before it reaches recovery.

4. **Reject identical repeats.** Before executing a call, if the exact argument
   fingerprint was already tried, the runner does **not** execute it again. It
   appends an "already tried" note (Option 1 the user chose) to a `Turn::Tool`
   result so the model sees the failed parameter sets and picks different ones,
   and `continue`s the loop.

5. **Block when exhausted.** When a retryable failure records the final allowed
   attempt (or an identical repeat is attempted with no retries left), the goal
   is marked `Blocked` and the loop early-stops with a **new** result variant,
   `AgenticResult::Blocked { reason }`. We add a variant rather than reusing
   `MaxIterations(usize)` because they are semantically different outcomes
   (a blocked goal vs. a spent budget) and callers distinguish them.

6. **Off by default / zero regression.** Recovery is active only while the
   reflection gate is on (`recovery_enabled() == reflection_enabled()`). With
   reflection off — the default — the tool loop is unchanged. The tracker is
   in-memory only and is reset at the start of each task and when turns are
   replaced (`/new`, `/resume`).

### Explicit boundaries

- No new tool, no new shell, no persisted state, no new persisted role in
  `state.db`; `tool_calls` semantics are unchanged.
- No general system-prompt activation (deferred per ADR 0003/0004). Recovery
  surfaces only as an annotation inside a tool result, which is already in the
  model's view.
- `Denied` is never retried; parameter mutation never re-executes identical
  arguments.

## Consequences

- A failing step stops after a bounded number of *distinct* attempts and reports
  `Blocked` to the caller, which the REPL prints, instead of silently spinning
  to `MaxIterations`.
- The model is nudged to mutate parameters by seeing exactly which parameter
  sets already failed — no extra instruction plumbing.
- Because recovery reuses the heuristic reflection gate, both remain off unless
  a user opts in, preserving reactive-mode parity (Spec 002 zero regression).
- Accepted trade-off: the deterministic heuristic (not an LLM) drives both
  reflection and recovery; no new model round-trip is introduced here.
