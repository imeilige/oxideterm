use super::*;

mod constants;
mod help;
mod helpers;
mod keybindings;
mod local_reconnect;

use constants::{
    SETTINGS_RECONNECT_FIELD_BASIS,
    SETTINGS_RECONNECT_HINT_LINE_HEIGHT,
};
pub(in crate::workspace) use helpers::open_path_external;
pub(in crate::workspace) use keybindings::settings_keybinding_scope_matches;

impl WorkspaceApp {
    pub(in crate::workspace) fn handle_settings_workspace_event(
        &mut self,
        settings: Entity<SettingsWorkspaceEntity>,
        event: &SettingsWorkspaceEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            SettingsWorkspaceEvent::ExternalStoresChanged => {
                // The Entity owns change detection; Workspace applies the
                // cross-system settings and connection-store side effects.
                let _ = self.reload_after_external_sync(cx);
            }
            SettingsWorkspaceEvent::DataDirectoryConfirmOpened => {
                self.reset_standard_confirm_focus();
                cx.notify();
            }
            SettingsWorkspaceEvent::DataDirectoryOperationReady => {
                let results =
                    settings.update(cx, |settings, _cx| settings.take_data_directory_results());
                for result in results {
                    match result {
                        DataDirectoryOperationResult::Changed => {
                            self.push_ai_settings_toast(
                                self.i18n.t("settings_view.general.data_directory_changed"),
                                TerminalNoticeVariant::Success,
                                cx,
                            );
                        }
                        DataDirectoryOperationResult::Reset => {
                            self.push_ai_settings_toast(
                                self.i18n.t("settings_view.general.data_directory_reset"),
                                TerminalNoticeVariant::Success,
                                cx,
                            );
                        }
                        DataDirectoryOperationResult::Failed(error) => {
                            self.push_ai_settings_toast(error, TerminalNoticeVariant::Error, cx);
                        }
                    }
                }
                cx.notify();
            }
            SettingsWorkspaceEvent::ThemeImportReady => {
                let results =
                    settings.update(cx, |settings, _cx| settings.take_theme_import_results());
                for result in results {
                    match result {
                        ThemeImportResult::Imported {
                            theme_id,
                            name,
                            value,
                        } => {
                            // The identifier is persisted both as the map key
                            // and as the active theme, requiring two owners.
                            let selected_theme_id = theme_id.clone();
                            self.edit_settings(
                                move |settings| {
                                    settings.custom_themes.insert(theme_id, value);
                                    settings.terminal.theme = selected_theme_id;
                                },
                                cx,
                            );
                            self.send_settings_notice(
                                self.i18n
                                    .t("settings_view.appearance.theme_import_success")
                                    .replace("{{name}}", &name),
                                TerminalNoticeVariant::Success,
                                cx,
                            );
                        }
                        ThemeImportResult::Failed(error) => {
                            self.send_settings_notice(
                                self.i18n
                                    .t("settings_view.appearance.theme_import_error")
                                    .replace("{{error}}", &error),
                                TerminalNoticeVariant::Error,
                                cx,
                            );
                        }
                    }
                }
                cx.notify();
            }
            SettingsWorkspaceEvent::ThemeEditorOperationReady => {
                let results =
                    settings.update(cx, |settings, _cx| settings.take_theme_editor_results());
                for result in results {
                    match result {
                        ThemeEditorOperationResult::Save(editor) => {
                            let mut saved_name = None;
                            self.edit_settings(
                                |settings| {
                                    saved_name =
                                        save_theme_editor_snapshot_to_settings(settings, &editor);
                                },
                                cx,
                            );
                            if let Some(name) = saved_name {
                                self.send_settings_notice(
                                    self.i18n
                                        .t("settings_view.appearance.theme_import_success")
                                        .replace("{{name}}", &name),
                                    TerminalNoticeVariant::Success,
                                    cx,
                                );
                            }
                        }
                        ThemeEditorOperationResult::Delete(editor) => {
                            self.edit_settings(
                                move |settings| {
                                    if let Some(theme_id) = editor.edit_theme_id.as_deref() {
                                        delete_custom_theme_from_settings(
                                            settings,
                                            theme_id,
                                            oxideterm_theme::DEFAULT_THEME.id,
                                        );
                                    }
                                },
                                cx,
                            );
                        }
                    }
                }
                cx.notify();
            }
            SettingsWorkspaceEvent::KeybindingFileOperationReady => {
                let results = settings.update(cx, |settings, _cx| {
                    settings.take_keybinding_file_operation_results()
                });
                for result in results {
                    match result {
                        KeybindingFileOperationResult::Exported => {
                            self.push_ai_settings_toast(
                                self.i18n.t("settings_view.keybindings.export_success"),
                                TerminalNoticeVariant::Success,
                                cx,
                            );
                        }
                        KeybindingFileOperationResult::ExportFailed => {
                            self.push_ai_settings_toast(
                                self.i18n.t("settings_view.keybindings.export_error"),
                                TerminalNoticeVariant::Error,
                                cx,
                            );
                        }
                        KeybindingFileOperationResult::Imported {
                            overrides: next_overrides,
                        } => {
                            let side = crate::keybindings::KeybindingSide::current();
                            let runtime_bindings = {
                                let previous_overrides =
                                    &self.settings_store.settings().keybindings.overrides;
                                crate::keybindings::ACTION_DEFINITIONS
                                    .iter()
                                    .flat_map(|definition| {
                                        let previous = crate::keybindings::effective_combo(
                                            definition,
                                            previous_overrides,
                                            side,
                                        );
                                        let next = crate::keybindings::effective_combo(
                                            definition,
                                            &next_overrides,
                                            side,
                                        );
                                        crate::keybindings::runtime_rebind_key_bindings(
                                            &definition.id,
                                            previous.as_ref(),
                                            next.as_ref(),
                                        )
                                    })
                                    .collect::<Vec<_>>()
                            };
                            self.edit_settings(
                                move |settings| {
                                    settings.keybindings.overrides = next_overrides;
                                },
                                cx,
                            );
                            Self::apply_runtime_key_bindings(runtime_bindings, cx);
                            self.push_ai_settings_toast(
                                self.i18n.t("settings_view.keybindings.import_success"),
                                TerminalNoticeVariant::Success,
                                cx,
                            );
                        }
                        KeybindingFileOperationResult::ImportFailed => {
                            self.push_ai_settings_toast(
                                self.i18n.t("settings_view.keybindings.import_invalid"),
                                TerminalNoticeVariant::Error,
                                cx,
                            );
                        }
                    }
                }
                cx.notify();
            }
            SettingsWorkspaceEvent::CliCompanionFinished { operation, success } => {
                if *operation == CliCompanionOperation::Refresh {
                    return;
                }
                if *success {
                    let message_key = match operation {
                        CliCompanionOperation::Install => "settings_view.general.cli_installed",
                        CliCompanionOperation::Uninstall => "settings_view.general.cli_uninstalled",
                        CliCompanionOperation::UninstallLegacy => {
                            "migration.cli_legacy_uninstalled"
                        }
                        CliCompanionOperation::Migrate => "migration.cli_migrated",
                        CliCompanionOperation::Refresh => unreachable!("handled above"),
                    };
                    self.push_ai_settings_toast(
                        self.i18n.t(message_key),
                        TerminalNoticeVariant::Success,
                        cx,
                    );
                } else if let Some(error) =
                    settings.read(cx).cli_companion_error().map(str::to_owned)
                {
                    self.push_ai_settings_toast(error, TerminalNoticeVariant::Error, cx);
                }
            }
        }
    }
}
