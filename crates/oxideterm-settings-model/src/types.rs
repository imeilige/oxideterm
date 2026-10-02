// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

//! Pure settings page identity types.
//!
//! These enums describe settings navigation, editable fields, selects, and
//! sliders without depending on GPUI. View crates can map them to anchors and
//! controls, while app code can use the same model keys for focus and drafts.

const PLUGIN_MANAGER_INPUT_ANCHOR_BASE: u64 = 28_000;
const PLUGIN_SETTING_INPUT_ANCHOR_BASE: u64 = 29_000;
const SETTINGS_SEARCH_INPUT_ANCHOR_KEY: u64 = 34_000;
const DEFAULT_SETTINGS_TEXTAREA_LINE_HEIGHT: f32 = 20.0;

/// Describes the installed CLI companion relative to the bundled executable.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CliCompanionStatus {
    pub bundled: bool,
    pub installed: bool,
    pub install_path: Option<String>,
    pub legacy_installed: bool,
    pub legacy_install_path: Option<String>,
    pub bundle_path: Option<String>,
    pub app_version: String,
    pub matches_bundled: Option<bool>,
    pub needs_reinstall: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SettingsTab {
    General,
    Terminal,
    Appearance,
    Connections,
    Sftp,
    Keybindings,
    Help,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalSettingsPage {
    Display,
    Input,
    Local,
    CommandBar,
    Awareness,
    Transfer,
    Highlight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsKeybindingScopeFilter {
    All,
    Global,
    Terminal,
    Split,
    Palette,
    Editor,
    Sftp,
    Files,
    Preview,
    RemoteDesktop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsSelect {
    Language,
    AppearanceTheme,
    AppearanceDensity,
    AppearanceAnimation,
    AppearanceRenderProfile,
    AppearanceFrostedGlass,
    CustomThemeDuplicate,
    IdeFontFamily,
    IdeCjkFontFamily,
    TerminalFontFamily,
    TerminalEncoding,
    TerminalBackspaceSequence,
    TerminalDeleteSequence,
    TerminalCursorStyle,
    RemoteShellIntegrationMode,
    TerminalTriggerMatchMode,
    TerminalTriggerAction,
    TerminalTriggerProcessMode,
    TerminalTriggerQuickCommand,
    TerminalTriggerTiming,
    TerminalTriggerScope,
    IdeAgentMode,
    LocalShell,
    LocalShellSemanticScheme(usize),
    ConnectionIdleTimeout,
    ReconnectMaxAttempts,
    ReconnectBaseDelay,
    ReconnectMaxDelay,
    SftpPresentation,
    SftpProtocol,
    SftpConcurrent,
    SftpDirectoryParallelism,
    SftpConflict,
    TerminalSemanticScheme,
    SemanticSchemeRuleClass(usize),
    SemanticSchemeRuleContext(usize),
    HighlightRuleSet,
    HighlightPreset,
    HighlightRenderMode(usize),
    HighlightMatchScope(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SettingsInput {
    SettingsSearch,
    TerminalCustomFontFamily,
    TerminalCjkFontFamily,
    TerminalFontSize,
    TerminalFontWeight,
    TerminalScrollback,
    TerminalLineHeight,
    TerminalPaddingHorizontal,
    TerminalPaddingVertical,
    IdeFontWeight,
    IdeCustomFontFamily,
    IdeCjkFontFamily,
    IdeFontSize,
    IdeLineHeight,
    AppearanceUiFont,
    LocalDefaultCwd,
    LocalGitBashPath,
    LocalOhMyPoshTheme,
    ConnectionDefaultUsername,
    ConnectionDefaultPort,
    SftpSpeedLimitKbps,
    InBandTransferMaxChunkBytes,
    InBandTransferMaxFileCount,
    InBandTransferMaxTotalBytes,
    TerminalCommandBarFocusHandoff,
    TerminalCommandSpecsJson,
    TerminalTriggerName,
    TerminalTriggerDescription,
    TerminalTriggerPattern,
    TerminalTriggerActionValue,
    TerminalTriggerExecutable,
    TerminalTriggerArguments,
    TerminalTriggerWorkingDirectory,
    TerminalTriggerDelayMs,
    TerminalTriggerCooldownMs,
    TerminalTriggerConnectionSearch,
    KeybindingSearch,
    CustomThemeName,
    CustomThemeTerminalColor(usize),
    CustomThemeUiColor(usize),
    SemanticSchemeName,
    SemanticSchemeRulePattern(usize),
    SemanticSchemeRuleCapture(usize),
    SemanticSchemeColor(usize),
    HighlightRuleSetName,
    HighlightLabel(usize),
    HighlightPattern(usize),
    HighlightForeground(usize),
    HighlightBackground(usize),
    NativePluginInstallUrl,
    NativePluginInstallChecksum,
    NativePluginRegistryUrl,
    NativePluginMarketplaceSearch,
    ManagedKeyFilePath,
    ManagedKeyFileName,
    ManagedKeyFilePassphrase,
    ManagedKeyPasteName,
    ManagedKeyPastePrivateKey,
    ManagedKeyPastePassphrase,
    ManagedKeyRenameName,
    PluginSetting(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsSlider {
    TerminalFontSize,
    AppearanceUiFontSize,
    AppearanceBorderRadius,
    OnboardingBorderRadius,
    VersionMigrationBorderRadius,
    AppearanceWindowOpacity,
}

impl TerminalSettingsPage {
    pub fn all() -> &'static [Self] {
        &[
            Self::Display,
            Self::Input,
            Self::Local,
            Self::CommandBar,
            Self::Awareness,
            Self::Transfer,
            Self::Highlight,
        ]
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Display => "settings_view.terminal.page_display",
            Self::Input => "settings_view.terminal.page_input",
            Self::Local => "settings_view.terminal.page_local",
            Self::CommandBar => "settings_view.terminal.page_commandBar",
            Self::Awareness => "settings_view.terminal.page_awareness",
            Self::Transfer => "settings_view.terminal.page_transfer",
            Self::Highlight => "settings_view.terminal.page_highlight",
        }
    }
}

impl SettingsKeybindingScopeFilter {
    pub fn all() -> &'static [Self] {
        &[
            Self::All,
            Self::Global,
            Self::Terminal,
            Self::Split,
            Self::Palette,
            Self::Editor,
            Self::Sftp,
            Self::Files,
            Self::Preview,
            Self::RemoteDesktop,
        ]
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::All => "settings_view.keybindings.scope_all",
            Self::Global => "settings_view.keybindings.scope_global",
            Self::Terminal => "settings_view.keybindings.scope_terminal",
            Self::Split => "settings_view.keybindings.scope_split",
            Self::Palette => "settings_view.keybindings.scope_palette",
            Self::Editor => "settings_view.keybindings.scope_editor",
            Self::Sftp => "settings_view.keybindings.scope_sftp",
            Self::Files => "settings_view.keybindings.scope_files",
            Self::Preview => "settings_view.keybindings.scope_preview",
            Self::RemoteDesktop => "settings_view.keybindings.scope_remote_desktop",
        }
    }
}

impl SettingsTab {
    pub fn all() -> &'static [Self] {
        &[
            Self::General,
            Self::Appearance,
            Self::Keybindings,
            Self::Terminal,
            Self::Connections,
            Self::Sftp,
            Self::Help,
        ]
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Terminal => "terminal",
            Self::Appearance => "appearance",
            Self::Connections => "connections",
            Self::Sftp => "sftp",
            Self::Keybindings => "keybindings",
            Self::Help => "help",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::all().iter().copied().find(|tab| tab.id() == id)
    }

    pub fn groups() -> &'static [&'static [Self]] {
        // Keep navigation groups aligned with user tasks: application preferences,
        // runtime behavior, remote access, productivity, then support.
        &[
            &[Self::General, Self::Appearance, Self::Keybindings],
            &[Self::Terminal],
            &[Self::Connections, Self::Sftp],
            &[Self::Help],
        ]
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::General => "settings.general.title",
            Self::Terminal => "settings.terminal.title",
            Self::Appearance => "settings_view.tabs.appearance",
            Self::Connections => "settings_view.connections.keys_and_connections_title",
            Self::Sftp => "settings_view.tabs.sftp",
            Self::Keybindings => "settings_view.tabs.keybindings",
            Self::Help => "settings_view.tabs.help",
        }
    }

    pub fn title_key(self) -> &'static str {
        match self {
            Self::General => "settings_view.general.title",
            Self::Terminal => "settings_view.terminal.title",
            Self::Appearance => "settings_view.appearance.title",
            Self::Connections => "settings_view.connections.keys_and_connections_title",
            Self::Sftp => "settings_view.sftp.title",
            Self::Keybindings => "settings_view.keybindings.title",
            Self::Help => "settings_view.help.title",
        }
    }

    pub fn description_key(self) -> &'static str {
        match self {
            Self::General => "settings_view.general.description",
            Self::Terminal => "settings_view.terminal.description",
            Self::Appearance => "settings_view.appearance.description",
            Self::Connections => "settings_view.connections.keys_and_connections_description",
            Self::Sftp => "settings_view.sftp.description",
            Self::Keybindings => "settings_view.keybindings.description",
            Self::Help => "settings_view.help.description",
        }
    }

    pub fn icon(self) -> SettingsTabIcon {
        match self {
            Self::General | Self::Appearance => SettingsTabIcon::Monitor,
            Self::Sftp => SettingsTabIcon::HardDrive,
            Self::Terminal => SettingsTabIcon::Terminal,
            Self::Connections => SettingsTabIcon::Shield,
            Self::Keybindings => SettingsTabIcon::Keyboard,
            Self::Help => SettingsTabIcon::HelpCircle,
        }
    }
}

