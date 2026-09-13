//! Spec 007 — tool execution sandbox.
//!
//! A [`SandboxPolicy`] describes the boundary a shell tool runs inside:
//! working directory, environment allowlist, resource limits, output cap,
//! and network isolation. Both `shell` and `shell_readonly` execute through
//! [`run_shell`], so there is exactly one place where a child process is
//! spawned for a model-issued command.
//!
//! Design constraints (see ADR 0006):
//!
//! - **Off by default / zero regression.** [`SandboxPolicy::inherit`] is the
//!   default and reproduces the pre-007 behaviour byte-for-byte:
//!   `sh -c <command>` with the parent's environment and cwd, no limits
//!   beyond the tool timeout. A config without a `sandbox:` section gets
//!   exactly that.
//! - **No new crates.** Resource limits are applied with the POSIX `ulimit`
//!   builtin of the wrapper shell (`sh -c 'ulimit …; exec sh -c "$1"' sh
//!   <command>`), and network isolation uses `unshare(1)` when requested.
//!   The model's command is always passed as a **positional argument**, never
//!   interpolated into the wrapper script, so it cannot break out of the
//!   `ulimit` prologue by quoting.
//! - **Fail closed.** When the policy asks for something the host cannot
//!   provide (e.g. `network: deny` without a working `unshare`), execution
//!   fails with a clear error instead of silently running unconfined.
//! - **Secrets stay in the parent.** With the sandbox enabled the child's
//!   environment is cleared and only the allowlisted variables are copied.
//!   Provider keys (`*_API_KEY`, `HERMES_*` secrets) are therefore never
//!   visible to a model-issued command unless the user allowlists them.
use super::ToolError;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{process::Command, time::timeout};
use tokio_util::sync::CancellationToken;

/// Default environment variables forwarded into a sandboxed child. Chosen so
/// ordinary tooling still works (`PATH`, locale, terminal) without leaking
/// credentials. Users extend it via `sandbox.env_allowlist`.
pub const DEFAULT_ENV_ALLOWLIST: &[&str] = &[
    "PATH", "HOME", "LANG", "LC_ALL", "LC_CTYPE", "TERM", "TZ", "USER", "SHELL", "TMPDIR",
];

/// Default cap on combined stdout+stderr returned to the model (64 KiB).
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// Marker appended when the child's output was cut at `max_output_bytes`.
pub const OUTPUT_TRUNCATED_MARKER: &str = "\n[output truncated by sandbox]";

/// Environment variable set inside every sandboxed child so scripts can
/// detect the boundary. Never set on the inherit path.
pub const SANDBOX_ENV_FLAG: &str = "HERMES_SANDBOX";

/// Network posture for sandboxed commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkPolicy {
    /// Child shares the parent's network (default).
    #[default]
    Inherit,
    /// Child runs in a fresh, loopback-only network namespace via
    /// `unshare --user --map-root-user --net`. Requires unprivileged user
    /// namespaces; when unavailable the tool call fails closed.
    Deny,
}

/// Resource limits applied through the wrapper shell's `ulimit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResourceLimits {
    /// `ulimit -t`: CPU seconds before the kernel kills the child.
    pub cpu_seconds: Option<u64>,
    /// `ulimit -f`: largest file the child may create, in KiB.
    pub max_file_size_kb: Option<u64>,
    /// `ulimit -u`: max user processes (fork-bomb guard). Note this is a
    /// per-user limit on Linux, so it also counts the parent's processes.
    pub max_processes: Option<u64>,
    /// `ulimit -v`: virtual memory ceiling in KiB.
    pub max_memory_kb: Option<u64>,
}

impl ResourceLimits {
    /// True when no limit is set (the wrapper prologue is then omitted).
    pub fn is_empty(&self) -> bool {
        self.cpu_seconds.is_none()
            && self.max_file_size_kb.is_none()
            && self.max_processes.is_none()
            && self.max_memory_kb.is_none()
    }

    /// The `ulimit` prologue for the wrapper shell. Numeric only — nothing
    /// user-controlled is interpolated here.
    pub fn ulimit_prologue(&self) -> String {
        let mut parts = Vec::new();
        if let Some(v) = self.cpu_seconds {
            parts.push(format!("ulimit -t {v}"));
        }
        if let Some(v) = self.max_file_size_kb {
            parts.push(format!("ulimit -f {v}"));
        }
        if let Some(v) = self.max_processes {
            parts.push(format!("ulimit -u {v}"));
        }
        if let Some(v) = self.max_memory_kb {
            parts.push(format!("ulimit -v {v}"));
        }
        parts.join(" && ")
    }
}

