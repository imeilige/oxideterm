// Compact Rich Input consumes history here; the remaining providers still back settings data.
#![allow(dead_code, unused_imports)]

use super::*;
use oxideterm_sftp::{FileType as RemotePathFileType, ListFilter, SortOrder};

mod common;
mod engine;
mod fig_provider;
mod fig_specs;
mod path_provider;
mod types;

pub(self) use common::{
    infer_terminal_ssh_identity_from_buffer, normalize_terminal_command_suggestions,
    terminal_command_risk_score_penalty, terminal_cwd_looks_remote,
};
pub(self) use fig_provider::{active_fig_arg_type, terminal_command_fig_suggestions};
pub(self) use fig_specs::{normalize_terminal_path_token, should_run_terminal_path_provider};
pub(self) use oxideterm_terminal::{TerminalShellParseResult, TerminalShellToken};
pub(self) use oxideterm_terminal::{
    escape_terminal_path_for_shell, load_local_shell_history_commands,
    normalize_terminal_autosuggest_command, terminal_autosuggest_fuzzy_score,
    tokenize_terminal_command_line,
};
pub(self) use types::{
    TerminalCommandContext, TerminalCommandContextType, TerminalFigArgType, TerminalFigOptionSpec,
    TerminalFigSpec, TerminalFigSubcommandSpec, TerminalPathCacheEntry, TerminalPathCompletionCache,
    TerminalPathEntry, TerminalPathParts,
};

// Preserve the settings UI path while keeping the implementation module private.
pub(in crate::workspace) use fig_specs::{
    built_in_terminal_fig_specs, normalize_terminal_command_specs_json,
    terminal_command_specs_editor_initial_json, terminal_command_specs_example_json,
    terminal_command_specs_path, user_terminal_fig_specs_count,
};
