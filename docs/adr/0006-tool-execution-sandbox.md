# ADR 0006: Tool execution sandbox is a process-level policy, off by default

- Status: accepted
- Date: 2026-09-13
- Scope: Spec 007 (tool execution sandbox)
- Related: `docs/SECURITY.md` (Spec 002 confirmation boundary), ADR 0005
  (never retry a denied tool), Spec 011 (MCP spawns are a separate surface)

## Context

Spec 002 shell tools run `sh -c <command>` with the full environment and
privileges of the Hermes-RS process. `docs/SECURITY.md` says so explicitly:
"They are not a substitute for an OS sandbox or container." Two concrete
gaps follow:

1. **Credential exposure.** Provider keys resolved from `key_env` live in the
   parent environment, so a model-issued `env` or `printenv` reads them.
2. **No resource boundary.** Beyond the 30 s tool timeout nothing bounds CPU,
   file size, process count, memory, or the size of output fed back into the
   context window.

Python Hermes solves this with Docker sandboxes (and an egress proxy). The
Rust port runs on a headless VM where Docker is not assumed, and the
workspace has a "no new crates for one call site" habit (ADR 0005).

## Decision

Introduce `hermes_core::tools::sandbox::SandboxPolicy` and route **every**
shell spawn (`shell`, `shell_readonly`) through one function, `run_shell`.

1. **Off by default.** `SandboxPolicy::inherit()` is the `Default` and what
   a config without `sandbox:` gets. It spawns exactly `sh -c <command>` with
   the inherited env and cwd — byte-for-byte Spec 002. The legacy
   constructors `ShellTool::new` / `ShellReadonlyTool::new` keep that policy;
   `.with_sandbox(policy)` opts in.
2. **Process-level boundary, no new crates.**
   - Environment: `env_clear()` + copy of an **allowlist of names**
     (`PATH`, `HOME`, locale, `TERM`, …, extended by config). Provider keys
     never reach the child unless the user names them.
   - Working directory: the tool root.
   - Output: combined stdout+stderr capped (default 64 KiB) with an explicit
     truncation marker.
   - Resource limits: POSIX `ulimit` (`-t -f -u -v`) in a wrapper shell.
     The model's command is passed as **`$1` positional**, never spliced
     into the wrapper script, so quoting cannot skip the prologue.
   - Network: optional `unshare --user --map-root-user --net` (util-linux).
3. **Fail closed.** If `network: deny` is requested and `unshare` is missing
   or user namespaces are disabled, the call fails with a clear error — it
   never falls back to running with network.
4. **Config validated at load.** A malformed `sandbox:` section (`network:
   dney`, `cpu_seconds: 0`, `env_allowlist: ["A=B"]`) is a
   `ConfigError::SandboxInvalid` at startup, not a silent no-op.
5. **Gates stay in order.** Blocklist → confirmation → sandbox. The sandbox
   is defense in depth; it does not relax the readonly blocklist or the
   confirmation requirement, and ADR 0005 still applies (denied is never
   retried).
6. **Visibility without leakage.** `/sandbox` (REPL) and `hermes info` print
   `SandboxPolicy::summary()`: variable *names* and numeric limits only.

### Explicitly out of scope

- Container/VM isolation, seccomp, Landlock, filesystem read denial
  (`read_file`/`list_dir` keep their own root jail; a sandboxed shell can
  still read files the user can read).
- Sandboxing MCP children (Spec 011 has its own trust model: the user
  configured that command) and `write_file` (already jailed + confirmed).
- Windows. The wrapper assumes a POSIX `sh`; on non-Unix the policy is
  accepted but only env/cwd/output caps take effect.

## Consequences

- A user can now stop model commands from seeing API keys with three lines
  of YAML, and bound runaway commands without Docker.
- `ulimit -u` is per-user on Linux, so `max_processes` also counts the
  parent's processes; document rather than "fix".
- `network: deny` depends on host support (unprivileged user namespaces);
  the fail-closed rule makes that visible instead of dangerous.
- Should stronger isolation be needed later, `run_shell` is the single seam
  where a container or Landlock backend would plug in.

## Amendment 2026-09-13 — default on (Spec 007b, per /ask-matt)

The original decision shipped the sandbox **opt-in** so that Spec 007 was a
zero-regression change. The follow-up ticket flips the default:

- `SandboxPolicy::from_config(None, root)` → `strict(root)` (was `inherit`).
- `SandboxConfig.enabled` is `Option<bool>`; only an explicit `false` opts
  out. A `sandbox:` section without `enabled` is on.
- New global CLI flag `--no-sandbox` → `inherit`, overriding config. All
  three frontends (REPL, TUI worker, `hermes info`) resolve through one
  helper, `tools::sandbox::resolve(cfg, root, no_sandbox)`.
- Legacy constructors (`ShellTool::new`, `ShellReadonlyTool::new`) still
  default to `inherit`; the *CLI* default changed, not the library type
  default, so embedders are unaffected.

Rationale: a sandbox nobody turns on protects nobody; the longer `off`
stays the default, the more surprising the flip becomes. Users who need the
parent environment inside shell tools have two explicit escape hatches.
