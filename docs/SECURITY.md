# Hermes-RS Tool Security Model

## Scope

Spec 002 tools execute with the permissions of the Hermes-RS process. They are not a substitute for an OS sandbox or container. Never run the CLI with untrusted prompts in a privileged account.

## Policy tiers

| Tool | Policy | Boundary |
|---|---|---|
| `read_file` | Always safe | Canonical path must remain inside cwd; 100 KB limit |
| `list_dir` | Always safe | Canonical path must remain inside cwd; 500 entry limit |
| `write_file` | Allowlist + confirmation | cwd-only, atomic temp-file rename, explicit `y`/`yes` |
| `shell_readonly` | Blocklist + confirmation | blocks destructive commands, pipes, redirects; 30 second timeout |

## Confirmation

Write and shell operations require an injected confirmation callback. The CLI defaults to deny: only `y` or `yes` authorizes an operation. EOF, malformed input, or callback failure denies execution.

## Path safety

Read and write tools reject absolute paths outside the root, lexical `..` traversal, invalid components, and symlink escapes for existing targets. New writes are staged inside the canonical root and renamed atomically.

## Cancellation and limits

Every tool receives a `CancellationToken`. Cancellation prevents new work and removes pending temporary writes. Shell commands have a bounded timeout. Output limits prevent accidental unbounded file/directory responses.

## Shell blocklist

Readonly shell blocks `rm`, `sudo`, `chmod`, `chown`, `curl`, `wget`, `dd`, `mkfs`, pipes, and redirects. This is defense-in-depth, not a complete command safety proof. `unsafe_mode` is an explicit API escape hatch and must never be enabled for untrusted input.

## Threat model

The model may generate malicious, surprising, or destructive tool arguments. The registry and each tool must independently enforce policy; prompt instructions are not authorization. Tool results are untrusted data and must not be interpreted as permission to broaden policy.

## Review requirements

Any new tool must define its root, input validation, output limits, timeout/cancellation behavior, confirmation policy, and tests for traversal, denial, and failure before registration in the CLI.
