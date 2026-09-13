# 01: Install the complete official Matt Pocock skills collection

- Status: DONE; installation checkpoint CI passed
- Label: `ready-for-agent`
- Owner: Arena agent
- Date: 2026-09-14
- Request: download Matt Pocock's skills and apply their instructions.
- Blocking edges: none for installation; individual skill capability limits
  are recorded in `docs/agents/matt-pocock-skills.md`.

## Scope and acceptance

- [x] Locate the author's repository rather than fabricate an `/ask-matt` skill.
- [x] Pin `mattpocock/skills` at `3cca18b368ae95cdbdebbff572ccafa662551015`.
- [x] Preserve all 164 upstream files verbatim, with license and blob manifest.
- [x] Link all 37 skills locally, preserving frontmatter and invocation policy.
- [x] Distinguish stable (25), in-progress (8), and misc (4) skills; retain the
  upstream warning that `retro` is a stub.
- [x] Connect the skill catalog to AGENTS, MEMORY, and progress inheritance.
- [x] Verify source completeness/hashes, symlinks, metadata, and existing QA tests.
- [x] Apply the original `/ask-matt` routing to the current closure handoff.
- [x] Commit/push this checkpoint and inspect its actual CI result: `b959ab3`,
  run `34777846810` SUCCESS (576 Rust tests and five QA tests).

## Application and limits

The integration document is the single source for project adaptations and
the complete linked catalog. Original skill bodies are not rewritten.
The repository already has the setup skill's issue tracker, triage-label,
and domain-layout artifacts; they remain unchanged rather than being reset.

`/ask-matt` is a router, not a contact/reviewer identity. It recommends
`/grill-with-docs` for the outstanding Spec 017 §J.7 evidence decision.
It neither grants sign-off nor starts that user-invoked flow automatically.
The prior missing-skill blocker is resolved without requesting a private
skill attachment or claiming a personal Matt verdict.

Native Skill tools, independent review subagents, and some external CLIs are
not available in this session. Applicable instructions can be read and
followed; workflows requiring unavailable tools need an explicitly agreed
alternative. Installation is not a claim that every workflow was executed.

No runtime Rust change, global installation, credential configuration,
Git hook, branch switch, PR, or merge is part of this task.
