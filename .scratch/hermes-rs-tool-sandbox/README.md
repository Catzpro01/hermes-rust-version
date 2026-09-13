# Spec 007 — Tool execution sandbox

Slice vertikal: batas eksekusi **process-level, opt-in** untuk tool shell
(`shell`, `shell_readonly`) tanpa crate baru dan tanpa Docker. Desain: ADR
0006; model ancaman: `docs/SECURITY.md` §"Tool execution sandbox".

## Prinsip
- **Zero regression:** tanpa `sandbox:` di config → `SandboxPolicy::inherit`
  = `sh -c <cmd>` persis Spec 002; konstruktor lama tidak berubah.
- **Satu titik spawn:** `tools::sandbox::run_shell`.
- **Urutan gerbang tetap:** blocklist → konfirmasi → sandbox.
- **Fail closed:** `network: deny` tanpa `unshare`/userns → error, bukan
  diam-diam jalan.
- **Tanpa bocor:** `/sandbox` & `hermes info` hanya nama variabel + angka.

## Tiket
| # | Tiket | Status |
|---|---|---|
| 01 | `SandboxPolicy` + `run_shell` + output cap | DONE (review Matt pending) |
| 02 | Env allowlist + cwd jail + `HERMES_SANDBOX` | DONE |
| 03 | `ulimit` wrapper posisional + `unshare --net` fail-closed | DONE |
| 04 | Config `sandbox:` + validasi saat load | DONE |
| 05 | Wiring REPL/TUI, `/sandbox`, `hermes info`, docs, E2E closure | DONE |

Rincian & bukti: [issues/01-05-sandbox.md](issues/01-05-sandbox.md).

## Catatan verifikasi
Ditulis di sandbox offline tanpa toolchain Rust. Sebelum merge:
`cargo fmt --all && cargo test --workspace && cargo clippy --workspace
--all-targets -- -D warnings`. Test yang bergantung host: `run_shell_file_size_limit_is_enforced`
dan `spec007_output_cap_and_rlimits_apply_to_both_shell_tools` memakai
`ulimit -f` (POSIX sh; dash/bash oke).