impl SettingsInput {
    pub fn accepts_newline(self) -> bool {
        // Keep multiline behavior beside the input identity so IME handling and
        // render controls cannot drift when new settings fields are added.
        matches!(
            self,
            Self::TerminalCommandBarFocusHandoff
                | Self::TerminalCommandSpecsJson
                | Self::TerminalTriggerArguments
                | Self::ManagedKeyPastePrivateKey
        )
    }

    pub fn textarea_line_height(self) -> f32 {
        // These values describe settings text areas in logical pixels; GPUI
        // converts them to concrete units at the view boundary.
        match self {
            Self::TerminalCommandBarFocusHandoff | Self::TerminalCommandSpecsJson => 20.0,
            Self::TerminalTriggerArguments => 20.0,
            Self::ManagedKeyPastePrivateKey => 20.0,
            _ => DEFAULT_SETTINGS_TEXTAREA_LINE_HEIGHT,
        }
    }

    pub fn anchor_key(self) -> u64 {
        match self {
            Self::SettingsSearch => SETTINGS_SEARCH_INPUT_ANCHOR_KEY,
            Self::TerminalCustomFontFamily => 19,
            Self::TerminalFontSize => 1,
            Self::TerminalFontWeight => 21,
            Self::TerminalScrollback => 33_000,
            Self::TerminalLineHeight => 2,
            Self::TerminalPaddingHorizontal => 22,
            Self::TerminalPaddingVertical => 23,
            Self::IdeFontWeight => 34_010,
            Self::IdeCustomFontFamily => 34_011,
            Self::IdeCjkFontFamily => 34_012,
            Self::IdeFontSize => 3,
            Self::IdeLineHeight => 4,
            Self::AppearanceUiFont => 5,
            Self::LocalDefaultCwd => 6,
            Self::LocalGitBashPath => 7,
            Self::LocalOhMyPoshTheme => 8,
            Self::ConnectionDefaultUsername => 9,
            Self::ConnectionDefaultPort => 10,
            Self::SftpSpeedLimitKbps => 12,
            Self::InBandTransferMaxChunkBytes => 13,
            Self::InBandTransferMaxFileCount => 14,
            Self::InBandTransferMaxTotalBytes => 15,
            Self::TerminalCommandBarFocusHandoff => 16,
            Self::TerminalCommandSpecsJson => 17,
            Self::TerminalTriggerName => 33_100,
            Self::TerminalTriggerDescription => 33_101,
            Self::TerminalTriggerPattern => 33_102,
            Self::TerminalTriggerActionValue => 33_103,
            Self::TerminalTriggerExecutable => 33_104,
            Self::TerminalTriggerArguments => 33_105,
            Self::TerminalTriggerWorkingDirectory => 33_106,
            Self::TerminalTriggerDelayMs => 33_107,
            Self::TerminalTriggerCooldownMs => 33_108,
            Self::TerminalTriggerConnectionSearch => 33_109,
            Self::KeybindingSearch => 18,
            Self::CustomThemeName => 10_000,
            Self::CustomThemeTerminalColor(index) => 10_100 + index as u64,
            Self::CustomThemeUiColor(index) => 10_200 + index as u64,
            Self::SemanticSchemeName => 10_300,
            Self::SemanticSchemeRulePattern(index) => 10_400 + index as u64,
            Self::SemanticSchemeRuleCapture(index) => 10_500 + index as u64,
            Self::SemanticSchemeColor(index) => 10_600 + index as u64,
            Self::HighlightRuleSetName => 10_700,
            Self::HighlightLabel(index) => 100 + index as u64 * 4,
            Self::HighlightPattern(index) => 101 + index as u64 * 4,
            Self::HighlightForeground(index) => 102 + index as u64 * 4,
            Self::HighlightBackground(index) => 103 + index as u64 * 4,
            Self::NativePluginInstallUrl => PLUGIN_MANAGER_INPUT_ANCHOR_BASE,
            Self::NativePluginInstallChecksum => PLUGIN_MANAGER_INPUT_ANCHOR_BASE + 1,
            Self::NativePluginRegistryUrl => PLUGIN_MANAGER_INPUT_ANCHOR_BASE + 2,
            Self::NativePluginMarketplaceSearch => PLUGIN_MANAGER_INPUT_ANCHOR_BASE + 3,
            Self::ManagedKeyFilePath => 30_000,
            Self::ManagedKeyFileName => 30_001,
            Self::ManagedKeyFilePassphrase => 30_002,
            Self::ManagedKeyPasteName => 30_003,
            Self::ManagedKeyPastePrivateKey => 30_004,
            Self::ManagedKeyPastePassphrase => 30_005,
            Self::ManagedKeyRenameName => 30_006,
            Self::PluginSetting(index) => PLUGIN_SETTING_INPUT_ANCHOR_BASE + index as u64,
            _ => 0,
        }
    }

    pub fn is_secret(self) -> bool {
        matches!(
            self,
            Self::ManagedKeyFilePassphrase
                | Self::ManagedKeyPastePrivateKey
                | Self::ManagedKeyPastePassphrase
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsTabIcon {
    HardDrive,
    HelpCircle,
    Keyboard,
    Monitor,
    Network,
    Shield,
    Square,
    Terminal,
    WifiOff,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_inputs_are_categorized_in_the_model_layer() {
        assert!(SettingsInput::ManagedKeyFilePassphrase.is_secret());
        assert!(!SettingsInput::TerminalFontSize.is_secret());
    }
}
