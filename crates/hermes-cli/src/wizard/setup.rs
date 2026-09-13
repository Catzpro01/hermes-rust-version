//! Spec 017 T05 — `hermes setup` multi-step wizard (v0.21.0 parity).
//!
//! Port of `hermes_cli/setup.py` `_run_setup_wizard_impl` (spec §C.2–C.7):
//! mode question → sections `Model & Provider` → `Terminal Backend` →
//! `Messaging Platforms` → `Tools`, or a single section via
//! `hermes setup model|terminal|gateway|tools`. Every prompt/notice string
//! with a Python original is verbatim (provenance in the doc comments).
//!
//! Design (per /ask-matt, T01 decisions carried forward):
//! * **collect first, write last** — ESC at any prompt → `Setup cancelled.`
//!   and *nothing* is written (rollback invariant); Ctrl+C → exit 130.
//! * **atomic + backup** — `config.yaml` is edited as a YAML mapping (unknown
//!   user keys survive), written to a temp file and renamed over the target;
//!   a previous file is backed up first (`Previous config backed up to:`).
//!   Secrets (platform tokens) go to `.env` (0600), never to `config.yaml`.
//! * **non-TTY** → clear error, exit 1, no writes (invariant 8).
//! * Terminal backend: only `Local` + `Docker` are wired upstream; the
//!   verbatim "not wired yet" line is printed.
//! * Nous Portal OAuth is not available in the Rust port: `Quick Setup`
//!   prints the verbatim portal notice and then runs the provider picker
//!   (same as `Full setup`, minus the messaging section — Python's quick
//!   path offers messaging as a yes/no, which is kept).

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_yaml::{Mapping, Value};

use super::catalog::{PlatformEntry, ProviderEntry, DEFAULT_OFF_TOOLSETS, PLATFORMS, PROVIDERS, TOOLSETS};
use super::{
    multiselect_with_defaults, password, require_tty, select, text_input, WizardError,
    CANCELED_MESSAGE, MODE_BLANK, MODE_FULL, MODE_QUESTION, MODE_QUICK, SECTIONS,
};

// ---------------------------------------------------------------------------
// Verbatim strings (hermes_cli/setup.py v0.21.0 unless noted).
// ---------------------------------------------------------------------------

/// setup.py L3294.
pub const FIRST_TIME: &str = "No existing configuration found — running first-time setup.";
/// setup.py L3282-3283.
pub const TIP_SECTIONS: &str = "Tip: jump straight to a section with 'hermes setup model|terminal|gateway|tools|agent', or fill only missing items with --quick.";
/// setup.py L3280.
pub const KEEP_HINT: &str = "Press Enter to keep it, or type a new value to change it.";
/// setup.py L3338.
pub const LOCATION_HEADER: &str = "Configuration Location";
/// setup.py L3344.
pub const LOCATION_FOOTER: &str = "You can edit these files directly or use 'hermes config edit'";
/// setup.py L3405-3407.
pub const BACKUP_NOTICE: &str = "Previous config backed up to: ";
pub const BACKUP_RESTORE: &str = "If setup changed a value you customized, restore it with:";
/// setup.py L965-966 (§C.3).
pub const MODEL_INTRO: &str = "Choose how to connect to your main chat model.";
pub const MODEL_GUIDE: &str =
    "   Guide: https://hermes-agent.nousresearch.com/docs/integrations/providers";
pub const PROVIDER_QUESTION: &str = "Select provider:";
/// setup.py L1405 / L1458 (§C.4).
pub const TERMINAL_INTRO: &str = "Choose where Hermes runs shell commands and code.";
pub const TERMINAL_QUESTION: &str = "Select terminal backend:";
pub const TERMINAL_LOCAL: &str = "Terminal backend: Local";
pub const TERMINAL_LOCAL_DESC: &str = "Commands run directly on this machine.";
pub const TERMINAL_DOCKER: &str = "Terminal backend: Docker";
pub const DOCKER_MISSING: &str = "Docker not found in PATH!";
pub const DOCKER_INSTALL: &str = "Install Docker: https://docs.docker.com/get-docker/";
pub const DOCKER_IMAGE_DEFAULT: &str = "nikolaik/python-nodejs:python3.11-nodejs20";
/// setup.py L1498 verbatim (§C.4 [KOREKSI]).
pub const TERMINAL_NOT_WIRED: &str =
    "   Docker only for now; Modal, SSH, Daytona, and Singularity are not wired yet.";
