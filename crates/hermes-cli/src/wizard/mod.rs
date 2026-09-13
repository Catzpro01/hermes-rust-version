//! Spec 017 (T01): setup-wizard skeleton + reusable interactive helpers.
//!
//! Wraps `inquire` (crossterm backend) with the Python wizard's key
//! semantics (Phase 0 findings, `docs/HERMES_UI_SPEC.md` §C.1):
//!
//! * ESC (`OperationCanceled`) cancels the current prompt; the wizard then
//!   rolls back to defaults — cancel is NOT an error (invariant 3);
//! * Ctrl+C (`OperationInterrupted`) interrupts the whole wizard; the error
//!   text contains "interrupted" so `main()` maps it to exit code 130
//!   (invariant 2);
//! * non-TTY stdin is rejected with a clear error before any raw-mode work
//!   (invariant 8) — `inquire` would return `NotTTY` anyway, but the
//!   pre-check keeps the message deterministic and unit-testable.
//!
//! T01 shipped the helpers + a hidden skeleton flag; T05 (`setup.rs`)
//! replaced it with the real `hermes setup` wizard (sections, atomic config
//! write, backup) and removed `--setup-skeleton`. Question strings that have
//! a Python original are verbatim from `hermes_cli/setup.py` v0.21.0
//! (provenance: `docs/HERMES_UI_SPEC.md` §C.2/§K). Verbatim catalogs live in
//! `catalog.rs`.

use std::io::{self, IsTerminal};

use inquire::InquireError;

pub mod catalog;
pub mod setup;

/// Errors a wizard step can surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardError {
    /// stdin is not a terminal (piped/redirected) → clear error, exit 1.
    NotTty,
    /// The user pressed ESC; the wizard rolls back to defaults.
    Canceled,
    /// The user pressed Ctrl+C during a prompt (exit 130 upstream).
    Interrupted,
    /// Anything else (rendering, configuration, terminal I/O).
    Other,
}

impl WizardError {
    /// Static message; `main()` prints it as `error: setup wizard {message}`.
    pub fn message(&self) -> &'static str {
        match self {
            WizardError::NotTty => "requires an interactive terminal (non-TTY stdin detected)",
            WizardError::Canceled => "cancelled",
            WizardError::Interrupted => "interrupted (Ctrl-C)",
            WizardError::Other => "failed to render or read the terminal",
        }
    }
}

impl std::fmt::Display for WizardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "setup wizard {}", self.message())
    }
}

impl std::error::Error for WizardError {}

fn map_inquire(err: InquireError) -> WizardError {
    match err {
        InquireError::OperationCanceled => WizardError::Canceled,
        InquireError::OperationInterrupted => WizardError::Interrupted,
        InquireError::NotTTY => WizardError::NotTty,
        _ => WizardError::Other,
    }
}

/// True when stdin is an interactive terminal (the wizard's only input).
pub fn is_interactive() -> bool {
    io::stdin().is_terminal()
}

pub(crate) fn require_tty() -> Result<(), WizardError> {
    if is_interactive() {
        Ok(())
    } else {
        Err(WizardError::NotTty)
    }
}

/// Yes/no prompt (ENTER confirms the highlighted value, ESC cancels).
#[allow(dead_code)] // OpenClaw import offer (§C.2) is not wired in this port yet.
pub fn confirm(message: &str, default: bool) -> Result<bool, WizardError> {
    require_tty()?;
    inquire::Confirm::new(message)
        .with_default(default)
        .prompt()
        .map_err(map_inquire)
}

/// Single-select list. The first option is highlighted by default (the
/// Python radio menus also start on the first item).
pub fn select<T: std::fmt::Display>(message: &str, options: Vec<T>) -> Result<T, WizardError> {
    require_tty()?;
    inquire::Select::new(message, options)
        .prompt()
        .map_err(map_inquire)
}

/// Single-select list with the cursor on `start` (Python radio menus start
/// on the current value when one is configured).
pub fn select_at<T: std::fmt::Display>(
    message: &str,
    options: Vec<T>,
    start: usize,
) -> Result<T, WizardError> {
    require_tty()?;
    let start = start.min(options.len().saturating_sub(1));
    inquire::Select::new(message, options)
        .with_starting_cursor(start)
        .prompt()
        .map_err(map_inquire)
}

/// Multi-select with pre-checked rows (`[✓]` in Python's checklist).
pub fn multiselect_with_defaults<T: std::fmt::Display>(
    message: &str,
    options: Vec<T>,
    defaults: &[usize],
) -> Result<Vec<T>, WizardError> {
    require_tty()?;
    inquire::MultiSelect::new(message, options)
        .with_default(defaults)
        .prompt()
        .map_err(map_inquire)
}

