// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

//! GPUI adapters for pure settings model types.
//!
//! The page model enums live in `oxideterm-settings-model`; this module keeps
//! only view-layer mapping to GPUI anchors.

use oxideterm_gpui_ui::select::SelectAnchorId;
pub use oxideterm_settings_model::{
    SettingsInput, SettingsKeybindingScopeFilter, SettingsSelect, SettingsSlider, SettingsTab,
    SettingsTabIcon, TerminalSettingsPage,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveSurface {
    Terminal,
    Settings,
}

pub trait SettingsSelectAnchorExt {
    fn anchor_id(self) -> SelectAnchorId;
}

impl SettingsSelectAnchorExt for SettingsSelect {
    fn anchor_id(self) -> SelectAnchorId {
        match self {
            Self::Language => SelectAnchorId::SettingsLanguage,
            Self::AppearanceTheme => SelectAnchorId::SettingsAppearanceTheme,
            Self::AppearanceDensity => SelectAnchorId::SettingsAppearanceDensity,
            Self::AppearanceAnimation => SelectAnchorId::SettingsAppearanceAnimation,
            Self::AppearanceRenderProfile => SelectAnchorId::SettingsAppearanceRenderProfile,
            Self::AppearanceFrostedGlass => SelectAnchorId::SettingsAppearanceFrostedGlass,
            Self::CustomThemeDuplicate => SelectAnchorId::SettingsCustomThemeDuplicate,
            Self::IdeFontFamily => SelectAnchorId::SettingsIdeFontFamily,
            Self::IdeCjkFontFamily => SelectAnchorId::SettingsIdeCjkFontFamily,
            Self::TerminalFontFamily => SelectAnchorId::SettingsTerminalFontFamily,
            Self::TerminalEncoding => SelectAnchorId::SettingsTerminalEncoding,
            Self::TerminalBackspaceSequence => SelectAnchorId::SettingsTerminalBackspaceSequence,
            Self::TerminalDeleteSequence => SelectAnchorId::SettingsTerminalDeleteSequence,
            Self::TerminalCursorStyle => SelectAnchorId::SettingsTerminalCursorStyle,
            Self::RemoteShellIntegrationMode => SelectAnchorId::SettingsRemoteShellIntegrationMode,
            Self::TerminalTriggerMatchMode => SelectAnchorId::SettingsTerminalTriggerMatchMode,
            Self::TerminalTriggerAction => SelectAnchorId::SettingsTerminalTriggerAction,
            Self::TerminalTriggerProcessMode => SelectAnchorId::SettingsTerminalTriggerProcessMode,
            Self::TerminalTriggerQuickCommand => {
                SelectAnchorId::SettingsTerminalTriggerQuickCommand
            }
            Self::TerminalTriggerTiming => SelectAnchorId::SettingsTerminalTriggerTiming,
            Self::TerminalTriggerScope => SelectAnchorId::SettingsTerminalTriggerScope,
            Self::IdeAgentMode => SelectAnchorId::SettingsIdeAgentMode,
            Self::LocalShell => SelectAnchorId::SettingsLocalShell,
            Self::LocalShellSemanticScheme(index) => {
                SelectAnchorId::SettingsLocalShellSemanticScheme(index)
            }
            Self::ConnectionIdleTimeout => SelectAnchorId::SettingsConnectionIdleTimeout,
            Self::ReconnectMaxAttempts => SelectAnchorId::SettingsReconnectMaxAttempts,
            Self::ReconnectBaseDelay => SelectAnchorId::SettingsReconnectBaseDelay,
            Self::ReconnectMaxDelay => SelectAnchorId::SettingsReconnectMaxDelay,
            Self::SftpPresentation => SelectAnchorId::SettingsSftpPresentation,
            Self::SftpProtocol => SelectAnchorId::SettingsSftpProtocol,
            Self::SftpConcurrent => SelectAnchorId::SettingsSftpConcurrent,
            Self::SftpDirectoryParallelism => SelectAnchorId::SettingsSftpDirectoryParallelism,
            Self::SftpConflict => SelectAnchorId::SettingsSftpConflict,
            Self::TerminalSemanticScheme => SelectAnchorId::SettingsTerminalSemanticScheme,
            Self::SemanticSchemeRuleClass(index) => {
                SelectAnchorId::SettingsSemanticSchemeRuleClass(index)
            }
            Self::SemanticSchemeRuleContext(index) => {
                SelectAnchorId::SettingsSemanticSchemeRuleContext(index)
            }
            Self::HighlightRuleSet => SelectAnchorId::SettingsHighlightRuleSet,
            Self::HighlightPreset => SelectAnchorId::SettingsHighlightPreset,
            Self::HighlightRenderMode(index) => SelectAnchorId::SettingsHighlightRenderMode(index),
            Self::HighlightMatchScope(index) => SelectAnchorId::SettingsHighlightMatchScope(index),
        }
    }
}