/// The boundary a shell tool runs inside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPolicy {
    /// Master switch. `false` == [`SandboxPolicy::inherit`] semantics even if
    /// other fields are set (they are ignored), so a config that only says
    /// `enabled: false` is inert.
    pub enabled: bool,
    /// Working directory for the child (normally the tool root). `None`
    /// inherits the parent's cwd.
    pub working_dir: Option<PathBuf>,
    /// Variables copied from the parent environment. Everything else is
    /// dropped (`env_clear`). Ignored when `enabled` is false.
    pub env_allowlist: Vec<String>,
    /// Cap on combined stdout+stderr bytes returned to the model.
    pub max_output_bytes: usize,
    pub limits: ResourceLimits,
    pub network: NetworkPolicy,
}

/// CLI resolution (Spec 007b): `--no-sandbox` wins over config, otherwise
/// [`SandboxPolicy::from_config`] (default strict). One call site per
/// frontend (REPL, TUI worker, `hermes info`) so they can never disagree.
pub fn resolve(
    cfg: Option<&crate::config::SandboxConfig>,
    root: &Path,
    no_sandbox: bool,
) -> SandboxPolicy {
    if no_sandbox {
        SandboxPolicy::inherit()
    } else {
        SandboxPolicy::from_config(cfg, root)
    }
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self::inherit()
    }
}

impl SandboxPolicy {
    /// Pre-007 behaviour: no boundary beyond the tool timeout. This is what
    /// every caller gets when `sandbox:` is absent from `config.yaml`.
    pub fn inherit() -> Self {
        Self {
            enabled: false,
            working_dir: None,
            env_allowlist: Vec::new(),
            max_output_bytes: usize::MAX,
            limits: ResourceLimits::default(),
            network: NetworkPolicy::Inherit,
        }
    }

    /// Secure defaults rooted at `root`: cleared env (default allowlist),
    /// cwd jail, 64 KiB output cap, network inherited (deny needs opt-in
    /// because it depends on host support), no rlimits (opt-in per field).
    pub fn strict(root: impl Into<PathBuf>) -> Self {
        Self {
            enabled: true,
            working_dir: Some(root.into()),
            env_allowlist: DEFAULT_ENV_ALLOWLIST
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
            limits: ResourceLimits::default(),
            network: NetworkPolicy::Inherit,
        }
    }

    /// Build from the optional `sandbox:` config section. `None` yields
    /// [`SandboxPolicy::strict`] (default-on); `enabled: false` yields
    /// [`SandboxPolicy::inherit`]. Fields the user
    /// omits fall back to [`SandboxPolicy::strict`] defaults; `env_allowlist`
    /// in config **extends** the default allowlist rather than replacing it,
    /// so a user cannot accidentally lose `PATH`.
    pub fn from_config(cfg: Option<&crate::config::SandboxConfig>, root: &Path) -> Self {
        // Spec 007b (per /ask-matt): default ON. No section -> strict
        // defaults; only an explicit `enabled: false` opts out.
        let Some(cfg) = cfg else {
            return Self::strict(root);
        };
        if cfg.enabled == Some(false) {
            return Self::inherit();
        }
        let mut policy = Self::strict(root);
        for extra in &cfg.env_allowlist {
            if !policy.env_allowlist.iter().any(|e| e == extra) {
                policy.env_allowlist.push(extra.clone());
            }
        }
        if let Some(kb) = cfg.max_output_kb {
            policy.max_output_bytes = (kb as usize).saturating_mul(1024).max(1);
        }
        policy.limits = ResourceLimits {
            cpu_seconds: cfg.cpu_seconds,
            max_file_size_kb: cfg.max_file_size_kb,
            max_processes: cfg.max_processes,
            max_memory_kb: cfg.max_memory_kb,
        };
        policy.network = match cfg.network.as_deref() {
            Some("deny") => NetworkPolicy::Deny,
            _ => NetworkPolicy::Inherit,
        };
        policy
    }