/// Masked secret input (platform tokens → `.env`, never echoed).
pub fn password(message: &str) -> Result<String, WizardError> {
    require_tty()?;
    inquire::Password::new(message)
        .without_confirmation()
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .prompt()
        .map_err(map_inquire)
}

/// Multi-select list (SPACE toggles, ENTER confirms — Python semantics).
#[allow(dead_code)] // `multiselect_with_defaults` is the wizard's variant; kept for T08/T09.
pub fn multiselect<T: std::fmt::Display>(
    message: &str,
    options: Vec<T>,
) -> Result<Vec<T>, WizardError> {
    require_tty()?;
    inquire::MultiSelect::new(message, options)
        .prompt()
        .map_err(map_inquire)
}

/// Free-text input pre-filled with `initial`.
pub fn text_input(message: &str, initial: &str) -> Result<String, WizardError> {
    require_tty()?;
    inquire::Text::new(message)
        .with_initial_value(initial)
        .prompt()
        .map_err(map_inquire)
}

// ---------------------------------------------------------------------------
// Verbatim Python originals (hermes_cli/setup.py v0.21.0, READ-ONLY).
// ---------------------------------------------------------------------------

/// setup.py L2799 (`_offer_openclaw_migration`) — only asked upstream when
/// an OpenClaw install is detected; not wired in this port yet.
#[allow(dead_code)]
pub const IMPORT_QUESTION: &str = "Would you like to see what can be imported?";
/// setup.py L3303 (`_run_setup_wizard_impl`).
pub const MODE_QUESTION: &str = "How would you like to set up Hermes?";
/// setup.py L3305.
pub const MODE_QUICK: &str =
    "Quick Setup (Nous Portal) — free OAuth login, no API keys, model + tools (recommended)";
/// setup.py L3306.
pub const MODE_FULL: &str =
    "Full setup — configure every provider, tool & option yourself (bring your own keys)";
/// setup.py L3307.
pub const MODE_BLANK: &str =
    "Blank Slate — everything off except the bare minimum; opt in to each capability";
/// setup.py L3395-3398 (wizard section labels).
pub const SECTIONS: [&str; 4] = [
    "Model & Provider",
    "Terminal Backend",
    "Messaging Platforms",
    "Tools",
];
/// setup.py L3108 (`run_setup_action_with_navigation`).
pub const CANCELED_MESSAGE: &str = "Setup cancelled.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_tty_stdin_yields_clear_not_tty_error() {
        // Under `cargo test` stdin is piped → every prompt helper must refuse
        // with the deterministic invariant-8 error (same path the E2E asserts).
        assert!(!is_interactive());
        let err = confirm(IMPORT_QUESTION, false).expect_err("piped stdin must not prompt");
        assert_eq!(err, WizardError::NotTty);
        assert!(err.to_string().contains("interactive terminal"), "{}", err);
        assert_eq!(
            select_at("q", vec!["a"], 5).unwrap_err(),
            WizardError::NotTty
        );
        assert_eq!(password("q").unwrap_err(), WizardError::NotTty);
    }

    #[test]
    fn python_verbatim_strings_are_pinned() {
        assert_eq!(
            IMPORT_QUESTION,
            "Would you like to see what can be imported?"
        );
        assert_eq!(MODE_QUESTION, "How would you like to set up Hermes?");
        assert_eq!(
            MODE_QUICK,
            "Quick Setup (Nous Portal) — free OAuth login, no API keys, model + tools (recommended)"
        );
        assert_eq!(
            MODE_FULL,
            "Full setup — configure every provider, tool & option yourself (bring your own keys)"
        );
        assert_eq!(
            MODE_BLANK,
            "Blank Slate — everything off except the bare minimum; opt in to each capability"
        );
        assert_eq!(
            SECTIONS,
            [
                "Model & Provider",
                "Terminal Backend",
                "Messaging Platforms",
                "Tools",
            ]
        );
        assert_eq!(CANCELED_MESSAGE, "Setup cancelled.");
    }

    #[test]
    fn error_messages_drive_exit_codes() {
        // main() maps any error chain containing "interrupted" to 130.
        assert!(WizardError::Interrupted.to_string().contains("interrupted"));
        assert_eq!(
            WizardError::NotTty.to_string(),
            "setup wizard requires an interactive terminal (non-TTY stdin detected)"
        );
        assert_eq!(WizardError::Canceled.to_string(), "setup wizard cancelled");
    }
}
