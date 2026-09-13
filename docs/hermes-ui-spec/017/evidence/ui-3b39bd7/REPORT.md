# Spec 017 — remaining four-area visual evidence

**Result: REVIEWED_WITH_DIFFERENCES — NOT ACCEPTED / NOT CLOSED.**

48 real Python/Rust paired PNGs (24 scenarios × 100×30 and 80×30), all directly
opened and inspected in this session. These are terminal replays, not generated
mockups. Raw streams, timed casts, input events, complete cell/style dumps,
source/binary/font hashes, fixture metadata and reproduction steps are retained.
The previous missing-four-area artifact gap is substantially addressed; this
is **not a claim of every nested wizard prompt or full Python CLI coverage**.
The section/step inventory and limits below must stay attached to any review.

## Outcome

| Area | Evidence | Comparison result |
|---|---:|---|
| Wizard | 14 scenarios × 2 widths | Differences in geometry, palette, visible rows, hints and some labels/flow; not waived by naming `inquire` |
| Session picker | 5 × 2 | Empty case matches; normal/filter/no-match/delete confirmation have differences beyond the historical adaptations |
| Actual completion | 3 × 2 | `/mod` completes to `/model`; Python native component displays alternatives, Rust REPL cycles inline; no dropdown parity claim |
| Summary | 2 × 2 | 0/3-tools counter text, positions and styles match; same panel body, deliberate title branding remains |
| Banner (prior packet) | 4 widths, 8 views | [Previously reviewed fixture evidence](../banner-5a8e12c/REPORT.md), unchanged; not a new full-state pass |

### V1 — Wizard (unresolved)

At both widths Python uses yellow headers, green selection and an alternate-screen
list. Rust keeps introductory configuration text above cyan inline prompts with
roughly seven visible options. At 80 columns Python truncates long labels while
Rust wraps them. ESC/SPACE guidance, terminal choice text and cancellation text
are not visually/literally equivalent. `wizard-tools` reaches Python's tools
configuration menu but Rust goes directly to a checklist; `wizard-tools-toggle`
reaches both checklists with the first tool disabled. Python shows 17/27 enabled
at entry and availability suffixes such as `[no API key]`; the ported static
catalog has 26 entries. Do not silently equate these catalogs/counters.

Quick setup reaches Python's terminal section while Rust displays its declared
OAuth-not-available notice followed by provider selection. This records a
known deferred feature path, not successful OAuth. Docker images show the same
unavailable notice but differ in surrounding flow (Python egress explanation,
Rust image prompt). No new backend/OAuth/registry implementation is authorized
by a screenshot discrepancy alone.

### V2 — Picker (unresolved)

Python's yellow/cyan header, green selected arrow row and dim footer at terminal
row 30 differ from Rust's plain header, inverse-white row and compact footer
immediately after results. Delete confirmation occupies the bottom row on both,
but Python is red and Rust plain. Filtering still displays `d delete` in Rust,
including when no row matches; Python does not. Python's no-match footer reads
`0/2 sessions`; Rust reads `0/0 sessions (filtered from 2) d delete`.
These static hints/counters must not be normalized away. Empty `No sessions
found.` matches in glyphs/styles at both widths.

Historical `docs/PARITY.md` permits eight-character Rust IDs, relative Active
and `done`/`empty` words, among other explicitly listed picker adaptations.
The pinned Python UI actually shows 18-character IDs and `intr` in this fixture,
contrary to the older document's “6 / no status” description. That factual
correction does not grant a new layout/palette/footer exception.

### V3 — Completion presentation (unresolved; component-host boundary)

The Python side invokes the unmodified `SlashCommandCompleter` in a real
`prompt_toolkit.PromptSession`. This is **not** the full upstream TextArea,
ThreadedCompleter, availability-filter wiring, autosuggest or CLI chrome.
The right side is the actual Rust REPL using its fake/offline provider.
Both receive `/mod`, `/skills ` or `/s` followed by two Tabs (no command Enter).
The snapshots show `/model` on both; a Python skills menu vs Rust `/skills browse`;
and a Python `/s` alternatives/description menu vs Rust `/snapshot`.

