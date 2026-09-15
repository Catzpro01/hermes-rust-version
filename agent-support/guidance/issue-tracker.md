# Issue tracker

This repository uses **local Markdown issues**. Each issue lives under:

```text
.scratch/<feature>/issues/<issue-slug>.md
```

Issues are tracked in Git alongside the source. There is no external issue service configured yet. A ticket should state its status, owner, acceptance criteria, and blocking edges.

## Workflow

1. Create an issue file under the feature directory.
2. Add `needs-triage` while the request is still raw.
3. Move to `ready-for-agent` only after scope and acceptance criteria are clear.
4. Record decisions in `CONTEXT.md` or `docs/adr/` when they affect the architecture.

## Wayfinding operations

This tracker has no native issue relationships, so Wayfinder maps and decision
tickets (see `agent-support/skills/wayfinder/SKILL.md`) use these local
conventions. This is a fallback expression of the same model, not a new tracker.

- **Identity**: the map is one issue file
  `.scratch/<feature>/issues/WAYFINDER-<slug>.md` carrying `Label:
  wayfinder:map`. Each decision ticket is a normal issue file in the same
  feature directory. Every wayfinding file starts with a metadata block:
  `Status: OPEN|CLOSED <date>`, `Type: wayfinder:<research|prototype|grilling|task>`,
  `HITL: yes|no`, `Owner:` (claim), `Parent map:` (link, tickets only),
  `Blocked-by:` (ticket links, may be empty).
- **Claim**: a session claims a ticket by writing its name into `Owner:`
  **before** any work. An empty `Owner:` means unclaimed.
- **Blocking**: `Blocked-by:` lists the tickets that must close first. A ticket
  is **unblocked** when every listed ticket is `CLOSED`.
- **Frontier**: the map's open (`Status: OPEN`), unblocked, unclaimed children.
  Find it by reading the children's metadata blocks; the map itself only lists
  closed decisions.
- **Resolution**: set `Status: CLOSED <date>`, append a `## Resolution` section
  with the dated answer (HITL answers record the exchange's outcome; assets are
  linked, not pasted), then append one line to the map's `Decisions so far`.
- **Discipline**: charting creates/wires tickets but resolves none; at most one
  ticket resolved per session (research excepted); HITL tickets resolve only
  through the live human exchange, never by the agent standing in.
