# Progress and successor handoff

Append observed results, not predictions. Read `MEMORY.md` and
`docs/agents/skills/progress-handoff/SKILL.md` before continuing.

## 2026-09-14 — Spec 017 closure review and GitHub progress workflow

**Request:** read the existing analysis, continue the closure review, fix
errors, preserve progress on GitHub, inherit skills/memory, and never merge
without an explicit instruction.

**Branch:** `arena/01a09c1e-hermes-rust-version` (base `6ded9dd`).

### Inherited context and skills

Read `AGENTS.md`, `CONTEXT.md`, `docs/agents/{issue-tracker,triage-labels,domain}.md`,
ADRs 0001/0002/0006, `docs/ROADMAP.md`, `docs/PARITY.md`, the relevant
`docs/HERMES_UI_SPEC.md` sections, and Spec 017 T08/T10/board notes.
The user's commit/push and memory-inheritance rules are now durable in
`AGENTS.md`, `MEMORY.md`, and the `progress-handoff` skill. No external
`~/.hermes` skill/memory installation exists in this sandbox; no import or
modification of Python Hermes is claimed.

### Changes

- Fixed CI's swallowed rustfmt failure: pipefail, fmt outcome, always-run
  final gate, and format diagnostics. Added three workflow regression tests
  and a separate CI job to run them on subsequent pushes.
- Corrected outdated parity claims (FTS5/function calling, default-on CLI
  sandbox, 101 upstream + 14 Rust completion entries, reference path).
- Kept closure sign-off open: existing PTY/unit evidence is not automatically
  a replacement for the complete §J.7 capture requirement.
- Added the detailed review ticket:
  `.scratch/hermes-rs-total-parity/issues/T11-closure-review.md`.
- No runtime Rust code or Python Hermes data changed in this checkpoint.

### Verification actually performed

- Three local workflow regression tests passed, including 125 gate-outcome
  tuples, format exit status 0/1/42, log retention, and diagnostics.
- Reproduced the old bug with a failing fake cargo: old pipeline exit 0,
  corrected pipeline exit 1. This is a workflow test, not a rustfmt run.
- YAML parse, workflow shell syntax, reviewed document links, source catalog
  counts, and `git diff --check` passed during the initial review.
- Historical GitHub job `103764353101`: summary annotations confirm 576
  tests passed and clippy success. Not a run of this checkpoint.

### Blockers / next actions

- Local Rust toolchain unavailable. Official rustup/static downloads failed
  TLS; retry and mirror access also failed. Debian apt indexes could not be
  downloaded. No successful Rust build/test/format check is claimed.
- Save this checkpoint to the assigned branch, then inspect that commit's
  CI using `gh`. Fix reported failures without weakening checks. Record the
  actual push/CI result in the next entry, not as a prediction here.
- Visual §J.7 evidence or explicit reviewer approval remains required.
- No merge or auto-merge is authorized. No PR has been opened in this session.

## 2026-09-14 — Checkpoint pushed; genuine format failure exposed

- Commit `52a5fba` pushed successfully to the assigned branch; remote HEAD
  was verified as `52a5fba2c3b73c544859318b4bc1627d76101078`.
- CI run [34776587517](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34776587517)
  failed correctly: annotations report `fmt=failure clippy=success test=success`.
  All **576 Rust tests** passed; the workflow regression job also passed.
- The new gate exposed pre-existing Rust formatting drift, beginning in
  `approval.rs`; the old `|| true` had hidden it. Do not disable the gate.
- The diagnostics also incorrectly called successful SIGINT test output
  (`error: interrupted`) a build failure. Restrict test-build annotations
  to failed test steps and retain a regression for genuine compiler errors.
- Signed artifact/raw log downloads are blocked (EOF), and local Rust
  downloads remain unavailable. Add a formatting-recovery step on the
  GitHub runner: run `cargo fmt --all`, verify formatting, run `cargo check`,
  and export a tracked-Rust-only patch as artifact plus compressed,
  checksummed annotations. It does **not** commit/push or mask the original
  failed check. The patch must be decoded, hash-checked, and reviewed locally.
- Next: push this diagnostics/recovery checkpoint, retrieve its verified
  formatting patch, review/apply it, commit with progress, and rerun CI.
  No merge is authorized or performed.

## 2026-09-14 — Recovery export transport correction

- Checkpoint `b1c8d5c` was pushed. CI run
  [34776704677](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34776704677)
  completed: five workflow regression tests passed, clippy and all 576 Rust
  tests passed, and the recovery step successfully ran `cargo fmt --all`,
  `cargo fmt --all -- --check`, and `cargo check --workspace --locked`.
  The overall job still correctly failed its original fmt check.
- False `test-build` annotations for successful SIGINT tests are gone.
- Patch retrieval exposed another concrete error: the API/proxy truncated
  each 48,000-character annotation to 4,096 characters. Gzip validation
  rejected the incomplete data; **no corrupt patch was applied**.
- Reduce encoded patch parts to 3,000 characters (maximum 40 parts) and
  extend the regression with multi-part, poorly compressible content.
  Preserve checksum validation before applying any remote patch.