These prove live component/key interaction rather than test-only registry data,
but do not prove a Rust dropdown exists, identical availability filtering, all
101 registry candidates, or whole Python/Rust REPL visual parity. The different
banner/chrome around these inputs is a host difference, not measured here as a
new banner regression. Do not claim full-CLI completion acceptance from this host.

### Summary — matched fixture, not extrapolated

`comparison.json` confirms equal emulator row counts (32 at width100; 30 at80),
no non-branding glyph-position differences, no same-glyph attribute differences,
and no background/inverse/underline differences for all four summary cases.
Direct PNG inspection agrees. The comparison excludes title branding and does
not treat invisible foreground attributes on whitespace as image differences.
0 and 3 are actual counts passed to the real display components, not replaced
output strings. Tools are list_dir/read_file/write_file; skills and MCP remain
zero. Nonzero skills/MCP and arbitrary wrapping/tool inventories are not claimed.

## Implemented wizard section/step inventory

The Rust implementation lives in `crates/hermes-cli/src/wizard/setup.rs`; Python
uses unmodified `hermes_cli.setup.run_setup_wizard`. The matrix covers setup-mode
selection and entry to each implemented section, plus representative nested/risk
states. It does not imply full completion of every field in every section.

| Implemented section / transition | Scenarios | Explicit boundary |
|---|---|---|
| Setup mode / first transition | mode, full, blank, quick | First screen after mode choice, not complete end-to-end setup |
| Model & Provider | model; full/blank also reach it | Provider list; per-provider authentication/model lists and Rust generic URL/key-env/model-name prompts not individually captured |
| Terminal Backend | terminal, local, docker, cancel | Local completion, missing Docker/image prompt, ESC; no Docker build or deferred backends/egress execution |
| Messaging Platforms | gateway, gateway-empty, gateway-token | List, no selection, Mattermost dummy URL to empty token prompt; not every subsequent platform field or service lifecycle |
| Tools | tools, tools-toggle | Entry + selection with first item disabled; no live registry execution, tool credentials or exhaustive catalog paging |

“Every implemented step” is therefore supported at **section level**, not every
nested prompt. Do not check an exhaustive-prompt acceptance box or silently
broaden this interpretation. Any additional required field-level/full-CLI evidence
must be captured, not inferred from tests or accepted by this agent on the user's behalf.

## Provenance and integrity

- Rust source: `3b39bd71d674af21e011991627e1eb69bfbce109`; committed source, empty
  Rust worktree diff. Compiler/tool/binary digests are in `paired-bundle.json`.
- [Capture 34788156824](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34788156824)
  and [CI 34788156828](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34788156828)
  succeeded on that exact source. CI success is not visual acceptance.
- Rust transport: 468151 bytes, SHA-256
  `8b02aa1275b8e745c664097663a788ccca083dbcf7c790ba141f71b1d8b93be9`;
  verified against the action's digest, ordered chunks and expected source.
- Python reference: `63279301bcbdc185c1b07b98a9312eb0c862f26d`, unmodified.
  Reused the clean **1c3c9dd Python capture**, not a newly claimed Python run.
  Capture-driver bytes are unchanged between that run and final Rust capture.
  `audit.json` verifies current reference source files against the captured hashes.
- Renderer: xterm5.5.0; Chromium138.0.7204.0; Playwright1.55.0;
  DejaVu Mono5.2.5,16px, scale1. All four font-face hashes are in `renderer.json`.
  Same renderer/font/terminal size on each side. Several emoji display missing-glyph
  boxes in this font environment; Unicode originals are retained, not substituted.
- Matched TERM=xterm-256color, COLORTERM=truecolor, LANG/LC_ALL=C.UTF-8.
  Both sizes are 30 rows. Screenshots show the bottom viewport at the declared
  snapshot endpoint; original scrollback and later recorded bytes remain available.
- Every `.ansi` and `.cast` roundtrips to its bundle stream: **96/96** checks.
  `comparison.json` is derived analysis, never a replacement for original streams.
  `SHA256SUMS` freezes every packet file except that manifest itself.

## Fixtures / isolation / temporal snapshots