/// setup.py L2270 / L2284 (§C.5).
pub const PLATFORMS_HINT: &str = "Toggle with Space, confirm with Enter.";
pub const PLATFORMS_QUESTION: &str = "Select platforms to configure:";
pub const PLATFORMS_NONE: &str =
    "No platforms selected. Run 'hermes setup gateway' later to configure.";
pub const PLATFORMS_DONE: &str = "Messaging platforms configured!";
/// setup.py L3389 (§C.6) — step header; the checklist is `hermes tools`.
pub const TOOLS_QUESTION: &str = "Select toolsets to enable:";
/// §C.7 Quick Setup (Nous Portal).
pub const NOUS_HEADER: &str = "Nous Portal";
pub const NOUS_LINE1: &str = "One subscription, 300+ models, plus the Tool Gateway:";
pub const NOUS_LINE2: &str = "  web search, image generation, TTS, browser automation.";
pub const NOUS_SIGNUP: &str = "Sign up: https://portal.nousresearch.com/manage-subscription";
pub const MESSAGING_QUESTION: &str = "Connect a messaging platform? (Telegram, Discord, etc.)";
pub const MESSAGING_NOW: &str = "Set up messaging now (recommended)";
pub const MESSAGING_SKIP: &str = "Skip — set up later with 'hermes setup gateway'";
pub const SETUP_COMPLETE: &str = "Setup complete! You're ready to go.";

/// Rust-only prompts (no Python original: upstream resolves URL/key_env per
/// provider module; here they are asked once, generic).
const API_URL_PROMPT: &str = "API base URL";
const KEY_ENV_PROMPT: &str = "Environment variable holding the API key";
const MODEL_PROMPT: &str = "Model name";
const DOCKER_IMAGE_PROMPT: &str = "Docker image";

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

/// `hermes setup <section>` targets (setup.py L3395-3398 labels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Model,
    Terminal,
    Gateway,
    Tools,
}

impl Section {
    /// CLI token → section (`agent` is accepted upstream but has no wired
    /// step in this port; the caller reports it).
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "model" => Some(Self::Model),
            "terminal" => Some(Self::Terminal),
            "gateway" => Some(Self::Gateway),
            "tools" => Some(Self::Tools),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Model => SECTIONS[0],
            Self::Terminal => SECTIONS[1],
            Self::Gateway => SECTIONS[2],
            Self::Tools => SECTIONS[3],
        }
    }

    const ALL: [Section; 4] = [Self::Model, Self::Terminal, Self::Gateway, Self::Tools];
}

/// Setup mode (setup.py L3305-3307).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Quick,
    Full,
    Blank,
}

impl Mode {
    /// Sections each mode runs (Python: quick = model + tools + optional
    /// messaging; full = all; blank = model only, every toolset off).
    fn sections(self) -> Vec<Section> {
        match self {
            Mode::Quick => vec![Section::Model, Section::Tools],
            Mode::Full => Section::ALL.to_vec(),
            Mode::Blank => vec![Section::Model],
        }
    }
}

