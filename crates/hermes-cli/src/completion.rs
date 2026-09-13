//! Spec 017 T08 — `/slash` autocomplete + ghost text (v0.21.0 parity).
//!
//! Behavioral port of Python `hermes_cli/commands.py` L1632-1830
//! (`SlashCommandCompleter` + `SlashCommandAutoSuggest`) for the rustyline
//! REPL, per spec §D.2 / §J.5 — parity is the *behavior*, not the crate:
//!
//! * **Command completion**: built-in slash commands + aliases from the
//!   verbatim `COMMAND_REGISTRY` (101 entries, `commands.py` L147) filtered
//!   to CLI-available commands (`gateway_only` excluded), plus the
//!   Hermes-RS-specific extensions (see [`RS_EXTENSIONS`]).
//! * **Subcommand completion**: second token of a command that declares
//!   `subcommands=` (e.g. `/skills s…`).
//! * **Skill completion**: drop-in skills from `~/.hermes/skills/`
//!   (directory name = skill name, `SKILL.md` frontmatter `description:` =
//!   short description). Skill tokens are normalized: underscore ≡ hyphen.
//! * **Stacked skill completions**: after `/skill-a ` a line whose remaining
//!   tokens are all skills offers the *other* skills, displayed as
//!   `⚡ {short_desc}` (commands.py L1741).
//! * **Path completions**: a word is a path when it starts with `./`, `../`,
//!   `~/` or `/`, or contains `/` — except URLs (`://`), which are excluded
//!   (commands.py L1772-1806).
//! * **Trailing-space trick**: completion text is `{cmd} ` so the dropdown
//!   stays reachable and backspace re-triggers — EXCEPT `_PICKER_COMMANDS`
//!   (`model`, `personality`, `skin`), which get no space so Enter opens the
//!   picker directly (commands.py L1752).
//! * **Ghost text** (auto-suggest): [`Hinter`] shows the remaining text of a
//!   unique completion (or the shared prefix of several), mirroring
//!   `SlashCommandAutoSuggest`; Tab accepts it via the completion menu.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;

/// One entry of Python `COMMAND_REGISTRY` (`CommandDef`).
///
/// Only the fields the completer needs are ported; the rest
/// (`busy_policy`, `busy_handler`, `argument_mode`, `execute`, `desktop`,
/// `gateway_config_gate`) belong to the gateway/REPL dispatcher and have no
/// completion role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandDef {
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub aliases: &'static [&'static str],
    pub args_hint: Option<&'static str>,
    pub subcommands: &'static [&'static str],
    pub cli_only: bool,
    pub gateway_only: bool,
}