Fresh temporary HOME/HERMES_HOME per scenario; empty of user credentials, with
only documented fixture config/DB seeding. No real accounts. Picker seeds:
`550e8400-e29b-41d4-a716-446655440000` / `660f8400-e29b-41d4-a716-446655440001`,
“deploy the thing” / “second topic”, timestamps1700000000/1, sourcecli,
one user message each. Filter `topic`, no-match `zzzz`; delete prompt is not confirmed.
Mattermost URL=`https://fixture.invalid`, token left empty. Summary model=
`parity-fixture`, cwd=`/fixture/demo`. REPL completion uses fake provider only.

Python child socket/DNS connects and external-process audit events are denied.
The earlier ui-1e3abe7 attempt tried gateway service integration: temporary-HOME
protection refused its unit write and systemd/linger calls failed. This was not
an authorized service setup. Clean captures add explicit process-launch denial;
full later raw retains the blocked attempts. Do not rerun unguarded or claim this
Python-level guard is a general OS security sandbox.

Docker PATH points to an empty directory on both sides. Python Docker snapshot
ends immediately before its next alternate-screen egress menu, after the missing
Docker notice. Python empty-gateway snapshot ends before deferred service
orchestration, after the no-platform result. Both boundaries were found at both
widths; `audit.json` lists prefix/full byte lengths. They select recorded instants,
not edited output. All later captured bytes/events are retained. No dynamic
normalization was performed: timestamps, random paths, IDs, geometry, colors,
static labels and counters remain visible.

## Recorder/adapter correction trail

- ui-1e3abe7: retained diagnostic rejection; process exit had incorrectly been
  treated as exhausted PTY output. An eight-iteration,60003-byte real-PTY regression
  observed RED (49152 bytes received), then GREEN after drain-until-EOF.
- ui-1c3c9dd: corrected recorder, but summary host omitted caller newline;
  intermediate packet retained without claiming complete direct image review.
