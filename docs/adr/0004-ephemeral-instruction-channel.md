# ADR 0004: Ephemeral instruction channel (plan generation)

- Status: accepted
- Date: 2026-09-05
- Scope: Spec 009 (planning & reflection) — Ticket 02
- Related: ADR 0003 (`Turn::Summary`), `docs/SECURITY.md`

## Context

Spec 009 needs the model to produce a structured plan (`[[plan]]…[[/plan]]`)
before executing tools. Hermes-RS sends only conversation turns to the provider;
there is **no system/instruction channel today**. `config.ephemeral_system_prompt`
exists but is deliberately not activated (ADR 0003 defers full system-prompt
activation). Ticket 02 needs a way to ask the model to plan without (a)
activating the general system prompt, (b) fabricating a user turn, (c) adding a
new persisted `Turn`/role (which would need an ADR like 0003).

## Decision

Add a narrow, additive **ephemeral instruction channel** to the provider layer,
used only for plan generation (Spec 009).

- `Provider::chat_with_instruction(&self, turns, instruction: Option<&str>, cancel)`
  added to the trait with a **default implementation that delegates to
  `chat_with_cancel`** (so any provider that does not override it is unchanged;
  `None` behaves exactly like the existing path). `Box<T>` forwards it.
- `HttpProvider` overrides it: in `chat_completions` the instruction is sent as a
  leading `system` message; in `completions` it is a leading
  `[Instruction] …` header. The stream remains tool-aware.
- `FallbackProvider` forwards the instruction to each hop.
- The runner calls it only inside plan generation (`ensure_plan`) with an
  **internal constant** instruction; it is never derived from user input.

### Explicit boundaries

- The instruction is **never persisted**, never a `Turn` variant, never a
  `state.db` role, and never appears in `/messages`. A user prompt is never
  executed as an instruction.
- `ephemeral_system_prompt` is **not** activated here. That is a separate,
  wider ADR (system-prompt activation) with its own provider-mapping, redaction,
  and STRIDE decisions.
- The channel is for future internal steering; exposing it to the REPL/CLI or to
  user input is out of scope until a further ADR.

## Why not the alternatives

- **Activate `ephemeral_system_prompt` (Option B):** too broad — that is the
  general system-prompt decision ADR 0003 defers (mapping, persistence,
  redaction of system content, STRIDE for user-controlled prompts). Using it
  only for planning would be a hammer for a tack.
- **Heuristic-only plan (Option C):** diverges from the requested "one model
  round-trip producing `[[plan]]`"; a hand-split goal adds little real value
  versus letting the model sequence tool calls.

## Consequences

- Providers gain an optional, backward-compatible method (default ignored).
- Plan text uses bracket delimiters (`[[plan]]`/`[[/plan]]`), disjoint from the
  `<tool_call>` XML parser; a literal tool tag in a plan is text, never
  executed.
- The channel is a controlled precursor: when system-prompt activation is
  designed later (a future ADR), this method is already the seam and can be
  widened deliberately rather than refactored from scratch.