    /// The variables that will reach the child, resolved against `parent`
    /// (sorted for deterministic display/tests). Pure.
    pub fn resolve_env(
        &self,
        parent: impl IntoIterator<Item = (String, String)>,
    ) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        if !self.enabled {
            return env; // inherit path: `Command` keeps the parent env itself
        }
        for (k, v) in parent {
            if self.env_allowlist.contains(&k) {
                env.insert(k, v);
            }
        }
        env.insert(SANDBOX_ENV_FLAG.to_owned(), "1".to_owned());
        env
    }

    /// One-line human summary for `/sandbox` and `hermes info`. Contains no
    /// environment *values*, only names and numbers.
    pub fn summary(&self) -> String {
        if !self.enabled {
            return "sandbox: off (inherit)".to_owned();
        }
        let mut parts = vec![format!(
            "cwd={}",
            self.working_dir
                .as_deref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "inherit".to_owned())
        )];
        parts.push(format!("env={}", self.env_allowlist.join(",")));
        parts.push(format!(
            "output<={}KiB",
            self.max_output_bytes.div_ceil(1024)
        ));
        if let Some(v) = self.limits.cpu_seconds {
            parts.push(format!("cpu={v}s"));
        }
        if let Some(v) = self.limits.max_file_size_kb {
            parts.push(format!("fsize={v}KiB"));
        }
        if let Some(v) = self.limits.max_processes {
            parts.push(format!("nproc={v}"));
        }
        if let Some(v) = self.limits.max_memory_kb {
            parts.push(format!("mem={v}KiB"));
        }
        parts.push(format!(
            "network={}",
            match self.network {
                NetworkPolicy::Inherit => "inherit",
                NetworkPolicy::Deny => "deny",
            }
        ));
        format!("sandbox: on | {}", parts.join(" | "))
    }

    /// The program + argv that will be spawned for `command`. Pure and
    /// pinned by tests: this is the whole "how does the model's string reach
    /// a shell" story.
    ///
    /// - inherit / no limits / no network isolation → `sh -c <command>`
    /// - limits → `sh -c 'ulimit …; exec sh -c "$1"' hermes-sandbox <command>`
    /// - network deny → prefix `unshare --user --map-root-user --net --`
    pub fn build_argv(&self, command: &str) -> (String, Vec<String>) {
        let mut program = "sh".to_owned();
        let mut argv: Vec<String> = Vec::new();
        if self.enabled && !self.limits.is_empty() {
            argv.push("-c".to_owned());
            argv.push(format!(
                "{} && exec sh -c \"$1\"",
                self.limits.ulimit_prologue()
            ));
            argv.push("hermes-sandbox".to_owned()); // $0 of the wrapper
            argv.push(command.to_owned()); // $1: never interpolated
        } else {
            argv.push("-c".to_owned());
            argv.push(command.to_owned());
        }
        if self.enabled && self.network == NetworkPolicy::Deny {
            let mut wrapped = vec![
                "--user".to_owned(),
                "--map-root-user".to_owned(),
                "--net".to_owned(),
                "--".to_owned(),
                program,
            ];
            wrapped.extend(argv);
            program = "unshare".to_owned();
            argv = wrapped;
        }
        (program, argv)
    }
}

/// Cut `bytes` at `max`, appending [`OUTPUT_TRUNCATED_MARKER`] when cut.
/// Never splits a UTF-8 sequence (lossy decode handles the tail).
pub fn cap_output(bytes: &[u8], max: usize) -> String {
    if bytes.len() <= max {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    let mut s = String::from_utf8_lossy(&bytes[..max]).into_owned();
    s.push_str(OUTPUT_TRUNCATED_MARKER);
    s
}

/// Result of a sandboxed shell run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellOutcome {
    pub content: String,
    pub success: bool,
}