- Adapter-only correction validated remotely before application:
  [34787994035/e40bf5d](https://github.com/Catzpro01/hermes-rust-version/actions/runs/34787994035),
  fmt/check/clippy/full workspace GREEN. Exact755-byte patch SHA-256
  `639bdad8cd9c6ec12d59a34c60b89f6c45538f73f47f8bef5ed869cac3990728`
  applied unchanged, producing final summary32/32-row GREEN comparison.
  This changes only the example's caller newline, not production banner rendering.
- Five recorder tests and eight workflow policy tests pass locally; remote CI
  covers the committed runtime. Audit integration and negative tamper check are
  supplementary. This is a limited direct Standards/Spec review, not an independent
  subagent or personal Matt Pocock approval.

## Reproduce without overwriting originals

Use source3b39bd7 and the verified Python reference; install the isolated Python
versions in `requirements-reference.txt` and the pinned renderer package manifest.
Set PYTHONPATH to those isolated dependencies. On a machine/runner with Rust:

```sh
cargo build -p hermes-cli --bin hermes-rs --example visual_summary
python3 scripts/capture_ui.py python "$REFERENCE" /NEW/python.json
python3 scripts/capture_ui.py rust target/debug/hermes-rs target/debug/examples/visual_summary /NEW/rust.json
python3 scripts/capture_ui.py pair /NEW/rust.json /NEW/python.json /NEW/paired-bundle.json
python3 scripts/capture_ui.py check /NEW/paired-bundle.json
node scripts/visual-renderer/render.cjs /NEW/paired-bundle.json /NEW/packet ui
# Copy that exact paired bundle into /NEW/packet/paired-bundle.json, then:
python3 scripts/audit_ui_evidence.py /NEW/packet
```

Node packages must be resolvable (e.g. NODE_PATH to the isolated pinned install);
Chromium shared libraries must be installed/resolvable (LD_LIBRARY_PATH if needed).
For this existing packet, run `sha256sum -c SHA256SUMS` from this directory and
`python3 scripts/audit_ui_evidence.py docs/hermes-ui-spec/017/evidence/ui-3b39bd7`
from the repo root. Audit does not modify files. Reproduction may differ in
random paths/timestamps; never replace immutable originals or hide those changes.

## Direct image inspection ledger — 48/48

Every link below was opened and inspected, at both widths. Group result labels
refer to the findings above, not automatic pixel equality or user acceptance.

| Scenario | 100×30 | 80×30 | Result |
|---|---|---|---|
| wizard-mode | [inspected](wizard-mode-100x30-bottom.png) | [inspected](wizard-mode-80x30-bottom.png) | V1 |
| wizard-full | [inspected](wizard-full-100x30-bottom.png) | [inspected](wizard-full-80x30-bottom.png) | V1 |
| wizard-blank | [inspected](wizard-blank-100x30-bottom.png) | [inspected](wizard-blank-80x30-bottom.png) | V1 |
| wizard-quick | [inspected](wizard-quick-100x30-bottom.png) | [inspected](wizard-quick-80x30-bottom.png) | V1 |
| wizard-model | [inspected](wizard-model-100x30-bottom.png) | [inspected](wizard-model-80x30-bottom.png) | V1 |
| wizard-terminal | [inspected](wizard-terminal-100x30-bottom.png) | [inspected](wizard-terminal-80x30-bottom.png) | V1 |
| wizard-local | [inspected](wizard-local-100x30-bottom.png) | [inspected](wizard-local-80x30-bottom.png) | V1 |
| wizard-docker | [inspected](wizard-docker-100x30-bottom.png) | [inspected](wizard-docker-80x30-bottom.png) | V1 |
| wizard-gateway | [inspected](wizard-gateway-100x30-bottom.png) | [inspected](wizard-gateway-80x30-bottom.png) | V1 |
| wizard-gateway-empty | [inspected](wizard-gateway-empty-100x30-bottom.png) | [inspected](wizard-gateway-empty-80x30-bottom.png) | V1 |
| wizard-gateway-token | [inspected](wizard-gateway-token-100x30-bottom.png) | [inspected](wizard-gateway-token-80x30-bottom.png) | V1 |
| wizard-tools | [inspected](wizard-tools-100x30-bottom.png) | [inspected](wizard-tools-80x30-bottom.png) | V1 |
| wizard-tools-toggle | [inspected](wizard-tools-toggle-100x30-bottom.png) | [inspected](wizard-tools-toggle-80x30-bottom.png) | V1 |
| wizard-cancel | [inspected](wizard-cancel-100x30-bottom.png) | [inspected](wizard-cancel-80x30-bottom.png) | V1 |
| picker-normal | [inspected](picker-normal-100x30-bottom.png) | [inspected](picker-normal-80x30-bottom.png) | V2 |
| picker-empty | [inspected](picker-empty-100x30-bottom.png) | [inspected](picker-empty-80x30-bottom.png) | Empty fixture match |
| picker-filter | [inspected](picker-filter-100x30-bottom.png) | [inspected](picker-filter-80x30-bottom.png) | V2 |
| picker-no-match | [inspected](picker-no-match-100x30-bottom.png) | [inspected](picker-no-match-80x30-bottom.png) | V2 |
| picker-delete | [inspected](picker-delete-100x30-bottom.png) | [inspected](picker-delete-80x30-bottom.png) | V2 |
| completion-command | [inspected](completion-command-100x30-bottom.png) | [inspected](completion-command-80x30-bottom.png) | V3 / host boundary |
| completion-subcommand | [inspected](completion-subcommand-100x30-bottom.png) | [inspected](completion-subcommand-80x30-bottom.png) | V3 / host boundary |
| completion-alternatives | [inspected](completion-alternatives-100x30-bottom.png) | [inspected](completion-alternatives-80x30-bottom.png) | V3 / host boundary |
| summary-zero | [inspected](summary-zero-100x30-bottom.png) | [inspected](summary-zero-80x30-bottom.png) | Summary fixture match + branding |
| summary-nonzero | [inspected](summary-nonzero-100x30-bottom.png) | [inspected](summary-nonzero-80x30-bottom.png) | Summary fixture match + branding |

**Closure remains blocked:** resolve in-scope unapproved differences, close any required field/full-CLI coverage gaps, rerun affected capture/CI, and obtain explicit user acceptance. No merge performed or authorized.