// ---------------------------------------------------------------------------
// Collected answers (nothing is written until `apply`).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelAnswer {
    pub provider_id: String,
    pub api_url: String,
    pub key_env: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalAnswer {
    Local,
    Docker { image: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Answers {
    pub model: Option<ModelAnswer>,
    pub terminal: Option<TerminalAnswer>,
    pub platforms: PlatformAnswers,
    /// Enabled toolset keys (catalog order).
    pub toolsets: Option<Vec<String>>,
}

impl Answers {
    pub fn is_empty(&self) -> bool {
        self.model.is_none()
            && self.terminal.is_none()
            && self.platforms.is_empty()
            && self.toolsets.is_none()
    }
}

/// Platform key → `(env var, value)` pairs (secrets included; `.env` only).
pub type PlatformAnswers = Vec<(String, Vec<(String, String)>)>;

/// Outcome of one wizard run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // `backup` is informational for callers/tests (already printed).
pub enum Outcome {
    /// Answers applied; `backup` is the pre-existing config's backup path.
    Applied { backup: Option<PathBuf> },
    /// ESC somewhere → nothing written.
    Cancelled,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Runs the wizard against `home`. `section == None` → full flow with the
/// mode question; `Some(s)` → that section only (`hermes setup <s>`).
pub fn run_setup(home: &Path, section: Option<Section>) -> Result<Outcome, WizardError> {
    require_tty()?;
    let config_path = home.join("config.yaml");
    let env_path = home.join(".env");
    let existing = read_yaml_mapping(&config_path);
    let existing_env = read_env(&env_path);

    if existing.is_none() {
        println!("{FIRST_TIME}");
    }
    print_location(home, &config_path, &env_path);
    println!("{TIP_SECTIONS}");
    println!("{KEEP_HINT}");
    println!();

    let (mode, sections) = match section {
        Some(s) => (None, vec![s]),
        None => {
            let choice = match select(MODE_QUESTION, vec![MODE_QUICK, MODE_FULL, MODE_BLANK]) {
                Ok(c) => c,
                Err(WizardError::Canceled) => return cancelled(),
                Err(e) => return Err(e),
            };
            let mode = if choice == MODE_QUICK {
                Mode::Quick
            } else if choice == MODE_FULL {
                Mode::Full
            } else {
                Mode::Blank
            };
            (Some(mode), mode.sections())
        }
    };

    if mode == Some(Mode::Quick) {
        println!();
        println!("{NOUS_HEADER}");
        println!("{NOUS_LINE1}");
        println!("{NOUS_LINE2}");
        println!("{NOUS_SIGNUP}");
        println!();
    }

    let current = existing.clone().unwrap_or_default();
    let mut answers = Answers::default();
    for s in &sections {
        println!();
        println!("{}", s.label());
        let step = match s {
            Section::Model => step_model(&current).map(|a| answers.model = Some(a)),
            Section::Terminal => step_terminal(&current).map(|a| answers.terminal = Some(a)),
            Section::Gateway => {
                step_platforms(&existing_env).map(|p| answers.platforms = p)
            }
            Section::Tools => {
                let blank = mode == Some(Mode::Blank);
                step_tools(&current, blank).map(|t| answers.toolsets = Some(t))
            }
        };
        match step {
            Ok(()) => {}
            Err(WizardError::Canceled) => return cancelled(),
            Err(e) => return Err(e),
        }
    }
    if mode == Some(Mode::Blank) {
        // Blank Slate: everything off except the bare minimum.
        answers.toolsets = Some(Vec::new());
    }
    if mode == Some(Mode::Quick) {
        println!();
        match select(MESSAGING_QUESTION, vec![MESSAGING_NOW, MESSAGING_SKIP]) {
            Ok(c) if c == MESSAGING_NOW => match step_platforms(&existing_env) {
                Ok(p) => answers.platforms = p,
                Err(WizardError::Canceled) => return cancelled(),
                Err(e) => return Err(e),
            },
            Ok(_) => {}
            Err(WizardError::Canceled) => return cancelled(),
            Err(e) => return Err(e),
        }
    }

    let backup = apply(home, &config_path, &env_path, existing, &answers)
        .map_err(|_| WizardError::Other)?;
    if let Some(b) = &backup {
        println!("{BACKUP_NOTICE}{}", b.display());
        println!("{BACKUP_RESTORE}");
        println!("  cp {} {}", b.display(), config_path.display());
    }
    println!("{SETUP_COMPLETE}");
    Ok(Outcome::Applied { backup })
}

fn cancelled() -> Result<Outcome, WizardError> {
    println!("{CANCELED_MESSAGE}");
    Ok(Outcome::Cancelled)
}

fn print_location(home: &Path, config_path: &Path, env_path: &Path) {
    println!("{LOCATION_HEADER}");
    println!("Config file:  {}", config_path.display());
    println!("Secrets file: {}", env_path.display());
    println!("Data folder:  {}", home.display());
    println!("Install dir:  {}", env!("CARGO_MANIFEST_DIR"));
    println!("{LOCATION_FOOTER}");
    println!();
}

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

fn provider_choice(p: &ProviderEntry) -> String {
    p.description.to_owned()
}

fn step_model(current: &Mapping) -> Result<ModelAnswer, WizardError> {
    println!("{MODEL_INTRO}");
    println!("{MODEL_GUIDE}");
    let current_provider = yaml_str(current, &["model", "provider"]);
    let start = current_provider
        .as_deref()
        .and_then(|id| PROVIDERS.iter().position(|p| p.id == id))
        .unwrap_or(0);
    let choices: Vec<String> = PROVIDERS.iter().map(provider_choice).collect();
    let picked = super::select_at(PROVIDER_QUESTION, choices.clone(), start)?;
    let idx = choices.iter().position(|c| *c == picked).unwrap_or(0);
    let entry = PROVIDERS[idx];

    let prov_cfg = current
        .get(Value::from("providers"))
        .and_then(Value::as_mapping)
        .and_then(|m| m.get(Value::from(entry.id)))
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();
    let api_url = text_input(
        API_URL_PROMPT,
        &yaml_str(&prov_cfg, &["api"]).unwrap_or_default(),
    )?;
    let key_env = text_input(
        KEY_ENV_PROMPT,
        &yaml_str(&prov_cfg, &["key_env"]).unwrap_or_else(|| default_key_env(entry.id)),
    )?;
    let current_model = yaml_str(current, &["model", "name"])
        .or_else(|| yaml_str(current, &["model", "default"]))
        .unwrap_or_default();
    let model = text_input(MODEL_PROMPT, &current_model)?;
    Ok(ModelAnswer {
        provider_id: entry.id.to_owned(),
        api_url: api_url.trim().to_owned(),
        key_env: key_env.trim().to_owned(),
        model: model.trim().to_owned(),
    })
}

/// `{ID}_API_KEY` with `-` → `_` (e.g. `openai-api` → `OPENAI_API_API_KEY`
/// is wrong, so the two OpenAI ids are special-cased like upstream).
pub fn default_key_env(provider_id: &str) -> String {
    match provider_id {
        "openai-api" | "openai-codex" => "OPENAI_API_KEY".to_owned(),
        "xai-oauth" | "xai" => "XAI_API_KEY".to_owned(),
        other => format!("{}_API_KEY", other.to_ascii_uppercase().replace('-', "_")),
    }
}

fn step_terminal(current: &Mapping) -> Result<TerminalAnswer, WizardError> {
    println!("{TERMINAL_INTRO}");
    println!("{TERMINAL_NOT_WIRED}");
    let current_backend = yaml_str(current, &["terminal", "backend"]);
    let start = usize::from(current_backend.as_deref() == Some("docker"));
    let choice = super::select_at(
        TERMINAL_QUESTION,
        vec![
            format!("{TERMINAL_LOCAL} — {TERMINAL_LOCAL_DESC}"),
            TERMINAL_DOCKER.to_owned(),
        ],
        start,
    )?;
    if choice.starts_with(TERMINAL_LOCAL) {
        if current_backend.as_deref() == Some("local") {
            println!("Keeping current backend: local");
        }
        return Ok(TerminalAnswer::Local);
    }
    match find_in_path("docker") {
        Some(bin) => println!("Docker found: {}", bin.display()),
        None => {
            println!("{DOCKER_MISSING}");
            println!("{DOCKER_INSTALL}");
        }
    }
    let image = text_input(
        DOCKER_IMAGE_PROMPT,
        &yaml_str(current, &["terminal", "docker_image"])
            .unwrap_or_else(|| DOCKER_IMAGE_DEFAULT.to_owned()),
    )?;
    Ok(TerminalAnswer::Docker {
        image: image.trim().to_owned(),
    })
}

/// `{emoji} {label}  ({status})` — setup.py L2280.
pub fn platform_row(p: &PlatformEntry, env: &BTreeMap<String, String>) -> String {
    let status = platform_status(p, env);
    format!("{} {}  ({status})", p.emoji, p.label)
}

/// `configured` | `not configured` | `partially` (setup.py L2280 statuses).
pub fn platform_status(p: &PlatformEntry, env: &BTreeMap<String, String>) -> &'static str {
    let has = |k: &str| env.get(k).map(|v| !v.is_empty()).unwrap_or(false);
    if p.vars.is_empty() {
        return if has(p.token_var) { "configured" } else { "not configured" };
    }
    let n = p.vars.iter().filter(|v| has(v.name)).count();
    if n == 0 {
        "not configured"
    } else if n == p.vars.len() {
        "configured"
    } else {
        "partially"
    }
}

fn step_platforms(env: &BTreeMap<String, String>) -> Result<PlatformAnswers, WizardError> {
    println!("{PLATFORMS_HINT}");
    let rows: Vec<String> = PLATFORMS.iter().map(|p| platform_row(p, env)).collect();
    let picked = multiselect_with_defaults(PLATFORMS_QUESTION, rows.clone(), &[])?;
    if picked.is_empty() {
        println!("{PLATFORMS_NONE}");
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for row in picked {
        let idx = rows.iter().position(|r| *r == row).unwrap_or(0);
        let p = &PLATFORMS[idx];
        println!();
        for line in p.setup_instructions {
            println!("{line}");
        }
        let mut vars = Vec::new();
        if p.vars.is_empty() {
            let v = text_input(p.token_var, env.get(p.token_var).map(String::as_str).unwrap_or(""))?;
            vars.push((p.token_var.to_owned(), v.trim().to_owned()));
        } else {
            for var in p.vars {
                println!("  {}", var.help);
                let existing = env.get(var.name).map(String::as_str).unwrap_or("");
                let value = if var.password {
                    let v = password(var.prompt)?;
                    if v.is_empty() {
                        existing.to_owned()
                    } else {
                        v
                    }
                } else {
                    text_input(var.prompt, existing)?
                };
                vars.push((var.name.to_owned(), value.trim().to_owned()));
            }
        }
        out.push((p.key.to_owned(), vars));
    }
    println!("{PLATFORMS_DONE}");
    Ok(out)
}

/// Pre-selected toolset indices: the config's `tools.enabled_toolsets` when
/// present, else the catalog minus `_DEFAULT_OFF_TOOLSETS`.
pub fn toolset_defaults(current: &Mapping) -> Vec<usize> {
    let configured: Option<Vec<String>> = current
        .get(Value::from("tools"))
        .and_then(Value::as_mapping)
        .and_then(|m| m.get(Value::from("enabled_toolsets")))
        .and_then(Value::as_sequence)
        .map(|s| {
            s.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        });
    TOOLSETS
        .iter()
        .enumerate()
        .filter(|(_, t)| match &configured {
            Some(on) => on.iter().any(|k| k == t.key),
            None => !DEFAULT_OFF_TOOLSETS.contains(&t.key),
        })
        .map(|(i, _)| i)
        .collect()
}

fn step_tools(current: &Mapping, blank: bool) -> Result<Vec<String>, WizardError> {
    let rows: Vec<String> = TOOLSETS
        .iter()
        .map(|t| format!("{}  ({})", t.label, t.tools))
        .collect();
    let defaults = if blank { Vec::new() } else { toolset_defaults(current) };
    let picked = multiselect_with_defaults(TOOLS_QUESTION, rows.clone(), &defaults)?;
    Ok(TOOLSETS
        .iter()
        .zip(rows.iter())
        .filter(|(_, row)| picked.contains(*row))
        .map(|(t, _)| t.key.to_owned())
        .collect())
}

// ---------------------------------------------------------------------------
// Apply: merge answers into the YAML mapping + .env, write atomically.
// ---------------------------------------------------------------------------

/// Pure merge (unit-tested): returns the new mapping. Unknown keys in
/// `existing` are preserved verbatim.
pub fn merge_answers(existing: Option<Mapping>, answers: &Answers) -> Mapping {
    let mut root = existing.unwrap_or_default();
    if let Some(m) = &answers.model {
        let model = mapping_at(&mut root, "model");
        model.insert(Value::from("provider"), Value::from(m.provider_id.as_str()));
        if !m.model.is_empty() {
            model.insert(Value::from("name"), Value::from(m.model.as_str()));
            model.insert(Value::from("default"), Value::from(m.model.as_str()));
        }
        let providers = mapping_at(&mut root, "providers");
        let entry = mapping_at(providers, m.provider_id.as_str());
        if !m.api_url.is_empty() {
            entry.insert(Value::from("api"), Value::from(m.api_url.as_str()));
        }
        if !m.key_env.is_empty() {
            entry.insert(Value::from("key_env"), Value::from(m.key_env.as_str()));
        }
        if !m.model.is_empty() {
            let models = mapping_at(entry, "models");
            if !models.contains_key(Value::from(m.model.as_str())) {
                models.insert(Value::from(m.model.as_str()), Value::Mapping(Mapping::new()));
            }
        }
    }
    if let Some(t) = &answers.terminal {
        let term = mapping_at(&mut root, "terminal");
        match t {
            TerminalAnswer::Local => {
                term.insert(Value::from("backend"), Value::from("local"));
            }
            TerminalAnswer::Docker { image } => {
                term.insert(Value::from("backend"), Value::from("docker"));
                term.insert(Value::from("docker_image"), Value::from(image.as_str()));
            }
        }
    }
    if let Some(keys) = &answers.toolsets {
        let tools = mapping_at(&mut root, "tools");
        tools.insert(
            Value::from("enabled_toolsets"),
            Value::Sequence(keys.iter().map(|k| Value::from(k.as_str())).collect()),
        );
    }
    root
}

/// Pure `.env` merge (unit-tested): keeps unrelated keys and order, updates
/// or appends the platform variables. Empty values are skipped (never wipe
/// an existing secret with nothing).
pub fn merge_env(existing: &str, answers: &Answers) -> String {
    let mut lines: Vec<String> = existing.lines().map(str::to_owned).collect();
    for (_, vars) in &answers.platforms {
        for (name, value) in vars {
            if value.is_empty() {
                continue;
            }
            let entry = format!("{name}={value}");
            let prefix = format!("{name}=");
            match lines.iter_mut().find(|l| l.starts_with(&prefix)) {
                Some(l) => *l = entry,
                None => lines.push(entry),
            }
        }
    }
    let mut out = lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

fn apply(
    home: &Path,
    config_path: &Path,
    env_path: &Path,
    existing: Option<Mapping>,
    answers: &Answers,
) -> std::io::Result<Option<PathBuf>> {
    if answers.is_empty() {
        return Ok(None);
    }
    fs::create_dir_all(home)?;
    let mut backup = None;
    let needs_config = answers.model.is_some() || answers.terminal.is_some() || answers.toolsets.is_some();
    if needs_config {
        if config_path.is_file() {
            let b = backup_path(config_path);
            fs::copy(config_path, &b)?;
            backup = Some(b);
        }
        let merged = merge_answers(existing, answers);
        let text = serde_yaml::to_string(&Value::Mapping(merged))
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        write_atomic(config_path, text.as_bytes(), 0o644)?;
    }
    if !answers.platforms.is_empty() {
        let current = fs::read_to_string(env_path).unwrap_or_default();
        let merged = merge_env(&current, answers);
        write_atomic(env_path, merged.as_bytes(), 0o600)?;
    }
    Ok(backup)
}

/// `config.yaml.bak.<unix-seconds>` next to the config (never collides with
/// the `.bak` the T01 skeleton era left behind).
pub fn backup_path(config_path: &Path) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = format!(
        "{}.bak.{ts}",
        config_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "config.yaml".to_owned())
    );
    config_path.with_file_name(name)
}

/// temp file in the same directory → fsync → rename over `path`.
pub fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(format!(
        ".{}.tmp-{}",
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_owned()),
        std::process::id()
    ));
    {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            f.set_permissions(fs::Permissions::from_mode(mode))?;
        }
        #[cfg(not(unix))]
        let _ = mode;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn read_yaml_mapping(path: &Path) -> Option<Mapping> {
    let text = fs::read_to_string(path).ok()?;
    match serde_yaml::from_str::<Value>(&text) {
        Ok(Value::Mapping(m)) => Some(m),
        Ok(Value::Null) => Some(Mapping::new()),
        _ => Some(Mapping::new()),
    }
}

/// Minimal `.env` reader (`KEY=value`, `#` comments, optional `export `).
pub fn parse_env(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').trim_matches('\'');
            out.insert(k.trim().to_owned(), v.to_owned());
        }
    }
    out
}

