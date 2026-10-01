mod app;
mod command_facts;
mod image_budget;
mod modem_worker;
mod privilege_prompt;
pub mod terminal_ui;
mod terminal_view;
mod trzsz_worker;

#[cfg(feature = "bench")]
pub use app::TerminalPlaybackUpdateTimings;
pub use app::{
    SharedTerminalSession, TerminalBroadcastInputKind, TerminalContextAction, TerminalCursorAnchor,
    TerminalCwdShellIntegrationStatus, TerminalInputBroadcaster, TerminalInputInterceptor,
    TerminalInputInterceptorResult, TerminalKeybindings, TerminalPane, TerminalPaneEvent,
    TerminalSearchStatus, TerminalSerialAction, TerminalSerialStatus, TerminalShortcut,
    TerminalTelnetAction, TerminalWorkingDirectorySource,
};
pub use command_facts::{
    SharedTerminalCommandHistory, TerminalAiCommandRecord, TerminalAutosuggestCommandRecord,
    TerminalAutosuggestInputState, TerminalCommandFact, TerminalCommandFactStatus,
};
pub use oxideterm_terminal::TerminalOutputProcessor;
pub use oxideterm_terminal_recording::{TerminalRecordingState, TerminalRecordingStatus};
pub use oxideterm_terminal_semantic::SemanticShellDialect;
pub use privilege_prompt::{
    PrivilegePromptConfidence, PrivilegePromptMatch, PrivilegePromptSnapshot,
    detect_custom_privilege_prompt, detect_privilege_prompt,
};
pub use terminal_ui::{
    TerminalAutosuggestLabels, TerminalCommandSelectionLabels, TerminalHighlightMatchScope,
    TerminalHighlightRenderMode, TerminalHighlightRule, TerminalHighlightRuleSetOverride,
    TerminalKittyFileTransmissionLabels, TerminalModemLabels, TerminalNotice,
    TerminalNoticeVariant, TerminalPasteLabels, TerminalSerialControlLabels, TerminalTmuxLabels,
    TerminalTrzszLabels, TerminalUiPreferenceOverrides, TerminalUiPreferences, TerminalUiTheme,
    resolved_terminal_semantic_scheme, terminal_semantic_color, terminal_semantic_line_band,
    terminal_semantic_variant_color,
};
