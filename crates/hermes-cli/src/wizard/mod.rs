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
//! T01 is a **skeleton**: three steps (import → mode → sections) collect
//! answers and print a summary; nothing is written to disk (T05 adds the
//! real sections, the atomic config write and the backup). Question strings
//! that have a Python original are verbatim from `hermes_cli/setup.py`
//! v0.21.0 (provenance: `docs/HERMES_UI_SPEC.md` §C.2/§K); skeleton-only
//! strings are marked. All output is static strings (no untrusted content),
//! so the CLI-boundary sanitization/redaction contract is trivially met.

use std::io::{self, IsTerminal};

use inquire::InquireError;

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

fn require_tty() -> Result<(), WizardError> {
    if is_interactive() {
        Ok(())
    } else {
        Err(WizardError::NotTty)
    }
}

/// Yes/no prompt (ENTER confirms the highlighted value, ESC cancels).
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

/// Multi-select list (SPACE toggles, ENTER confirms — Python semantics).
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
#[allow(dead_code)] // T05 sections (config values) use this; T01 ships it complete.
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

/// setup.py L2799 (`_offer_openclaw_migration`).
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

/// Skeleton-only prompt (v0.21.0 has no original — the real wizard runs
/// every section in order). Style mirrors `Select platforms to configure:`.
const SECTIONS_QUESTION: &str = "Select sections to configure:";

/// Skeleton-only completion marker (clearly marked so E2E can pin it).
pub const COMPLETE_MARKER: &str = "Setup (skeleton) complete:";

// ---------------------------------------------------------------------------
// T01 skeleton
// ---------------------------------------------------------------------------

/// Result of one `run_skeleton` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkeletonResult {
    /// All three steps answered (the section list may be empty).
    Completed {
        import_offered: bool,
        mode: &'static str,
        sections: Vec<&'static str>,
    },
    /// ESC at some step → rollback to defaults (NOT an error, invariant 3).
    Canceled {
        import_offered: bool,
        mode: Option<&'static str>,
    },
}

impl SkeletonResult {
    #[allow(dead_code)] // used by unit tests and T05's outcome branching.
    pub fn is_completed(&self) -> bool {
        matches!(self, SkeletonResult::Completed { .. })
    }
}

/// T01 skeleton: import → mode → sections. Performs no writes (T05 adds
/// the real sections, atomic config write and backup).
pub fn run_skeleton() -> Result<SkeletonResult, WizardError> {
    require_tty()?;

    // Step 1 — import (default: no).
    let import_offered = confirm(IMPORT_QUESTION, false)?;

    // Step 2 — mode (default: first option = Quick Setup).
    let mode = match select(MODE_QUESTION, vec![MODE_QUICK, MODE_FULL, MODE_BLANK]) {
        Ok(mode) => mode,
        Err(WizardError::Canceled) => {
            return Ok(SkeletonResult::Canceled {
                import_offered,
                mode: None,
            })
        }
        Err(e) => return Err(e),
    };

    // Step 3 — sections (multi-select, nothing pre-selected).
    let sections = match multiselect(SECTIONS_QUESTION, SECTIONS.to_vec()) {
        Ok(sections) => sections,
        Err(WizardError::Canceled) => {
            return Ok(SkeletonResult::Canceled {
                import_offered,
                mode: Some(mode),
            })
        }
        Err(e) => return Err(e),
    };

    Ok(SkeletonResult::Completed {
        import_offered,
        mode,
        sections,
    })
}

/// CLI entry for the hidden `--setup-skeleton` flag (T05 replaces it with
/// `hermes setup`). Prints the outcome; exit-code semantics: Ok → 0
/// (cancel included), Err(Interrupted) → 130 via `main`'s "interrupted"
/// mapping, other Err → 1.
pub fn run_skeleton_cli() -> anyhow::Result<()> {
    match run_skeleton() {
        Ok(SkeletonResult::Completed {
            import_offered,
            mode,
            sections,
        }) => {
            // Skeleton-only summary block (no config written in T01).
            println!("{COMPLETE_MARKER}");
            println!("  import: {}", if import_offered { "yes" } else { "no" });
            println!("  mode: {mode}");
            let secs = if sections.is_empty() {
                "(none selected)".to_owned()
            } else {
                sections.join(", ")
            };
            println!("  sections: {secs}");
        }
        Ok(SkeletonResult::Canceled { .. }) => {
            println!("{CANCELED_MESSAGE}");
        }
        Err(e) => anyhow::bail!("{e}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_tty_stdin_yields_clear_not_tty_error() {
        // Under `cargo test` stdin is piped → the wizard must refuse with
        // the deterministic invariant-8 error (same path the E2E asserts).
        assert!(!is_interactive());
        let err = run_skeleton().expect_err("piped stdin must not run the wizard");
        assert_eq!(err, WizardError::NotTty);
        assert!(err.to_string().contains("interactive terminal"), "{}", err);
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

    #[test]
    fn skeleton_result_semantics() {
        let canceled = SkeletonResult::Canceled {
            import_offered: true,
            mode: None,
        };
        assert!(!canceled.is_completed());
        let completed = SkeletonResult::Completed {
            import_offered: false,
            mode: MODE_FULL,
            sections: vec!["Tools"],
        };
        assert!(completed.is_completed());
    }
}