/// `COMMAND_REGISTRY` (commands.py L147-1414) — 101 entries, ported
/// verbatim from `docs/hermes-ui-spec/017/verbatim/commands_registry.txt`
/// (AST `unparse`, Hermes Python v0.21.0). The `catalog_matches_verbatim_file`
/// test re-parses that file and compares every entry.
///
/// One provenance note: the `indicator` entry in the Python source has
/// non-literal kwargs (`args_hint=f'[{'|'.join(INDICATOR_STYLES)}]'`,
/// `subcommands=INDICATOR_STYLES`). Its style list is taken from the verbatim
/// tips catalog (`/indicator kaomoji|emoji|unicode|ascii …`) and is
/// therefore excluded from the verbatim field comparison in the test.
pub const COMMAND_REGISTRY: &[CommandDef] = &[
    CommandDef {
        name: "start",
        description: "Acknowledge platform start pings without a reply",
        category: "Session",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: true,
    },
    CommandDef {
        name: "new",
        description: "Start a new session (fresh session ID + history)",
        category: "Session",
        aliases: &["reset"],
        args_hint: Some("[name]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "topic",
        description: "Enable or inspect Telegram DM topic sessions",
        category: "Session",
        aliases: &[],
        args_hint: Some("[off|help|session-id]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: true,
    },
    CommandDef {
        name: "clear",
        description: "Clear screen and start a new session",
        category: "Session",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "redraw",
        description: "Force a full UI repaint (recovers from terminal drift)",
        category: "Session",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "history",
        description: "Show conversation history",
        category: "Session",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "save",
        description: "Export the current conversation (bare /save shows usage)",
        category: "Session",
        aliases: &[],
        args_hint: Some("<json|md|html> [filename] [redact]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "retry",
        description: "Retry the last message (resend to agent)",
        category: "Session",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "prompt",
        description: "Compose your next prompt in $EDITOR (markdown), then send it",
        category: "Session",
        aliases: &["compose"],
        args_hint: Some("[initial text]"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "undo",
        description: "Back up N user turns and re-prompt (default 1)",
        category: "Session",
        aliases: &[],
        args_hint: Some("[N]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "title",
        description: "Set a title for the current session",
        category: "Session",
        aliases: &[],
        args_hint: Some("[name]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "handoff",
        description: "Hand off this session to a messaging platform (Telegram, Discord, etc.)",
        category: "Session",
        aliases: &[],
        args_hint: Some("<platform>"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "branch",
        description: "Branch the current session (explore a different path)",
        category: "Session",
        aliases: &["fork"],
        args_hint: Some("[name]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "worktree",
        description: "Show, list, create, or prune isolated git worktrees",
        category: "Session",
        aliases: &[],
        args_hint: Some("[new [name]|list|prune [--dry-run]]"),
        subcommands: &["new", "list", "prune"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "compress",
        description: "Compress conversation context (add 'here [N]' to keep recent N turns; --preview shows what would happen)",
        category: "Session",
        aliases: &["compact"],
        args_hint: Some("[here [N] | focus topic | --preview|--dry-run]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "rollback",
        description: "List or restore filesystem checkpoints (restores keep your hand-edits; --all overrides)",
        category: "Session",
        aliases: &[],
        args_hint: Some("[number] [--all]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "snapshot",
        description: "Create or restore state snapshots of Hermes config/state",
        category: "Session",
        aliases: &["snap"],
        args_hint: Some("[create|restore <id>|prune]"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "export",
        description: "Export a profile (config, skills, theme) to a shareable archive",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[profile] [-o output.tar.gz]"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "import",
        description: "Import a shared profile archive as a new profile",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("<archive.tar.gz> [--name <name>]"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "stop",
        description: "Kill all running background processes",
        category: "Session",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "pause",
        description: "Pause new work globally (emergency stop); '/pause off' resumes",
        category: "Session",
        aliases: &[],
        args_hint: Some("[reason | off]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: true,
    },
    CommandDef {
        name: "approve",
        description: "Approve a pending dangerous command",
        category: "Session",
        aliases: &[],
        args_hint: Some("[session|always]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: true,
    },
    CommandDef {
        name: "deny",
        description: "Deny a pending dangerous command (optionally with a reason)",
        category: "Session",
        aliases: &[],
        args_hint: Some("[all] [reason]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: true,
    },
    CommandDef {
        name: "bg",
        description: "Run a prompt in a separate background session",
        category: "Session",
        aliases: &[],
        args_hint: Some("<prompt>"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "btw",
        description: "Ask a side question about the current conversation without interrupting it",
        category: "Session",
        aliases: &[],
        args_hint: Some("<question>"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "agents",
        description: "Show active agents and running tasks",
        category: "Session",
        aliases: &["tasks"],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "journey",
        description: "Open the learning journey timeline",
        category: "Session",
        aliases: &["learning", "memory-graph"],
        args_hint: Some("[list|delete <id>|edit <id>]"),
        subcommands: &["list", "delete", "edit"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "queue",
        description: "Queue a prompt for the next turn (doesn't interrupt)",
        category: "Session",
        aliases: &["q"],
        args_hint: Some("<prompt>"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "steer",
        description: "Inject a message after the next tool call without interrupting",
        category: "Session",
        aliases: &[],
        args_hint: Some("<prompt>"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "goal",
        description: "Set a standing goal Hermes works on across turns until achieved",
        category: "Session",
        aliases: &[],
        args_hint: Some("[text | draft <text> | show | gate add <cmd> | pause | resume | clear | status | wait <pid> | unwait]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "heartbeat",
        description: "Set a recurring prompt that re-enters this session when idle",
        category: "Session",
        aliases: &["hb"],
        args_hint: Some("[every <interval> <prompt> | status | pause | resume | clear]"),
        subcommands: &["status", "pause", "resume", "clear"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "refine",
        description: "Review this conversation now and save lessons to memory/skills",
        category: "Session",
        aliases: &[],
        args_hint: Some("[focus instructions]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "review",
        description: "Spawn an independent subagent to review the work just discussed (PR, code, docs)",
        category: "Session",
        aliases: &[],
        args_hint: Some("[review instructions]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "loop",
        description: "Re-run a prompt on a recurring interval in this session",
        category: "Session",
        aliases: &["proactive"],
        args_hint: Some("[interval] <prompt> [--times N] [--until <condition>] | status | pause | resume | stop"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "plan",
        description: "Write a markdown implementation plan to .hermes/plans/ without executing anything",
        category: "Session",
        aliases: &[],
        args_hint: Some("[task]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "moa",
        description: "Run one prompt through the default Mixture of Agents preset, then restore your model",
        category: "Session",
        aliases: &[],
        args_hint: Some("<prompt>"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "subgoal",
        description: "Add or manage extra criteria on the active goal",
        category: "Session",
        aliases: &[],
        args_hint: Some("[text | remove N | clear]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "status",
        description: "Show session, model, token, and context info",
        category: "Session",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "egress",
        description: "Show Docker egress proxy status",
        category: "Session",
        aliases: &[],
        args_hint: Some("[status]"),
        subcommands: &["status"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "context",
        description: "Show detailed context window view with usage gauge, category breakdown, compression stats, and throughput",
        category: "Session",
        aliases: &["ctx"],
        args_hint: Some("[all]"),
        subcommands: &["all"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "whoami",
        description: "Show your slash command access (admin / user)",
        category: "Info",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "profile",
        description: "Show active profile name and home directory",
        category: "Info",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "sethome",
        description: "Set this chat as the home channel",
        category: "Session",
        aliases: &["set-home"],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: true,
    },
    CommandDef {
        name: "resume",
        description: "Resume a previously-named session",
        category: "Session",
        aliases: &[],
        args_hint: Some("[name]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "sessions",
        description: "Browse and resume previous sessions",
        category: "Session",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "config",
        description: "Show current configuration",
        category: "Configuration",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "model",
        description: "Switch model (session-scoped; --global to persist)",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[model] [--provider name] [--global|--session] [--refresh]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "codex-runtime",
        description: "Toggle codex app-server runtime for OpenAI/Codex models",
        category: "Configuration",
        aliases: &["codex_runtime"],
        args_hint: Some("[auto|codex_app_server]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "personality",
        description: "Set a predefined personality",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[name]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "statusbar",
        description: "Toggle the context/model status bar",
        category: "Configuration",
        aliases: &["sb"],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "battery",
        description: "Toggle a color-coded battery indicator in the status bar",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[on|off|status]"),
        subcommands: &["on", "off", "status"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "timestamps",
        description: "Toggle [HH:MM] timestamps on messages and /history",
        category: "Configuration",
        aliases: &["ts"],
        args_hint: Some("[on|off|status]"),
        subcommands: &["on", "off", "status"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "diff",
        description: "Show git changes in the working directory",
        category: "Info",
        aliases: &[],
        args_hint: Some("[staged|all|session] [--stat] [path...]"),
        subcommands: &["staged", "all", "session"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "verbose",
        description: "Cycle tool progress display: off -> new -> all -> verbose",
        category: "Configuration",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "focus",
        description: "Toggle focus view — show only your prompt and the final response",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[on|off|status]"),
        subcommands: &["on", "off", "status"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "footer",
        description: "Toggle gateway runtime-metadata footer on final replies",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[on|off|status]"),
        subcommands: &["on", "off", "status"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "yolo",
        description: "Toggle YOLO mode (skip all dangerous command approvals)",
        category: "Configuration",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "approvals",
        description: "Show or set the persistent dangerous-command approval mode",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[manual|smart|off]"),
        subcommands: &["manual", "smart", "off"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "reasoning",
        description: "Manage reasoning effort and display",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[level|show|hide|full|clamp] [--global]"),
        subcommands: &["none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra", "show", "hide", "on", "off", "full", "clamp", "--global"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "fast",
        description: "Fast mode — OpenAI Priority Processing / Anthropic Fast Mode (normal/fast/auto/cold)",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[normal|fast|auto|cold|status] [--global]"),
        subcommands: &["normal", "fast", "auto", "cold", "status", "on", "off", "--global"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "skin",
        description: "Show or change the display skin/theme",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[name]"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "indicator",
        description: "Pick the TUI busy-indicator style",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[kaomoji|emoji|unicode|ascii]"),
        subcommands: &["kaomoji", "emoji", "unicode", "ascii"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "voice",
        description: "Toggle voice mode",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[on|off|tts|status]"),
        subcommands: &["on", "off", "tts", "status"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "wake",
        description: "Toggle the 'Hey Hermes' wake word listener",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[on|off|status]"),
        subcommands: &["on", "off", "status"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "busy",
        description: "Control how messages behave while Hermes is working",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[queue|steer|interrupt|status]"),
        subcommands: &["queue", "steer", "interrupt", "status"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "tools",
        description: "Manage tools: /tools [list|disable|enable] [name...]",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: Some("[list|disable|enable] [name...]"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "toolsets",
        description: "List available toolsets",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "skills",
        description: "Search, install, inspect, or manage skills",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: None,
        subcommands: &["search", "browse", "inspect", "install", "audit", "pending", "approve", "reject", "diff", "approval"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "memory",
        description: "Review pending memory writes / toggle the approval gate",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: Some("[pending|approve|reject|approval] [id|on|off]"),
        subcommands: &["pending", "approve", "reject", "approval"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "bundles",
        description: "List skill bundles (aliases /<name> for multiple skills)",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "pet",
        description: "Toggle or adopt a petdex mascot (/pet, /pet list, /pet <slug>)",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: Some("[toggle|list|scale <n>|<slug>]"),
        subcommands: &["toggle", "list", "scale", "off"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "hatch",
        description: "Generate a new petdex pet from a description",
        category: "Tools & Skills",
        aliases: &["generate-pet"],
        args_hint: Some("[description]"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "learn",
        description: "Learn a reusable skill from anything you describe (dirs, URLs, this chat, notes)",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: Some("<what to learn from>"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "init",
        description: "Generate or update AGENTS.md project instructions from a repo scan",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: Some("[notes]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "cron",
        description: "Manage scheduled tasks",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: Some("[subcommand]"),
        subcommands: &["list", "add", "create", "edit", "pause", "resume", "run", "remove"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "suggestions",
        description: "Review suggested automations (accept/dismiss)",
        category: "Tools & Skills",
        aliases: &["suggest"],
        args_hint: Some("[accept|dismiss N | catalog]"),
        subcommands: &["accept", "dismiss", "catalog", "clear"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "blueprint",
        description: "Set up an automation from a blueprint template",
        category: "Tools & Skills",
        aliases: &["bp"],
        args_hint: Some("[name] [slot=value ...]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "curator",
        description: "Background skill maintenance (status, run, pin, archive, list-archived)",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: Some("[subcommand]"),
        subcommands: &["status", "run", "pause", "resume", "pin", "unpin", "restore", "list-archived"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "kanban",
        description: "Multi-profile collaboration board (tasks, links, comments)",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: Some("[subcommand]"),
        subcommands: &["init", "boards", "create", "list", "ls", "show", "assign", "reclaim", "reassign", "diagnostics", "diag", "link", "unlink", "claim", "comment", "complete", "edit", "block", "unblock", "archive", "tail", "dispatch", "stats", "notify-subscribe", "notify-list", "notify-unsubscribe", "log", "runs", "heartbeat", "assignees", "context", "specify", "gc"],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "reload",
        description: "Reload .env variables into the running session",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "reload-mcp",
        description: "Reload MCP servers from config",
        category: "Tools & Skills",
        aliases: &["reload_mcp"],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "reload-skills",
        description: "Re-scan ~/.hermes/skills/ for newly installed or removed skills",
        category: "Tools & Skills",
        aliases: &["reload_skills"],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "browser",
        description: "Connect browser tools to your live Chromium-family browser via CDP, or switch to Browser Use mode",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: Some("[connect|disconnect|status|use]"),
        subcommands: &["connect", "disconnect", "status", "use"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "plugins",
        description: "List installed plugins and their status",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "commands",
        description: "Browse all commands and skills (paginated)",
        category: "Info",
        aliases: &[],
        args_hint: Some("[page]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: true,
    },
    CommandDef {
        name: "help",
        description: "Show available commands (/help skills lists skill commands, /help <text> filters)",
        category: "Info",
        aliases: &[],
        args_hint: Some("[skills|<filter>]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "palette",
        description: "Open the fuzzy command palette (also Ctrl+P)",
        category: "Info",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "restart",
        description: "Gracefully restart the gateway after draining active runs",
        category: "Session",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: true,
    },
    CommandDef {
        name: "usage",
        description: "Show token usage and rate limits; `reset` redeems a banked Codex limit reset",
        category: "Info",
        aliases: &[],
        args_hint: Some("[reset [--force]]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "subscription",
        description: "View your Nous plan and change it in the browser",
        category: "Info",
        aliases: &["upgrade"],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "topup",
        description: "Show your Nous balance and manage billing on the portal",
        category: "Info",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "insights",
        description: "Show usage insights and analytics",
        category: "Info",
        aliases: &[],
        args_hint: Some("[days]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "platforms",
        description: "Show gateway/messaging platform status",
        category: "Info",
        aliases: &["gateway"],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "platform",
        description: "Pause, resume, or list a failing gateway platform",
        category: "Info",
        aliases: &[],
        args_hint: Some("<pause|resume|list> [name]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: true,
    },
    CommandDef {
        name: "copy",
        description: "Copy the last assistant response to clipboard",
        category: "Info",
        aliases: &[],
        args_hint: Some("[number]"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "paste",
        description: "Attach clipboard image from your clipboard",
        category: "Info",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "image",
        description: "Attach a local image file for your next prompt",
        category: "Info",
        aliases: &[],
        args_hint: Some("<path>"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "update",
        description: "Update Hermes Agent to the latest version",
        category: "Info",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "version",
        description: "Show Hermes Agent version",
        category: "Info",
        aliases: &["v"],
        args_hint: None,
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "debug",
        description: "Upload debug report (system info + logs) and get shareable links",
        category: "Info",
        aliases: &[],
        args_hint: Some("[nous|local]"),
        subcommands: &[],
        cli_only: false,
        gateway_only: false,
    },
    CommandDef {
        name: "quit",
        description: "Exit the CLI (use --delete to also remove session history)",
        category: "Exit",
        aliases: &["exit"],
        args_hint: Some("[--delete]"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
];

/// `_PICKER_COMMANDS` (commands.py L1752): commands that open a picker on
/// Enter. Their completion text gets **no** trailing space.
const PICKER_COMMANDS: &[&str] = &["model", "personality", "skin"];

/// Hermes-RS REPL commands that have **no** v0.21.0 `COMMAND_REGISTRY`
/// entry (Rust-side additions from earlier specs: the Spec 003 inspection
/// family, the Spec 005 `/provider` switch, the Spec 007 `/sandbox` display,
/// Spec 011b `/mcp` management, the Spec 008 pin family, the
/// mascot/petdex displays). They work in the Hermes-RS dispatch, so the
/// completer offers them alongside the verbatim registry.
pub const RS_EXTENSIONS: &[CommandDef] = &[
    CommandDef {
        name: "tool-calls",
        description: "Show tool calls in a session (usage: /tool-calls <id>)",
        category: "Info",
        aliases: &[],
        args_hint: Some("<id>"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "inspect",
        description: "Inspect session metadata (usage: /inspect <id>)",
        category: "Info",
        aliases: &[],
        args_hint: Some("<id>"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "messages",
        description: "Show messages in a session (usage: /messages <id>)",
        category: "Info",
        aliases: &[],
        args_hint: Some("<id>"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "search",
        description: "Search message history (usage: /search <query>)",
        category: "Info",
        aliases: &[],
        args_hint: Some("<query>"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "info",
        description: "Show provider & context accounting",
        category: "Info",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "provider",
        description: "Switch active provider (usage: /provider [name])",
        category: "Configuration",
        aliases: &[],
        args_hint: Some("[name]"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "sandbox",
        description: "Show the tool execution sandbox policy (Spec 007)",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "mcp",
        description: "Show MCP server status and manage servers",
        category: "Tools & Skills",
        aliases: &[],
        args_hint: Some("[list|restart <name>]"),
        subcommands: &["list", "restart"],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "pinned",
        description: "List pinned turns in current session",
        category: "Session",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "pin",
        description: "Pin turn against compression (usage: /pin <n>)",
        category: "Session",
        aliases: &[],
        args_hint: Some("<n>"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "unpin",
        description: "Unpin turn (usage: /unpin <n>)",
        category: "Session",
        aliases: &[],
        args_hint: Some("<n>"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "reflect",
        description: "Reflection mode [on|off]",
        category: "Session",
        aliases: &[],
        args_hint: Some("[on|off]"),
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "mascot",
        description: "Show current Hermes mascot/pet",
        category: "Info",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
    CommandDef {
        name: "petdex",
        description: "Browse Petdex companions",
        category: "Info",
        aliases: &[],
        args_hint: None,
        subcommands: &[],
        cli_only: true,
        gateway_only: false,
    },
];

/// A drop-in skill discovered under `<hermes-home>/skills/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    /// Directory name (the canonical skill token).
    pub name: String,
    /// `short_desc` for the `⚡ {short_desc}` display — the
    /// `description:` key of `SKILL.md` YAML frontmatter, when present.
    pub description: String,
}

/// `~/.hermes/skills` layout (spec §C.7, tip: `/reload-skills re-scans
/// ~/.hermes/skills/`): every non-hidden **subdirectory** is a drop-in
/// skill; the name is the directory name and the short description comes
/// from the `SKILL.md` frontmatter when the file exists.
pub fn discover_skills(skills_dir: &Path) -> Vec<Skill> {
    let mut skills = Vec::new();
    if let Ok(entries) = std::fs::read_dir(skills_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let description = skill_description(&path.join("SKILL.md"));
            skills.push(Skill { name, description });
        }
    }
    skills.sort_by_key(|a| a.name.to_ascii_lowercase());
    skills
}

/// Best-effort `description:` extraction from `SKILL.md` YAML frontmatter
/// (the leading `---` fenced block).
fn skill_description(skill_md: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(skill_md) else {
        return String::new();
    };
    let Some(rest) = text.trim_start().strip_prefix("---") else {
        return String::new();
    };
    let Some(end) = rest.find("\n---") else {
        return String::new();
    };
    for line in rest[..end].lines() {
        if let Some(value) = line.trim().strip_prefix("description:") {
            let value = value.trim();
            return value.trim_matches(|c| c == '"' || c == '\'').to_string();
        }
    }
    String::new()
}

/// Skill-token normalization (commands.py): underscore ≡ hyphen, compared
/// case-insensitively.
pub fn normalize_skill_token(s: &str) -> String {
    s.to_ascii_lowercase().replace('_', "-")
}

/// Normalized key for a skill token: a leading `/` is stripped first
/// (skills are invoked as `/name`), then [`normalize_skill_token`].
pub fn skill_key(s: &str) -> String {
    normalize_skill_token(s.strip_prefix('/').unwrap_or(s))
}

/// A single completion candidate: `replacement` is inserted at the word
/// start (the trailing-space trick lives here); `display` is the menu text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub replacement: String,
    pub display: String,
}

/// Upper bound for the `⚡ {short_desc}` skill display (the exact upstream
/// limit is not documented in the verbatim artifacts — 60 chars keeps the
/// dropdown readable; see the T08 issue, open item).
const SKILL_DESC_LIMIT: usize = 60;
/// Upper bound for filesystem completion candidates.
const PATH_CANDIDATE_LIMIT: usize = 100;

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        let head: String = chars[..max].iter().collect();
        format!("{head}…")
    }
}

/// Path-likeness rule (commands.py L1772-1806): starts with `./`, `../`,
/// `~/`, `/`, or contains `/` — URLs (`://`) are excluded.
pub fn is_path_token(token: &str) -> bool {
    if token.contains("://") {
        return false;
    }
    token.starts_with("./")
        || token.starts_with("../")
        || token.starts_with("~/")
        || token.starts_with('/')
        || token.contains('/')
}

/// Filesystem candidates for a path-like token. `~` expands to the user's
/// home directory; a relative base resolves against `cwd`; directory
/// entries keep a trailing `/`.
fn path_candidates(token: &str, user_home: &Path, cwd: &Path) -> Vec<Candidate> {
    let home_rel = token.starts_with("~/");
    let expanded = if home_rel {
        // `~/x` -> `{user_home}/x` (drop only the `~`, keep the `/`).
        format!("{}/{}", user_home.display(), &token[2..])
    } else {
        token.to_string()
    };
    let (base, prefix) = match expanded.rsplit_once('/') {
        Some(x) => x,
        None => return Vec::new(),
    };
    let dir: String = if base.is_empty() {
        "/".to_string()
    } else if home_rel || base.starts_with('/') {
        base.to_string()
    } else {
        cwd.join(base).display().to_string()
    };
    let dir = dir.as_str();
    // Rebuild the user-visible base: keep `~/` spelling for home-relative
    // paths instead of the expanded absolute directory.
    let display_base = if token.starts_with("~/") {
        match base.strip_prefix(user_home.to_string_lossy().as_ref()) {
            Some(rest) => format!("~{rest}"),
            None => base.to_string(),
        }
    } else {
        base.to_string()
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(String, bool)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && !prefix.starts_with('.') {
                return None;
            }
            Some((name, e.path().is_dir()))
        })
        .filter(|(name, _)| name.starts_with(prefix))
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
        .into_iter()
        .take(PATH_CANDIDATE_LIMIT)
        .map(|(name, is_dir)| {
            let mut replacement = if display_base.is_empty() {
                format!("/{name}")
            } else {
                format!("{display_base}/{name}")
            };
            if is_dir {
                replacement.push('/');
            }
            let display = replacement.clone();
            Candidate {
                replacement,
                display,
            }
        })
        .collect()
}

/// The rustyline helper: completion + ghost text for the Hermes REPL.
#[derive(Clone)]
pub struct HermesCompleter {
    /// Installed skills (scanned once at REPL start).
    pub(crate) skills: Vec<Skill>,
    /// User home for `~` expansion (NOT the Hermes home).
    pub(crate) user_home: PathBuf,
    /// Working directory relative path tokens resolve against.
    pub(crate) cwd: PathBuf,
}

impl Default for HermesCompleter {
    fn default() -> Self {
        Self {
            skills: Vec::new(),
            user_home: PathBuf::new(),
            cwd: PathBuf::new(),
        }
    }
}

impl HermesCompleter {
    /// `hermes_home` = `~/.hermes` (skills live under `skills/`); the user's
    /// own home is resolved from the environment for `~` path expansion.
    pub fn new(hermes_home: &Path) -> Self {
        let user_home = std::env::home_dir().unwrap_or_default();
        let cwd = std::env::current_dir().unwrap_or_default();
        Self::with_home(hermes_home, &user_home, &cwd)
    }

    /// Explicit homes + base dir (used by tests so no environment is needed).
    pub fn with_home(hermes_home: &Path, user_home: &Path, cwd: &Path) -> Self {
        Self {
            skills: discover_skills(&hermes_home.join("skills")),
            user_home: user_home.to_path_buf(),
            cwd: cwd.to_path_buf(),
        }
    }

    /// CLI-available commands: verbatim registry minus `gateway_only`,
    /// plus the Hermes-RS extensions.
    fn cli_commands(&self) -> Vec<&'static CommandDef> {
        let mut out: Vec<&'static CommandDef> = COMMAND_REGISTRY
            .iter()
            .filter(|c| !c.gateway_only)
            .collect();
        out.extend(RS_EXTENSIONS.iter());
        out
    }

    fn command_by_token(&self, token: &str) -> Option<&'static CommandDef> {
        self.cli_commands()
            .iter()
            .find(|d| {
                d.name.eq_ignore_ascii_case(token)
                    || d.aliases.iter().any(|a| a.eq_ignore_ascii_case(token))
            })
            .copied()
    }

    fn is_skill_token(&self, token: &str) -> bool {
        let n = skill_key(token);
        self.skills.iter().any(|s| skill_key(&s.name) == n)
    }

    /// First-word candidates: commands (+aliases) and skills matching the
    /// prefix (the prefix is the token without its leading `/`).
    fn command_candidates(&self, prefix: &str) -> Vec<Candidate> {
        let prefix = prefix.to_ascii_lowercase();
        let mut seen: HashSet<String> = HashSet::new();
        let mut out = Vec::new();
        for def in self.cli_commands() {
            if def.name.to_ascii_lowercase().starts_with(&prefix) {
                let replacement = if PICKER_COMMANDS.contains(&def.name) {
                    format!("/{}", def.name)
                } else {
                    format!("/{} ", def.name)
                };
                if seen.insert(replacement.clone()) {
                    out.push(Candidate {
                        replacement,
                        display: format!("{:<20} {}", def.name, def.description),
                    });
                }
            }
            for alias in def.aliases {
                if alias.to_ascii_lowercase().starts_with(&prefix) {
                    let replacement = format!("/{alias} ");
                    if seen.insert(replacement.clone()) {
                        out.push(Candidate {
                            replacement,
                            display: format!("{alias:<20} alias for /{}", def.name),
                        });
                    }
                }
            }
        }
        out
    }

    /// Skill candidates matching the normalized prefix, minus `used`
    /// (already-present skill tokens in a stacked line). The replacement
    /// mirrors the input: a leading `/` in the token is kept (skills are
    /// invoked as `/name`), the trailing space applies the usual trick.
    fn skill_candidates(&self, token: &str, used: &HashSet<String>) -> Vec<Candidate> {
        let with_slash = token.starts_with('/');
        let prefix = skill_key(token);
        self.skills
            .iter()
            .filter(|s| {
                let n = skill_key(&s.name);
                n.starts_with(&prefix) && !used.contains(&n)
            })
            .map(|s| {
                let desc = if s.description.is_empty() {
                    s.name.clone()
                } else {
                    truncate(&s.description, SKILL_DESC_LIMIT)
                };
                let replacement = if with_slash {
                    format!("/{} ", s.name)
                } else {
                    format!("{} ", s.name)
                };
                Candidate {
                    replacement,
                    display: format!("⚡ {desc}"),
                }
            })
            .collect()
    }

    fn subcommand_candidates(&self, cmd: &CommandDef, token: &str) -> Vec<Candidate> {
        let token = token.to_ascii_lowercase();
        cmd.subcommands
            .iter()
            .filter(|s| s.to_ascii_lowercase().starts_with(&token))
            .map(|s| Candidate {
                replacement: format!("{s} "),
                display: s.to_string(),
            })
            .collect()
    }

    /// The shared completion core: returns `(start, candidates)` where the
    /// replacement applies at byte offset `start` of the line (up to `pos`).
    fn candidates(&self, line: &str, pos: usize) -> (usize, Vec<Candidate>) {
        let pos = pos.min(line.len());
        let upto = &line[..pos];
        let word_start = upto
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        let token = &upto[word_start..];
        let is_first = word_start == 0;

        if is_first {
            if token.is_empty() {
                return (pos, Vec::new());
            }
            if let Some(prefix) = token.strip_prefix('/') {
                let mut cands = self.command_candidates(prefix);
                let used = HashSet::new();
                cands.extend(self.skill_candidates(token, &used));
                if !cands.is_empty() {
                    return (0, cands);
                }
                // No command/skill matches: the word may be an absolute path
                // typed as a message (spec: path rule applies to any word).
                if is_path_token(token) {
                    return (0, path_candidates(token, &self.user_home, &self.cwd));
                }
                return (0, Vec::new());
            }
            if is_path_token(token) {
                return (0, path_candidates(token, &self.user_home, &self.cwd));
            }
            return (0, Vec::new());
        }

        let first_word = upto[..word_start].split_whitespace().next().unwrap_or("");
        if let Some(cmd_token) = first_word.strip_prefix('/') {
            if let Some(cmd) = self.command_by_token(cmd_token) {
                if !cmd.subcommands.is_empty() {
                    let cands = self.subcommand_candidates(cmd, token);
                    if !cands.is_empty() {
                        return (word_start, cands);
                    }
                }
            } else if self.is_skill_token(cmd_token) {
                // Stacked skills (commands.py L1741): every completed token
                // between the first word and the cursor must itself be a
                // skill; then offer the remaining skills.
                let middle: Vec<&str> = upto[..word_start].split_whitespace().skip(1).collect();
                if middle.iter().all(|t| self.is_skill_token(t)) {
                    let used: HashSet<String> = std::iter::once(cmd_token)
                        .chain(middle.iter().copied())
                        .map(skill_key)
                        .collect();
                    let cands = self.skill_candidates(token, &used);
                    if !cands.is_empty() {
                        return (word_start, cands);
                    }
                }
            }
        }
        if is_path_token(token) {
            return (
                word_start,
                path_candidates(token, &self.user_home, &self.cwd),
            );
        }
        (word_start, Vec::new())
    }
}

impl Completer for HermesCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let (start, cands) = self.candidates(line, pos);
        Ok((
            start,
            cands
                .into_iter()
                .map(|c| Pair {
                    display: c.display,
                    replacement: c.replacement,
                })
                .collect(),
        ))
    }
}

impl HermesCompleter {
    /// Ghost text for the cursor position (the rustyline hint): the
    /// remainder of the unique completion, or the shared-prefix remainder
    /// when several candidates agree (SlashCommandAutoSuggest behavior).
    pub(crate) fn ghost_suffix(&self, line: &str, pos: usize) -> Option<String> {
        let (word_start, cands) = self.candidates(line, pos);
        if cands.is_empty() {
            return None;
        }
        let token = &line[word_start..pos.min(line.len())];
        let common = if cands.len() == 1 {
            cands[0].replacement.clone()
        } else {
            let mut common = cands[0].replacement.clone();
            for c in &cands[1..] {
                common.truncate(common_prefix_len(&common, &c.replacement));
            }
            common
        };
        let suffix = common.strip_prefix(token)?;
        (!suffix.is_empty()).then(|| suffix.to_string())
    }
}

impl Hinter for HermesCompleter {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        self.ghost_suffix(line, pos)
    }
}

fn common_prefix_len(a: &str, b: &str) -> usize {
    let n = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    a.chars().take(n).map(char::len_utf8).sum()
}

impl Highlighter for HermesCompleter {}
impl Validator for HermesCompleter {}
impl Helper for HermesCompleter {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const VERBATIM: &str =
        include_str!("../../../docs/hermes-ui-spec/017/verbatim/commands_registry.txt");

    fn cand_completer() -> HermesCompleter {
        HermesCompleter::default()
    }

    /// The pure completion core (no rustyline Context needed in tests).
    fn complete(c: &HermesCompleter, line: &str, pos: usize) -> (usize, Vec<Candidate>) {
        c.candidates(line, pos)
    }

    fn hint(c: &HermesCompleter, line: &str, pos: usize) -> Option<String> {
        c.ghost_suffix(line, pos)
    }

    fn names(cands: &[Candidate]) -> Vec<&str> {
        cands.iter().map(|p| p.replacement.as_str()).collect()
    }

    // ---- catalog port integrity -------------------------------------------

    #[test]
    fn registry_has_101_entries() {
        assert_eq!(COMMAND_REGISTRY.len(), 101);
        assert!(COMMAND_REGISTRY.iter().all(|c| !c.description.is_empty()));
    }

    #[test]
    fn no_duplicate_command_or_alias_names() {
        let mut seen: HashSet<String> = HashSet::new();
        for c in COMMAND_REGISTRY.iter().chain(RS_EXTENSIONS.iter()) {
            assert!(
                seen.insert(c.name.to_string()),
                "duplicate {name:?}",
                name = c.name
            );
            for a in c.aliases {
                assert!(seen.insert((*a).to_string()), "duplicate alias {a}");
            }
        }
    }

    // A small parser for the AST-unparsed `CommandDef(...)` list. Only the
    // fields the port claims to mirror are extracted.
    struct VerbatimDef {
        name: String,
        description: String,
        category: String,
        aliases: Vec<String>,
        args_hint: Option<String>, // None => absent or non-literal
        args_hint_nonliteral: bool,
        subcommands: Option<Vec<String>>, // None => absent or non-literal
        subcommands_nonliteral: bool,
        cli_only: bool,
        gateway_only: bool,
    }

    fn balanced_close(text: &str, open: usize) -> Option<usize> {
        let bytes = text.as_bytes();
        let mut depth = 0i32;
        let mut quote: Option<u8> = None;
        let mut i = open;
        while i < bytes.len() {
            let c = bytes[i];
            match quote {
                Some(q) => {
                    if c == b'\\' {
                        i += 1;
                    } else if c == q {
                        quote = None;
                    }
                }
                None => match c {
                    b'\'' | b'"' => quote = Some(c),
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(i);
                        }
                    }
                    _ => {}
                },
            }
            i += 1;
        }
        None
    }

    fn split_top(s: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut depth = 0i32;
        let mut quote: Option<u8> = None;
        let mut cur = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            match quote {
                Some(q) => {
                    cur.push(c);
                    if c == '\\' {
                        if let Some(n) = chars.next() {
                            cur.push(n);
                        }
                    } else if c as u8 == q {
                        quote = None;
                    }
                }
                None => match c {
                    '\'' | '"' => {
                        quote = Some(c as u8);
                        cur.push(c);
                    }
                    '(' => {
                        depth += 1;
                        cur.push(c);
                    }
                    ')' => {
                        depth -= 1;
                        cur.push(c);
                    }
                    ',' if depth == 0 => {
                        out.push(cur.trim().to_string());
                        cur.clear();
                    }
                    _ => cur.push(c),
                },
            }
        }
        if !cur.trim().is_empty() {
            out.push(cur.trim().to_string());
        }
        out
    }

    fn unquote(s: &str) -> String {
        let s = s.trim();
        let b = s.as_bytes();
        let is_quote = |c: u8| c == 0x27 || c == 0x22; // 0x27 = '  0x22 = "
        if b.len() >= 2 && is_quote(b[0]) && b[b.len() - 1] == b[0] {
            let mut out = String::with_capacity(s.len() - 2);
            let mut chars = s[1..s.len() - 1].chars().peekable();
            while let Some(c) = chars.next() {
                if c as u8 == 0x5C {
                    // 0x5C = backslash: drop it, keep the next char as-is.
                    if let Some(n) = chars.next() {
                        out.push(n);
                    }
                } else {
                    out.push(c);
                }
            }
            out
        } else {
            s.to_string()
        }
    }

    fn parse_tuple(v: &str) -> Vec<String> {
        let inner = v.trim().trim_start_matches('(').trim_end_matches(')');
        if inner.trim().is_empty() {
            return Vec::new();
        }
        split_top(inner).into_iter().map(|s| unquote(&s)).collect()
    }

    fn parse_verbatim(text: &str) -> Vec<VerbatimDef> {
        let mut out = Vec::new();
        let mut from = 0usize;
        while let Some(rel) = text[from..].find("CommandDef(") {
            let open = from + rel + "CommandDef(".len() - 1;
            let close = balanced_close(text, open).expect("unbalanced CommandDef");
            let fields = split_top(&text[open + 1..close]);
            let mut d = VerbatimDef {
                name: unquote(&fields[0]),
                description: unquote(&fields[1]),
                category: unquote(&fields[2]),
                aliases: Vec::new(),
                args_hint: None,
                args_hint_nonliteral: false,
                subcommands: Some(Vec::new()),
                subcommands_nonliteral: false,
                cli_only: false,
                gateway_only: false,
            };
            for f in fields.iter().skip(3) {
                let (k, v) = f.split_once('=').expect("kwarg");
                let v = v.trim();
                match k.trim() {
                    "aliases" => {
                        if v.starts_with('(') {
                            d.aliases = parse_tuple(v);
                        }
                    }
                    "args_hint" => {
                        if v.starts_with('\'') || v.starts_with('"') {
                            d.args_hint = Some(unquote(v));
                        } else {
                            d.args_hint_nonliteral = true;
                        }
                    }
                    "subcommands" => {
                        if v.starts_with('(') {
                            d.subcommands = Some(parse_tuple(v));
                        } else {
                            d.subcommands = None;
                            d.subcommands_nonliteral = true;
                        }
                    }
                    "cli_only" => d.cli_only = v == "True",
                    "gateway_only" => d.gateway_only = v == "True",
                    _ => {}
                }
            }
            out.push(d);
            from = close + 1;
        }
        out
    }

    #[test]
    fn catalog_matches_verbatim_file() {
        let parsed = parse_verbatim(VERBATIM);
        assert_eq!(parsed.len(), COMMAND_REGISTRY.len(), "entry count");
        for (i, (v, r)) in parsed.iter().zip(COMMAND_REGISTRY.iter()).enumerate() {
            assert_eq!(v.name, r.name, "entry {i}");
            assert_eq!(
                v.description, r.description,
                "description mismatch at {i} ({})",
                r.name
            );
            assert_eq!(v.category, r.category, "category at {i} ({})", r.name);
            let aliases: Vec<&str> = v.aliases.iter().map(String::as_str).collect();
            assert_eq!(aliases, r.aliases, "aliases at {i} ({})", r.name);
            assert_eq!(v.cli_only, r.cli_only, "cli_only at {i} ({})", r.name);
            assert_eq!(
                v.gateway_only, r.gateway_only,
                "gateway_only at {i} ({})",
                r.name
            );
            if let Some(h) = &v.args_hint {
                assert_eq!(
                    r.args_hint,
                    Some(h.as_str()),
                    "args_hint at {i} ({})",
                    r.name
                );
            }
            match &v.subcommands {
                Some(s) => {
                    let subs: Vec<&str> = s.iter().map(String::as_str).collect();
                    assert_eq!(subs, r.subcommands, "subcommands at {i} ({})", r.name);
                }
                None => {
                    assert!(
                        v.subcommands_nonliteral,
                        "subcommands missing at {i} ({})",
                        r.name
                    );
                }
            }
            if v.args_hint.is_none() && !v.args_hint_nonliteral {
                assert!(
                    r.args_hint.is_none(),
                    "args_hint missing at {i} ({})",
                    r.name
                );
            }
        }
        // The `indicator` entry is the ONLY one with non-literal kwargs in
        // the source (INDICATOR_STYLES); pin that so a source edit cannot
        // silently change the comparison scope.
        let non_literal: Vec<&str> = parsed
            .iter()
            .filter(|v| v.args_hint_nonliteral || v.subcommands_nonliteral)
            .map(|v| v.name.as_str())
            .collect();
        assert_eq!(non_literal, vec!["indicator"]);
    }

    // ---- command / alias / picker completion -------------------------------

    #[test]
    fn completes_commands_and_aliases() {
        let c = cand_completer();
        let (_, cands) = complete(&c, "/ne", 3);
        assert!(names(&cands).contains(&"/new "), "{:?}", names(&cands));

        // alias of /new
        let (_, cands) = complete(&c, "/res", 4);
        assert!(names(&cands).contains(&"/reset "));

        // name + alias of /compress
        let (_, cands) = complete(&c, "/comp", 5);
        let ns = names(&cands);
        assert!(ns.contains(&"/compress "), "{ns:?}");
        assert!(ns.contains(&"/compact "), "{ns:?}");
    }

    #[test]
    fn trailing_space_trick_and_picker_commands() {
        let c = cand_completer();
        let (start, cands) = complete(&c, "/new", 4);
        assert_eq!(start, 0);
        assert!(names(&cands).contains(&"/new "));

        // _PICKER_COMMANDS: no trailing space (Enter opens the picker).
        for (line, pos, expected) in [
            ("/model", 6, "/model"),
            ("/personality", 12, "/personality"),
            ("/skin", 5, "/skin"),
        ] {
            let (_, cands) = complete(&c, line, pos);
            assert!(
                names(&cands).contains(&expected),
                "{line}: {:?}",
                names(&cands)
            );
            assert!(
                !names(&cands).iter().any(|n| *n == format!("{expected} ")),
                "{line} must not add a space"
            );
        }
    }

    #[test]
    fn bare_slash_lists_cli_commands_only() {
        let c = cand_completer();
        let (_, cands) = complete(&c, "/", 1);
        let ns = names(&cands);
        // shared + cli_only commands are offered...
        assert!(ns.contains(&"/new "));
        assert!(ns.contains(&"/quit "));
        // ...gateway_only commands are not (they cannot run in the CLI).
        assert!(
            !ns.iter().any(|n| matches!(*n, "/start" | "/start ")),
            "{ns:?}"
        );
        assert!(
            !ns.iter().any(|n| matches!(*n, "/pause" | "/pause ")),
            "{ns:?}"
        );
        assert!(
            !ns.iter().any(|n| matches!(*n, "/approve" | "/approve ")),
            "{ns:?}"
        );
        // extensions are offered.
        assert!(ns.contains(&"/sandbox "));
        assert!(ns.contains(&"/tool-calls "));
    }

    #[test]
    fn no_match_returns_empty() {
        let c = cand_completer();
        let (_, cands) = complete(&c, "/zzzz", 5);
        assert!(cands.is_empty());
        let (_, cands) = complete(&c, "hello world", 11);
        assert!(cands.is_empty());
    }

    // ---- subcommand completion ---------------------------------------------

    #[test]
    fn completes_subcommands() {
        let c = cand_completer();
        let (_, cands) = complete(&c, "/skills s", 9);
        assert!(names(&cands).contains(&"search "));

        let (_, cands) = complete(&c, "/reasoning h", 12);
        let ns = names(&cands);
        assert!(ns.contains(&"high "), "{ns:?}");
        assert!(ns.contains(&"hide "), "{ns:?}");

        // second token of a command without subcommands -> nothing.
        let (_, cands) = complete(&c, "/new xyz", 8);
        assert!(cands.is_empty());
    }

    // ---- skill completion ----------------------------------------------------

    fn skill_home() -> (TempDir, HermesCompleter) {
        let tmp = TempDir::new().expect("tempdir");
        let skills = tmp.path().join("skills");
        std::fs::create_dir_all(&skills).expect("mkdir skills");
        let alpha = skills.join("my-skill");
        std::fs::create_dir_all(&alpha).expect("mkdir my-skill");
        std::fs::write(
            alpha.join("SKILL.md"),
            "---\nname: my-skill\ndescription: Does great things for testing\n---\nbody\n",
        )
        .expect("write SKILL.md");
        std::fs::create_dir_all(skills.join("other_skill")).expect("mkdir other_skill");
        std::fs::create_dir_all(skills.join("third")).expect("mkdir third");
        // Non-skill entries must be ignored.
        std::fs::create_dir_all(skills.join(".curator_backups")).expect("mkdir hidden");
        std::fs::write(skills.join("stray.md"), "not a skill dir").expect("stray file");
        let c = HermesCompleter::with_home(tmp.path(), tmp.path(), tmp.path());
        (tmp, c)
    }

    #[test]
    fn discover_skills_reads_dir_and_frontmatter() {
        let (tmp, _) = skill_home();
        let skills = discover_skills(&tmp.path().join("skills"));
        let got: Vec<(&str, &str)> = skills
            .iter()
            .map(|s| (s.name.as_str(), s.description.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("my-skill", "Does great things for testing"),
                ("other_skill", ""),
                ("third", ""),
            ]
        );
    }

    #[test]
    fn completes_skills_with_bolt_display() {
        let (_tmp, c) = skill_home();
        // First-word position: the replacement keeps the leading slash.
        let (_, cands) = complete(&c, "/my-s", 5);
        let names: Vec<&str> = names(&cands);
        assert!(names.contains(&"/my-skill "), "{names:?}");
        let disp = cands
            .iter()
            .find(|p| p.replacement == "/my-skill ")
            .expect("candidate");
        assert_eq!(disp.display, "⚡ Does great things for testing");
    }

    #[test]
    fn skill_tokens_normalize_underscore_hyphen() {
        let (_tmp, c) = skill_home();
        // typing underscores still completes the hyphenated skill name.
        let (_, cands) = complete(&c, "/my_skill", 9);
        assert!(names(&cands).contains(&"/my-skill "));
        // and a stacked token typed with hyphens matches the underscore dir.
        let (_, cands) = complete(&c, "/my-skill other-sk", 20);
        assert!(names(&cands).contains(&"other_skill "));
    }

    #[test]
    fn stacked_tokens_mirrors_input_slash() {
        let (_tmp, c) = skill_home();
        // Stacked skill typed without a slash -> replacement stays bare.
        let (_, cands) = complete(&c, "/my-skill oth", 15);
        assert!(names(&cands).contains(&"other_skill "));
        // Same stacked skill typed WITH a slash keeps the slash.
        let (_, cands) = complete(&c, "/my-skill /oth", 16);
        assert!(names(&cands).contains(&"/other_skill "));
    }

    #[test]
    fn stacked_skills_offer_remaining_only() {
        let (_tmp, c) = skill_home();
        // After `/my-skill ` the cursor sits on an empty second token:
        // every other skill is offered, the used one is not.
        let (_, cands) = complete(&c, "/my-skill ", 10);
        let ns = names(&cands);
        assert!(ns.contains(&"other_skill "), "{ns:?}");
        assert!(ns.contains(&"third "), "{ns:?}");
        assert!(!ns.contains(&"my-skill "), "{ns:?}");
        // With the second skill present, only `third` remains.
        let (_, cands) = complete(&c, "/my-skill other_skill ", 22);
        assert_eq!(names(&cands), vec!["third "]);
        // A non-skill token in the middle breaks the stack.
        let (_, cands) = complete(&c, "/my-skill not-a-skill ", 22);
        assert!(cands.is_empty());
        // A built-in command first breaks the stack too.
        let (_, cands) = complete(&c, "/new my-skill ", 14);
        assert!(cands.is_empty());
    }

    // ---- path completion -----------------------------------------------------

    fn path_completer() -> (TempDir, HermesCompleter) {
        let home = TempDir::new().expect("tempdir");
        std::fs::write(home.path().join("foo.txt"), "x").expect("foo");
        std::fs::write(home.path().join("baz.md"), "x").expect("baz");
        std::fs::create_dir_all(home.path().join("bar")).expect("bar");
        std::fs::write(home.path().join("bar").join("inner.txt"), "x").expect("inner");
        std::fs::write(home.path().join("xfile"), "x").expect("xfile");
        std::fs::create_dir_all(home.path().join("sub")).expect("sub");
        std::fs::write(home.path().join(".hidden"), "x").expect("hidden");
        // empty hermes home (no skills)
        let hermes = home.path().join("hermes-home");
        std::fs::create_dir_all(&hermes).expect("hermes home");
        let c = HermesCompleter::with_home(&hermes, home.path(), home.path());
        (home, c)
    }

    #[test]
    fn path_completion_relative() {
        let (_home, c) = path_completer();
        let (_, cands) = complete(&c, "./f", 3);
        assert_eq!(names(&cands), vec!["./foo.txt"]);
        let (_, cands) = complete(&c, "./", 2);
        let ns = names(&cands);
        assert!(ns.contains(&"./bar/"), "{ns:?}");
        assert!(ns.contains(&"./baz.md"), "{ns:?}");
        assert!(ns.contains(&"./foo.txt"), "{ns:?}");
        assert!(!ns.iter().any(|n| n.contains(".hidden")), "{ns:?}");
        // inside a subdirectory (trailing slash keeps it a path)
        let (_, cands) = complete(&c, "./bar/", 6);
        assert_eq!(names(&cands), vec!["./bar/inner.txt"]);
    }

    #[test]
    fn path_completion_tilde() {
        let (_home, c) = path_completer();
        let (_, cands) = complete(&c, "~/xf", 4);
        assert_eq!(names(&cands), vec!["~/xfile"]);
        let (_, cands) = complete(&c, "~/su", 4);
        assert_eq!(names(&cands), vec!["~/sub/"]);
    }

    #[test]
    fn first_word_absolute_path_completes_when_no_command_matches() {
        let (home, c) = path_completer();
        let root = home.path().display().to_string();
        let line = format!("{root}/b");
        let (_, cands) = complete(&c, &line, line.len());
        let bar = format!("{root}/bar/");
        let baz = format!("{root}/baz.md");
        let ns = names(&cands);
        assert!(ns.contains(&bar.as_str()), "{ns:?}");
        assert!(ns.contains(&baz.as_str()), "{ns:?}");
    }

    #[test]
    fn urls_are_not_paths() {
        let (_home, c) = path_completer();
        for line in ["https://example.com/a", "http://x/y", "file://local/p"] {
            let (_, cands) = complete(&c, line, line.len());
            assert!(cands.is_empty(), "{line}: {:?}", names(&cands));
        }
        assert!(!is_path_token("https://a/b"));
        assert!(is_path_token("./a"));
        assert!(is_path_token("a/b"));
        assert!(!is_path_token("plainword"));
    }

    #[test]
    fn path_works_as_command_argument() {
        let (_home, c) = path_completer();
        // /image <path> (registry) — second token path completion.
        let (_, cands) = complete(&c, "/image ./b", 10);
        let ns = names(&cands);
        assert!(ns.contains(&"./bar/"), "{ns:?}");
        assert!(ns.contains(&"./baz.md"), "{ns:?}");
    }

    // ---- ghost text (Hinter) --------------------------------------------------

    #[test]
    fn ghost_text_unique_completion() {
        let c = cand_completer();
        assert_eq!(hint(&c, "/ne", 3), Some("w ".to_string()));
        // picker command: no trailing space in the ghost.
        assert_eq!(hint(&c, "/mod", 4), Some("el".to_string()));
        // fully typed command ghosts the trailing space.
        assert_eq!(hint(&c, "/new", 4), Some(" ".to_string()));
        assert_eq!(hint(&c, "/model", 6), None);
        // no candidates -> no ghost.
        assert_eq!(hint(&c, "hello", 5), None);
        assert_eq!(hint(&c, "/zzz", 4), None);
    }

    #[test]
    fn ghost_text_shared_prefix() {
        let c = cand_completer();
        // /to matches /tools, /toolsets and /topup — the common prefix ends
        // right at the typed text, so there is nothing to ghost.
        assert_eq!(hint(&c, "/to", 3), None);
        // Subcommand position: "high " vs "hide " share "hi".
        assert_eq!(hint(&c, "/reasoning h", 12), Some("i".to_string()));
    }

    #[test]
    fn ghost_text_subcommand_and_skill() {
        let c = cand_completer();
        assert_eq!(hint(&c, "/skills s", 9), Some("earch ".to_string()));
        let (_tmp, cs) = skill_home();
        assert_eq!(hint(&cs, "/my-sk", 6), Some("ill ".to_string()));
    }

    // ---- helpers ---------------------------------------------------------------

    #[test]
    fn truncate_caps_long_skill_descriptions() {
        let long = "x".repeat(80);
        let t = truncate(&long, 60);
        assert_eq!(t.chars().count(), 61);
        assert!(t.ends_with('…'));
        assert_eq!(truncate("short", 60), "short");
    }

    #[test]
    fn skill_desc_display_is_capped() {
        let tmp = TempDir::new().expect("tempdir");
        let skills = tmp.path().join("skills");
        let s = skills.join("long-desc");
        std::fs::create_dir_all(&s).expect("mkdir");
        std::fs::write(
            s.join("SKILL.md"),
            format!("---\ndescription: {}\n---\n", "y".repeat(80)),
        )
        .expect("write");
        let c = HermesCompleter::with_home(tmp.path(), tmp.path(), tmp.path());
        let (_, cands) = complete(&c, "/long-des", 9);
        let disp = &cands[0].display;
        assert!(disp.starts_with("⚡ "));
        assert!(disp.chars().count() <= 60 + 2 + 1, "{disp:?}");
    }
}
