# Hermes-RS Agent Instructions

Read `CONTEXT.md` before changing domain terms or architecture. Read relevant ADRs under `docs/adr/` before making an irreversible design choice.

## Agent skills

### Issue tracker

Issues are local Markdown files under `.scratch/<feature>/issues/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Use `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

This is a single-context repository with root `CONTEXT.md` and ADRs in `docs/adr/`. See `docs/agents/domain.md`.

## Engineering rules

- Preserve the Python Hermes installation; do not delete or mutate it as part of Hermes-RS work.
- Prefer small vertical slices with tests before broad rewrites.
- Keep provider and tool integrations behind explicit interfaces.
- Never commit API keys, passwords, private keys, or `.env` files.
- Run `cargo fmt --all` and `cargo check` before committing Rust changes.