- Next: push the corrected exporter, fetch complete parts, verify checksum,
  inspect/apply the Rust-only patch, and rerun CI. No merge performed.

## 2026-09-14 — Respect Actions' per-step annotation quota

- Checkpoint `fe8c5f8` pushed; run
  [34776815998](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34776815998)
  again passed clippy/576 tests and verified the formatted workspace with
  `cargo check`. The five QA tests passed. Original fmt failure stays visible.
- The complete patch is 274,328 bytes (26 compressed parts). Individual
  3,000-character parts now survive the API, but Actions retained only the
  first ten notices in the exporter step (digest + nine parts).
  Completeness validation rejected retrieval; no incomplete patch applied.
- Split export into groups of eight parts across separate workflow steps,
  leaving room for the digest within the per-step quota. Extend the test
  past eight parts and assert per-step notice and per-part size bounds.
- The sandbox's `gh` lacks `--slurp`; use `gh api .../annotations?per_page=100`
  and validate part indices plus digest when retrieving the patch.
- Next action remains checksum-verified patch application, then fresh CI.
  No merge or auto-merge has been performed.

## 2026-09-14 — Verified Rust formatting patch applied

- Checkpoint `815d6ad` pushed. Recovery run
  [34776913905](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34776913905),
  job `103776595076`, successfully ran `cargo fmt --all`, then
  `cargo fmt --all -- --check`, and `cargo check --workspace --locked` on the
  formatted workspace. The pre-format clippy/576 tests and five QA tests
  also passed; the run correctly remained red for its original fmt failure.
- Retrieved **all 26 parts**, decoded the gzip payload, and verified SHA-256:
  `b13539d24b5567695c7178309d8ddb1582874ca0b19f1be42ad54ccc80af5874`.
  Verified every patch path is a tracked Rust file under `crates/`, ran
  `git apply --check`, reviewed representative diffs, and applied the patch.
- Changes: mechanical rustfmt output in **52 Rust files** (2,816 changed
  lines), including import ordering, wrapping, trailing commas, and newline
  normalization. No manual runtime behavior change was introduced.
- The local Rust diff is byte-identical to the patch that passed fmt/check
  on the runner. This is remote pre-commit verification, not a local Rust
  execution; the local toolchain/download blocker still exists.
- `git diff --check` passed. Commit this formatting checkpoint with progress,
  push, and verify a fresh CI run on the actual formatted commit before
  claiming the gate is green. §J.7 visual sign-off remains separate.
- No merge or auto-merge performed.

## 2026-09-14 — Formatting checkpoint CI green; handoff ready

- Formatting commit `3e0e8d9` was pushed successfully. CI run
  [34776992554](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34776992554)
  tested exact SHA `3e0e8d978e9b6047102ef8d6225907cf25e33637` and is **SUCCESS**.
- Rust job `103776809106` annotations explicitly report
  **`fmt=success clippy=success test=success`**; all **576 Rust tests passed**.
  The corrective export steps were skipped because formatting now passes.
- Workflow-regression job `103776809256` also passed (**5 tests**, including
  125 gate-status tuples, real versus expected error diagnostics, multi-part
  patch integrity, and annotation size/quota bounds).
- No failure annotations remain. There is a pre-existing non-blocking
  GitHub action Node.js 20 deprecation warning; this is not a Rust/clippy error.
- Pre-commit `cargo fmt --all` and `cargo check --workspace --locked` were
  performed on the runner-generated source before applying its byte-identical
  verified patch. Local Rust remains unavailable; do not claim a local build.
- This follow-up updates only progress, memory, skill handoff, and closure
  evidence, then commits/pushes them. The exact final commit/push/check status
  is reported to the user after it exists, rather than predicted here.

### Next agent

Read `AGENTS.md` → `CONTEXT.md` → `MEMORY.md` → this entry → the
`progress-handoff` skill and T11. CI/error repair is verified on the source
checkpoint above. **Remaining closure work is §J.7 visual evidence or an
explicitly approved alternative, followed by human sign-off.** Do not mark
that requirement complete based solely on green CI. Use the assigned branch
of your own session; never switch to another branch based on an old handoff.
Always commit/push progress. **No merge or auto-merge has been authorized or
performed.** The Python installation was not modified.

## 2026-09-14 — `/ask-matt` awaiting the existing skill definition

- Restored context from AGENTS, CONTEXT, MEMORY, progress-handoff, local
  issue/triage/domain guides, and T11. Rechecked CI `34777096070`: both jobs
  succeeded on exact checkpoint `3794df36ccde3481c210fa1e23d153a41568d0f9`.
- Repository references prior `/ask-matt` decisions but no callable command,
  skill definition, or reviewer connector was found in the available checkout
  and inspected workspace paths. This does not establish that the skill is
  absent from the user's external environment.
- User selected **use the existing Matt skill**, not substitute agent review.
  Status: `needs-info` — await its path/attachment and any required resources.
- Next action: read that actual skill and follow its procedure. Do not invent
  a Matt verdict, recreate a substitute skill, or close §J.7 while waiting.
