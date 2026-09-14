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