fn read_env(path: &Path) -> BTreeMap<String, String> {
    fs::read_to_string(path)
        .map(|t| parse_env(&t))
        .unwrap_or_default()
}

fn yaml_str(m: &Mapping, path: &[&str]) -> Option<String> {
    let mut cur = Value::Mapping(m.clone());
    for key in path {
        cur = cur.as_mapping()?.get(Value::from(*key))?.clone();
    }
    cur.as_str().map(str::to_owned)
}

fn mapping_at<'a>(root: &'a mut Mapping, key: &str) -> &'a mut Mapping {
    let k = Value::from(key);
    if !matches!(root.get(&k), Some(Value::Mapping(_))) {
        root.insert(k.clone(), Value::Mapping(Mapping::new()));
    }
    root.get_mut(&k)
        .and_then(Value::as_mapping_mut)
        .expect("ensured mapping")
}

fn find_in_path(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(bin))
        .find(|p| p.is_file())
}

/// `.env`/`config.yaml` are the only files the wizard touches; exported for
/// the non-TTY guard test.
#[allow(dead_code)]
pub fn touched_files(home: &Path) -> [PathBuf; 2] {
    [home.join("config.yaml"), home.join(".env")]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml(s: &str) -> Mapping {
        match serde_yaml::from_str::<Value>(s).unwrap() {
            Value::Mapping(m) => m,
            _ => panic!("not a mapping"),
        }
    }

    #[test]
    fn verbatim_setup_strings_are_pinned() {
        assert_eq!(FIRST_TIME, "No existing configuration found — running first-time setup.");
        assert_eq!(KEEP_HINT, "Press Enter to keep it, or type a new value to change it.");
        assert_eq!(TERMINAL_QUESTION, "Select terminal backend:");
        assert_eq!(
            TERMINAL_NOT_WIRED,
            "   Docker only for now; Modal, SSH, Daytona, and Singularity are not wired yet."
        );
        assert_eq!(PLATFORMS_QUESTION, "Select platforms to configure:");
        assert_eq!(PLATFORMS_HINT, "Toggle with Space, confirm with Enter.");
        assert_eq!(
            PLATFORMS_NONE,
            "No platforms selected. Run 'hermes setup gateway' later to configure."
        );
        assert_eq!(PLATFORMS_DONE, "Messaging platforms configured!");
        assert_eq!(SETUP_COMPLETE, "Setup complete! You're ready to go.");
        assert_eq!(MODEL_INTRO, "Choose how to connect to your main chat model.");
        assert_eq!(DOCKER_MISSING, "Docker not found in PATH!");
    }

    #[test]
    fn section_parse_and_labels() {
        assert_eq!(Section::parse("model"), Some(Section::Model));
        assert_eq!(Section::parse("gateway"), Some(Section::Gateway));
        assert_eq!(Section::parse("agent"), None);
        assert_eq!(Section::Gateway.label(), "Messaging Platforms");
        assert_eq!(Mode::Full.sections(), Section::ALL.to_vec());
        assert_eq!(Mode::Blank.sections(), vec![Section::Model]);
    }

    #[test]
    fn merge_preserves_unknown_keys_and_sets_model() {
        let existing = yaml("custom_key: keep-me\nmodel:\n  provider: auto\n  fallback_chain: [fake]\nsandbox:\n  enabled: true\n");
        let answers = Answers {
            model: Some(ModelAnswer {
                provider_id: "lmstudio".into(),
                api_url: "http://localhost:1234/v1".into(),
                key_env: "LMSTUDIO_API_KEY".into(),
                model: "qwen3".into(),
            }),
            terminal: Some(TerminalAnswer::Docker {
                image: DOCKER_IMAGE_DEFAULT.into(),
            }),
            platforms: vec![],
            toolsets: Some(vec!["web".into(), "file".into()]),
        };
        let merged = merge_answers(Some(existing), &answers);
        let text = serde_yaml::to_string(&Value::Mapping(merged.clone())).unwrap();
        assert!(text.contains("custom_key: keep-me"), "{text}");
        assert!(text.contains("enabled: true"), "{text}");
        assert_eq!(yaml_str(&merged, &["model", "provider"]).as_deref(), Some("lmstudio"));
        assert_eq!(yaml_str(&merged, &["model", "name"]).as_deref(), Some("qwen3"));
        assert_eq!(
            yaml_str(&merged, &["providers", "lmstudio", "api"]).as_deref(),
            Some("http://localhost:1234/v1")
        );
        assert_eq!(yaml_str(&merged, &["terminal", "backend"]).as_deref(), Some("docker"));
        // fallback_chain preserved inside `model`.
        assert!(text.contains("fallback_chain"), "{text}");
        // The result must still load through the strict HermesConfig schema.
        let cfg: hermes_core::config::HermesConfig = serde_yaml::from_str(&text).unwrap();
        assert_eq!(cfg.model.provider.as_deref(), Some("lmstudio"));
        assert!(cfg.providers["lmstudio"].models.contains_key("qwen3"));
    }

    #[test]
    fn env_merge_updates_in_place_and_appends() {
        let existing = "# secrets\nOTHER=1\nMATTERMOST_TOKEN=old\n";
        let answers = Answers {
            platforms: vec![(
                "mattermost".into(),
                vec![
                    ("MATTERMOST_URL".into(), "https://mm".into()),
                    ("MATTERMOST_TOKEN".into(), "new".into()),
                    ("MATTERMOST_HOME_CHANNEL".into(), String::new()),
                ],
            )],
            ..Default::default()
        };
        let out = merge_env(existing, &answers);
        assert_eq!(
            out,
            "# secrets\nOTHER=1\nMATTERMOST_TOKEN=new\nMATTERMOST_URL=https://mm\n"
        );
        let parsed = parse_env(&out);
        assert_eq!(parsed["MATTERMOST_TOKEN"], "new");
        assert!(!parsed.contains_key("MATTERMOST_HOME_CHANNEL"));
    }

    #[test]
    fn platform_status_follows_env() {
        let mm = PLATFORMS.iter().find(|p| p.key == "mattermost").unwrap();
        let mut env = BTreeMap::new();
        assert_eq!(platform_status(mm, &env), "not configured");
        env.insert("MATTERMOST_TOKEN".to_owned(), "x".to_owned());
        assert_eq!(platform_status(mm, &env), "partially");
        for v in mm.vars {
            env.insert(v.name.to_owned(), "x".to_owned());
        }
        assert_eq!(platform_status(mm, &env), "configured");
        assert_eq!(platform_row(mm, &env), "💬 Mattermost  (configured)");
        let signal = PLATFORMS.iter().find(|p| p.key == "signal").unwrap();
        assert_eq!(platform_status(signal, &env), "not configured");
    }

    #[test]
    fn toolset_defaults_follow_catalog_then_config() {
        let d = toolset_defaults(&Mapping::new());
        let keys: Vec<&str> = d.iter().map(|&i| TOOLSETS[i].key).collect();
        assert!(keys.contains(&"web") && !keys.contains(&"spotify"));
        let cfg = yaml("tools:\n  enabled_toolsets: [spotify]\n");
        let d = toolset_defaults(&cfg);
        assert_eq!(d.iter().map(|&i| TOOLSETS[i].key).collect::<Vec<_>>(), vec!["spotify"]);
    }

    #[test]
    fn default_key_env_names() {
        assert_eq!(default_key_env("openai-api"), "OPENAI_API_KEY");
        assert_eq!(default_key_env("deepseek"), "DEEPSEEK_API_KEY");
        assert_eq!(default_key_env("openai-codex"), "OPENAI_API_KEY");
        assert_eq!(default_key_env("tencent-tokenhub"), "TENCENT_TOKENHUB_API_KEY");
    }

    #[test]
    fn write_atomic_and_backup_paths() {
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join("config.yaml");
        write_atomic(&p, b"a: 1\n", 0o644).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "a: 1\n");
        // no temp leftovers
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty());
        let b = backup_path(&p);
        assert!(b.file_name().unwrap().to_string_lossy().starts_with("config.yaml.bak."));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let e = dir.path().join(".env");
            write_atomic(&e, b"K=v\n", 0o600).unwrap();
            assert_eq!(fs::metadata(&e).unwrap().permissions().mode() & 0o777, 0o600);
        }
    }

    #[test]
    fn apply_writes_backup_config_and_env() {
        let dir = tempfile::TempDir::new().unwrap();
        let home = dir.path();
        let cfg = home.join("config.yaml");
        fs::write(&cfg, "model:\n  provider: auto\nkeep: yes\n").unwrap();
        let answers = Answers {
            model: Some(ModelAnswer {
                provider_id: "fireworks".into(),
                api_url: "https://api.fireworks.ai/inference/v1".into(),
                key_env: "FIREWORKS_API_KEY".into(),
                model: "llama".into(),
            }),
            platforms: vec![("signal".into(), vec![("SIGNAL_HTTP_URL".into(), "http://s".into())])],
            ..Default::default()
        };
        let backup = apply(home, &cfg, &home.join(".env"), read_yaml_mapping(&cfg), &answers)
            .unwrap()
            .expect("backup made");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "model:\n  provider: auto\nkeep: yes\n");
        let text = fs::read_to_string(&cfg).unwrap();
        assert!(text.contains("keep: yes") && text.contains("provider: fireworks"), "{text}");
        assert_eq!(fs::read_to_string(home.join(".env")).unwrap(), "SIGNAL_HTTP_URL=http://s\n");
        // Empty answers never write anything.
        let dir2 = tempfile::TempDir::new().unwrap();
        let c2 = dir2.path().join("config.yaml");
        assert_eq!(apply(dir2.path(), &c2, &dir2.path().join(".env"), None, &Answers::default()).unwrap(), None);
        assert!(!c2.exists());
    }

    #[test]
    fn non_tty_refuses_before_any_write() {
        let dir = tempfile::TempDir::new().unwrap();
        let err = run_setup(dir.path(), None).expect_err("piped stdin");
        assert_eq!(err, WizardError::NotTty);
        for f in touched_files(dir.path()) {
            assert!(!f.exists());
        }
    }
}
