# 007-01..05: Tool execution sandbox
**Status:** DONE — menunggu review Matt. Label: `ready-for-human`.

## Cakupan
- **Core** `crates/hermes-core/src/tools/sandbox.rs` (baru):
  `SandboxPolicy { enabled, working_dir, env_allowlist, max_output_bytes,
  limits: ResourceLimits, network: NetworkPolicy }`; `inherit()` (default),
  `strict(root)`, `from_config(Option<&SandboxConfig>, root)`;
  `resolve_env` (pure), `build_argv` (pure, pinned), `summary()`,
  `cap_output`, `run_shell` (satu-satunya spawn site).
- **Shell tools** `shell.rs`: field `sandbox`, `with_sandbox(policy)`,
  `sandbox()`; kedua tool memanggil `run_shell`. Konstruktor `new` tetap
  inherit.
- **Config** `SandboxConfig` (`enabled, network, cpu_seconds,
  max_file_size_kb, max_processes, max_memory_kb, max_output_kb,
  env_allowlist`) + `validate()`; `load_config` → `ConfigError::SandboxInvalid`.
- **CLI**: REPL & TUI worker membangun policy dari config dan root tool;
  `/sandbox` mencetak `summary()`; `hermes info` menambah baris `sandbox: …`;
  `/help` memuat `/sandbox`.
- **Docs**: ADR 0006, SECURITY (STRIDE), ROADMAP (Fase 4 Spec 007 → Done),
  tools_summary, PARITY, CONTEXT.

## Keputusan desain (ringkas; lengkap di ADR 0006)
- `ulimit` lewat wrapper `sh -c '<prologue> && exec sh -c "$1"'
  hermes-sandbox <cmd>` — perintah model selalu argv, tidak pernah
  diinterpolasi.
- `unshare --user --map-root-user --net` untuk `network: deny`; jika binary
  tidak ada / userns dimatikan → `ToolError::Failed` (fail closed).
- `env_allowlist` config **menambah** default (tidak mengganti) agar `PATH`
  tidak hilang tanpa sengaja.
- Di luar cakupan: container/Landlock/seccomp, penolakan baca FS, anak MCP,
  `write_file`, Windows.

## Bukti
- Unit (`sandbox.rs`, 13): default==inherit & argv `sh -c`; field inert
  saat `enabled:false`; env difilter + flag; wrapper posisional dengan
  perintah hostile; prefix `unshare`; cap output + marker (aman multi-byte);
  `from_config` none/disabled/extends; `run_shell` inherit == legacy,
  strict menyembunyikan secret + cwd jail, cap output, `fsize` rlimit,
  timeout & cancel tetap berlaku.
- Unit (`shell.rs`): blocklist tetap lebih dulu di bawah sandbox.
- Unit (`schema.rs`): parse `sandbox:`; absen secara default; `validate`.
- Unit (`subcommands.rs`): `info` baris sandbox tanpa nilai env.
- E2E core (`tests/sandbox_e2e.rs`, 5): lewat `chat_agentic` nyata — secret
  parent tak terlihat, `HERMES_SANDBOX=1`, cwd jail, baris `tool_calls`
  tersimpan; inherit == Spec 002; cap + rlimit di kedua tool; gerbang
  sebelum spawn (deny tidak membuat file); perintah hostile tidak lolos
  wrapper.
- E2E CLI (`subcommands_e2e.rs`, 2): `info` + `/sandbox` melaporkan
  policy; `network: dney` ditolak saat load (exit 1), `version` tetap OK.
