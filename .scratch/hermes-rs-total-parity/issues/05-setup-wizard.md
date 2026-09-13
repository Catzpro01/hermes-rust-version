# T05 — Setup wizard multi-step `hermes setup` (Spec 017)

Status: IN REVIEW (CI-verified)

## Ruang lingkup

Menggantikan stub `hermes setup` (radiolist + "Setup complete." palsu) dan
flag tersembunyi `--setup-skeleton` (T01) dengan wizard nyata (spec §C):

- `hermes setup` → `How would you like to set up Hermes?` (Quick/Full/Blank
  verbatim) → section `Model & Provider` → `Terminal Backend` →
  `Messaging Platforms` → `Tools` (Quick: model+tools+tawaran messaging;
  Blank: model saja + semua toolset off).
- `hermes setup model|terminal|gateway|tools` → satu section (`agent`
  tidak di-wire → error jelas).
- Header verbatim: `No existing configuration found — running first-time
  setup.`, blok `Configuration Location`, `Tip: jump straight to a
  section…`, `Press Enter to keep it, or type a new value to change it.`
- Model: picker 39 provider verbatim (`catalog.rs` ← `providers_canonical.txt`),
  lalu URL / key_env / model (prefill dari config; key_env di-resolve dari
  config seperti Python, default `{ID}_API_KEY`).
- Terminal: **Local + Docker saja**, baris verbatim `   Docker only for now;
  Modal, SSH, Daytona, and Singularity are not wired yet.`; deteksi
  `Docker found: …` / `Docker not found in PATH!`; image default
  `nikolaik/python-nodejs:python3.11-nodejs20`.
- Gateway: `Toggle with Space, confirm with Enter.` / `Select platforms to
  configure:`; baris `{emoji} {label}  ({status})` dengan status
  configured/not configured/partially dari `.env`; 6 `_PLATFORMS` verbatim
  (instruksi + vars, password dimask). Token ke `.env` (0600), bukan
  config.yaml.
- Tools: checklist 26 toolset verbatim, pre-check = `tools.enabled_toolsets`
  dari config atau katalog − `_DEFAULT_OFF_TOOLSETS`.

## Invariants

- Collect-first-write-last: ESC di prompt mana pun → `Setup cancelled.`,
  exit 0, **tidak ada file tersentuh** (E2E membuktikan config byte-identik
  dan direktori tidak berubah). Ctrl+C → exit 130. Non-TTY → exit 1.
- Tulis atomik (temp + fsync + rename) dan backup
  `config.yaml.bak.<unix>` + notice verbatim `Previous config backed up
  to:` / `If setup changed a value you customized, restore it with:` /
  `  cp …`.
- Merge di level YAML mapping: key user yang tidak dikenal (mis. `sandbox`,
  `fallback_chain`, key custom) dipertahankan; hasil tetap lolos
  `HermesConfig` strict (unit test).
- `serde_yaml` naik dari dev-dep ke dep normal `hermes-rs`.

## Keputusan per /ask-matt (2026-09-13)

- Quick Setup mencetak blok Nous Portal verbatim **lalu** baris Rust-only
  `  Nous Portal OAuth is not available in Hermes-RS yet — pick a provider
  below.` (konstanta `NOUS_NOT_AVAILABLE`, ditandai bukan string Python),
  kemudian picker provider biasa. E2E
  `quick_setup_states_nous_oauth_is_unavailable`.
- Nous Portal OAuth → **tunda**, tiket terpisah (integrasi eksternal).
- OpenClaw import → **skip permanen** (`IMPORT_QUESTION` tetap ada, tidak
  ditanyakan).
- Egress firewall Docker → **tunda** sampai backend Docker dieksekusi.
- `hermes setup agent`, `--quick` → tiket kecil setelah T06/T07.

## Tes

- Unit `wizard::setup` (10): string verbatim, parse section, merge YAML
  (preservasi key + lolos schema), merge `.env`, status platform, default
  toolset, key_env, atomic write + mode 0600, apply + backup, non-TTY guard.
- Unit `wizard::catalog` (3): 39/26/8/6 dibandingkan ulang dengan file
  verbatim.
- E2E `wizard_e2e`: non-TTY exit 1 tanpa tulis; section tak dikenal;
  `--setup-skeleton` hilang; PTY `setup terminal` (first-time, not-wired,
  tulis atomik); PTY ESC (config byte-identik, tanpa backup); PTY
  `setup model` (backup + merge + key custom dipertahankan).