- Documentation-only checkpoint; no new Rust test run, external reviewer
  contact, PR, merge, or auto-merge performed. Save this blocker to GitHub.

## 2026-09-14: Official Matt Pocock toolkit installed and applied

- User asked to download Matt Pocock's skills and apply their contents.
  Located the author's public `mattpocock/skills` repository through GitHub.
  Pinned source: `3cca18b368ae95cdbdebbff572ccafa662551015`, MIT license.
- Downloaded all **164 upstream files (666,067 bytes)** through `gh api`,
  checking each Git blob hash against the pinned tree. Preserved upstream
  content and modes, including its internal AGENTS symlink. The full snapshot
  and lock live under `docs/agents/vendor/`.
- Installed **37 relative project-local skill links** under `.agents/skills/`:
  25 stable, eight in-progress, four misc. The latter buckets retain their
  upstream warnings; `retro` is a stub. No upstream installer or package
  script was executed, and Python Hermes/global skill directories are untouched.
- Read the original ask-matt/router and phase-boundary instructions, setup
  preconditions, invocation policy, writing-for-agents and skill mechanics,
  and code-review/TDD/codebase-design requirements relevant to subsequent
  work. Scanned every skill's metadata; deeper references are read on trigger,
  not indiscriminately loaded or executed as a single giant workflow.
- Corrected the earlier misunderstanding: `/ask-matt` is **a router**, not an
  external Matt contact or approval service. The previous missing-skill
  blocker is resolved. Its route for the current §J.7 decision is
  `/grill-with-docs`, then scoped specification/tickets/implementation if
  needed. No new user-invoked flow or closure approval was silently started.
- Existing local-ticket, five-label, and single-context configuration already
  meets the setup preconditions; retained it. Added concise AGENTS pointers,
  refreshed MEMORY, and documented the exact-source catalog, permission
  overrides, experimental status, and missing native Skill/subagent tools.
- Verification: all 164 blobs/modes and 37 links passed; every YAML skill
  frontmatter agrees with its `agents/openai.yaml` invocation policy. Existing
  **five CI workflow regression tests passed locally**. No Rust code changed
  and no new local Rust run is claimed. Run final whitespace/link checks,
  commit/push progress to the assigned branch, and inspect checkpoint CI.
- Tracking: `.scratch/hermes-rs-agent-skills/issues/01-matt-pocock-toolkit.md`.
  No branch switch, PR, merge, auto-merge, global Git hook, or credential
  request occurred. The next agent should read the original skill on invocation
  and state any unavailable capability instead of fabricating its execution.

## 2026-09-14: Toolkit checkpoint verified on GitHub

- Commit `b959ab396422bdd98daccf7b0f8c9710c21cfc0e` was pushed and remote
  HEAD verified. Run
  [34777846810](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34777846810)
  is **SUCCESS** for that exact installation checkpoint.
- Both jobs passed. Rust job `103779132129` explicitly reports
  `fmt=success clippy=success test=success`, **576 Rust tests passed**.
  Workflow regression job `103779132017` passed the existing **five QA tests**.
- Marked the installation ticket DONE. This follow-up changes only the
  ticket and progress notes; its own commit/run status will be reported after
  push, not predicted in this entry.
- Successor entry point: `docs/agents/matt-pocock-skills.md`, then the exact
  `.agents/skills/<name>/SKILL.md`. The user's earlier `/ask-matt` request can
  now be answered from its original router. §J.7 still needs an evidence
  decision; no reviewer approval or merge permission was inferred.
- All source/metadata checks passed; no download error remains. No merge,
  branch switch, global installer, hook, or Python Hermes mutation occurred.

## 2026-09-14: `/ask-matt` route for Spec 017 closure

- Read the installed original router, phase-boundary tree, and active T11.
  Rechecked run `34777939387`: both CI jobs passed on `f5f820a`.
- Recommendation: user invokes `/grill-with-docs` to settle §J.7's required
  visual evidence, normalization/adaptations, and acceptance/sign-off criteria.
  Build only after those decisions; split into spec/tickets if multi-session.
- Continue in this session: the current reasoning remains relevant. No clear,
  compact, new user-only flow, independent review, or merge was performed.
- Routing/progress documentation only; no code change or new Rust test run.

## 2026-09-14: `/grill-with-docs` started, first decision pending

- Read original grilling/domain-modeling instructions and their glossary/ADR
  formats; inspected §J.7, T11, documented adaptations, and local tools.
  CI `34778089793` is SUCCESS on `37d10f1`; no new local Rust run.
- Recorded the design tree in `.scratch/hermes-rs-total-parity/grilling.md`.
  Current frontier: retain real side-by-side capture plus tests (recommended),
  or explicitly amend the evidence requirement. Later matrix/comparison/gate
  questions depend on that answer and are not assumed.
- No user answer yet: no glossary definition, ADR, waiver, implementation,
  or closure approval written as settled. Facts were inspected directly;
  no native Skill invocation or independent subagent execution is claimed.
- Documentation only; commit/push this interview state. No merge authorized.