/// Spawn `command` inside `policy`, bounded by `tool_timeout` and `cancel`.
/// This is the **only** spawn site for `shell` / `shell_readonly`.
pub async fn run_shell(
    policy: &SandboxPolicy,
    command: &str,
    tool_timeout: Duration,
    cancel: CancellationToken,
) -> Result<ShellOutcome, ToolError> {
    let (program, argv) = policy.build_argv(command);
    let mut cmd = Command::new(&program);
    cmd.args(&argv).stdin(Stdio::null()).kill_on_drop(true);
    if policy.enabled {
        cmd.env_clear();
        for (k, v) in policy.resolve_env(std::env::vars()) {
            cmd.env(k, v);
        }
        if let Some(dir) = &policy.working_dir {
            cmd.current_dir(dir);
        }
    }
    let child = cmd.output();
    let output = tokio::select! {
        _ = cancel.cancelled() => return Err(ToolError::Cancelled),
        r = timeout(tool_timeout, child) => r
            .map_err(|_| ToolError::Timeout(tool_timeout))?
            .map_err(|e| {
                if program == "unshare" && e.kind() == std::io::ErrorKind::NotFound {
                    ToolError::Failed(
                        "sandbox network=deny requires `unshare` (util-linux) on PATH".into(),
                    )
                } else {
                    ToolError::Failed(e.to_string())
                }
            })?,
    };
    // Fail closed: `unshare` itself failing (no user namespaces) must not
    // look like the command ran. util-linux exits 1 and prints
    // "unshare: ..." on stderr before exec'ing anything.
    if program == "unshare" && !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        if err.starts_with("unshare:") {
            return Err(ToolError::Failed(format!(
                "sandbox network=deny unavailable on this host: {}",
                err.trim()
            )));
        }
    }
    let combined = [output.stdout, output.stderr].concat();
    Ok(ShellOutcome {
        content: cap_output(&combined, policy.max_output_bytes),
        success: output.status.success(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherit_is_the_default_and_spawns_plain_sh() {
        let p = SandboxPolicy::default();
        assert_eq!(p, SandboxPolicy::inherit());
        assert!(!p.enabled);
        assert_eq!(
            p.build_argv("printf ok"),
            (
                "sh".to_owned(),
                vec!["-c".to_owned(), "printf ok".to_owned()]
            )
        );
        assert!(p.resolve_env([("A".to_owned(), "b".to_owned())]).is_empty());
        assert_eq!(p.summary(), "sandbox: off (inherit)");
    }

    #[test]
    fn disabled_fields_are_inert() {
        // `enabled: false` with limits set must still be plain `sh -c`.
        let mut p = SandboxPolicy::strict("/tmp");
        p.enabled = false;
        p.limits.cpu_seconds = Some(1);
        p.network = NetworkPolicy::Deny;
        assert_eq!(p.build_argv("x").0, "sh");
        assert_eq!(p.build_argv("x").1.len(), 2);
        assert!(p
            .resolve_env([("PATH".to_owned(), "/bin".to_owned())])
            .is_empty());
    }

    #[test]
    fn strict_filters_env_and_flags_sandbox() {
        let p = SandboxPolicy::strict("/tmp");
        let env = p.resolve_env([
            ("PATH".to_owned(), "/bin".to_owned()),
            ("OPENAI_API_KEY".to_owned(), "sk-secret".to_owned()),
            ("HOME".to_owned(), "/h".to_owned()),
        ]);
        assert_eq!(env.get("PATH").map(String::as_str), Some("/bin"));
        assert_eq!(env.get("HOME").map(String::as_str), Some("/h"));
        assert!(!env.contains_key("OPENAI_API_KEY"));
        assert_eq!(env.get(SANDBOX_ENV_FLAG).map(String::as_str), Some("1"));
        assert!(!p.summary().contains("sk-secret"));
    }

    #[test]
    fn limits_use_a_positional_wrapper_never_interpolation() {
        let mut p = SandboxPolicy::strict("/tmp");
        p.limits = ResourceLimits {
            cpu_seconds: Some(5),
            max_file_size_kb: Some(1024),
            max_processes: Some(64),
            max_memory_kb: None,
        };
        let hostile = "echo $1; \"; rm -rf /; '";
        let (prog, argv) = p.build_argv(hostile);
        assert_eq!(prog, "sh");
        assert_eq!(argv[0], "-c");
        assert_eq!(
            argv[1],
            "ulimit -t 5 && ulimit -f 1024 && ulimit -u 64 && exec sh -c \"$1\""
        );
        assert_eq!(argv[2], "hermes-sandbox");
        assert_eq!(argv[3], hostile, "command travels as $1 verbatim");
        assert!(!argv[1].contains("rm -rf"), "never spliced into the script");
    }

    #[test]
    fn network_deny_prefixes_unshare() {
        let mut p = SandboxPolicy::strict("/tmp");
        p.network = NetworkPolicy::Deny;
        let (prog, argv) = p.build_argv("id");
        assert_eq!(prog, "unshare");
        assert_eq!(
            argv,
            vec!["--user", "--map-root-user", "--net", "--", "sh", "-c", "id"]
        );
        assert!(p.summary().ends_with("network=deny"));
    }

    #[test]
    fn output_is_capped_with_marker() {
        assert_eq!(cap_output(b"abc", 3), "abc");
        let s = cap_output(b"abcdef", 3);
        assert_eq!(s, format!("abc{OUTPUT_TRUNCATED_MARKER}"));
        // Cutting inside a multi-byte char never panics.
        let s = cap_output("héllo".as_bytes(), 2);
        assert!(s.ends_with(OUTPUT_TRUNCATED_MARKER));
    }

    #[test]
    fn from_config_default_is_strict_and_explicit_false_is_inherit() {
        let root = Path::new("/tmp");
        // Spec 007b: no section -> strict defaults rooted at `root`.
        assert_eq!(
            SandboxPolicy::from_config(None, root),
            SandboxPolicy::strict(root)
        );
        // A section without `enabled` is also on.
        let bare = crate::config::SandboxConfig::default();
        assert!(SandboxPolicy::from_config(Some(&bare), root).enabled);
        let off = crate::config::SandboxConfig {
            enabled: Some(false),
            cpu_seconds: Some(1),
            ..Default::default()
        };
        assert_eq!(
            SandboxPolicy::from_config(Some(&off), root),
            SandboxPolicy::inherit()
        );
    }

    #[test]
    fn resolve_flag_overrides_config() {
        let root = Path::new("/tmp");
        assert_eq!(resolve(None, root, true), SandboxPolicy::inherit());
        assert!(resolve(None, root, false).enabled);
        let on = crate::config::SandboxConfig {
            enabled: Some(true),
            ..Default::default()
        };
        assert_eq!(resolve(Some(&on), root, true), SandboxPolicy::inherit());
    }

    #[test]
    fn from_config_extends_allowlist_and_maps_fields() {
        let cfg = crate::config::SandboxConfig {
            enabled: Some(true),
            network: Some("deny".into()),
            cpu_seconds: Some(3),
            max_file_size_kb: Some(10),
            max_processes: Some(8),
            max_memory_kb: Some(262144),
            max_output_kb: Some(2),
            env_allowlist: vec!["CARGO_HOME".into(), "PATH".into()],
        };
        let p = SandboxPolicy::from_config(Some(&cfg), Path::new("/w"));
        assert!(p.enabled);
        assert_eq!(p.working_dir.as_deref(), Some(Path::new("/w")));
        assert_eq!(p.network, NetworkPolicy::Deny);
        assert_eq!(p.limits.cpu_seconds, Some(3));
        assert_eq!(p.limits.max_memory_kb, Some(262144));
        assert_eq!(p.max_output_bytes, 2048);
        assert!(p.env_allowlist.contains(&"CARGO_HOME".to_owned()));
        assert_eq!(
            p.env_allowlist.iter().filter(|e| *e == "PATH").count(),
            1,
            "no duplicate"
        );
        assert!(
            p.env_allowlist.contains(&"HOME".to_owned()),
            "defaults kept"
        );
    }

    #[tokio::test]
    async fn run_shell_inherit_matches_legacy_behaviour() {
        let out = run_shell(
            &SandboxPolicy::inherit(),
            "printf ok; printf err 1>&2",
            Duration::from_secs(5),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(out.content, "okerr");
        assert!(out.success);
    }

    #[tokio::test]
    async fn run_shell_strict_hides_parent_secrets_and_jails_cwd() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HERMES_TEST_SECRET_007", "leak-me");
        let p = SandboxPolicy::strict(dir.path());
        let out = run_shell(
            &p,
            "printf '%s|%s|' \"$HERMES_TEST_SECRET_007\" \"$HERMES_SANDBOX\"; pwd",
            Duration::from_secs(5),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        let canon = dir.path().canonicalize().unwrap();
        assert_eq!(out.content, format!("|1|{}\n", canon.display()));
        std::env::remove_var("HERMES_TEST_SECRET_007");
    }

    #[tokio::test]
    async fn run_shell_caps_output() {
        let mut p = SandboxPolicy::strict(std::env::temp_dir());
        p.max_output_bytes = 10;
        let out = run_shell(
            &p,
            "printf '0123456789ABCDEF'",
            Duration::from_secs(5),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(out.content, format!("0123456789{OUTPUT_TRUNCATED_MARKER}"));
    }

    #[tokio::test]
    async fn run_shell_file_size_limit_is_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = SandboxPolicy::strict(dir.path());
        p.limits.max_file_size_kb = Some(1);
        // 8 KiB write into a 1 KiB limit: SIGXFSZ / short write, non-zero.
        let out = run_shell(
            &p,
            "head -c 8192 /dev/zero > big.bin",
            Duration::from_secs(5),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert!(!out.success, "write beyond fsize must fail: {out:?}");
        let size = std::fs::metadata(dir.path().join("big.bin"))
            .map(|m| m.len())
            .unwrap_or(0);
        assert!(size <= 1024, "file capped at 1 KiB, got {size}");
    }

    #[tokio::test]
    async fn run_shell_timeout_and_cancel_still_apply_under_sandbox() {
        let p = SandboxPolicy::strict(std::env::temp_dir());
        let e = run_shell(
            &p,
            "sleep 5",
            Duration::from_millis(50),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
        assert!(matches!(e, ToolError::Timeout(_)));
        let cancel = CancellationToken::new();
        cancel.cancel();
        let e = run_shell(&p, "sleep 5", Duration::from_secs(5), cancel)
            .await
            .unwrap_err();
        assert!(matches!(e, ToolError::Cancelled));
    }
}
