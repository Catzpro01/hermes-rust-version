# Spec 007b — Sandbox default `strict` + `--no-sandbox` (per /ask-matt)

Status: IN REVIEW (CI-verified)

## Keputusan
Matt (via "jalankan semuanya", 2026-09-13): sandbox proses (Spec 007)
menjadi **default on**. Alasan: sandbox yang tidak dinyalakan siapa pun
tidak melindungi siapa pun; makin lama default `off`, makin mengejutkan
saat diubah.

## Perubahan
- `SandboxConfig.enabled: Option<bool>` — absen = on; hanya `false`
  eksplisit yang mematikan. Section `sandbox:` tanpa `enabled` = on.
- `SandboxPolicy::from_config(None, root)` → `strict(root)`.
- Helper tunggal `tools::sandbox::resolve(cfg, root, no_sandbox)` dipakai
  REPL, TUI worker, dan `hermes info` — tidak mungkin berbeda.
- Flag global baru `--no-sandbox` (menang atas config) → `inherit`.
- Konstruktor library (`ShellTool::new` dll.) tetap `inherit` — default
  CLI yang berubah, bukan tipe library (embedder tidak terdampak).
- Docs: SECURITY.md, ADR 0006 amendment, ROADMAP baris 007b.

## Tes
- Unit: `from_config_default_is_strict_and_explicit_false_is_inherit`,
  `resolve_flag_overrides_config`; `render_info` default-on + flag.
- E2E `no_sandbox_flag_and_enabled_false_yield_inherit`: flag sebelum/
  sesudah subcommand, `/sandbox` REPL, `enabled: false`, section tanpa
  `enabled` (+ `cpu=2s` terbawa).
- `hermes info` tanpa config kini melaporkan `sandbox: on | cwd=…`.
