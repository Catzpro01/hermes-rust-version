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

## Provider credentials (Spec 005 — per-provider `key_env` routing)

Provider API keys are resolved by `ProviderRegistry` when a provider is built.
This is a new credential surface, so its behavior is pinned below and governed
by the fallback chain in `crates/hermes-core/src/provider/registry.rs`
(`resolve_api_key`).

### Fallback chain (per configured provider)

1. `key_env` declared and its environment variable holds a non-empty value →
   use it.
2. `key_env` declared but the variable is unset/empty → **error** naming the
   variable. It must NOT fall back to `model.api_key`.
3. `key_env` absent → fall back to the global `model.api_key`, else error.

The legacy single-provider path (`model_level_fallback`, used only when no
`providers:` section selects the active provider) keeps its historical
`OPENAI_API_KEY` → `HERMES_API_KEY` → `model.api_key` order.

### STRIDE

- **Spoofing.** A `key_env` pin could be pointed at another provider's variable.
  The resolution only ever sends the key named by the active provider's own
  pin (or the explicit global `model.api_key`); it never guesses from
  well-known variable names for a configured provider. Because a pinned but
  empty `key_env` errors instead of falling back, one provider's key can never
  be silently forwarded to another provider's endpoint (no cross-provider
  credential leakage).
- **Tampering / Repudiation.** Credentials are wrapped in `SecretString` from
  resolution through the HTTP bearer header; they never transit as plain
  `String` outside the provider. Config is read-only at runtime.
- **Information disclosure.** This is the most common leak vector. Error and
  log messages name the missing *variable* (`environment variable 'X' is not
  set or empty`), never its value. The registry and `HttpProvider` redact
  credentials on every error/output path; the tests
  (`missing_key_env_names_the_variable_not_a_value`,
  `pinned_but_empty_key_env_does_not_fall_back_to_model_key`) assert no value
  ever appears in a message.
- **Elevation of privilege / DoS.** No new execution or network surface is
  introduced by key resolution; a failed resolution only prevents that one
  provider from being constructed and does not affect an already-active
  provider (rollback semantics of `/provider`).
