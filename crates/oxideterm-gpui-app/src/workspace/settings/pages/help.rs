use super::*;

pub(in crate::workspace) const HELP_LEGAL_MARKDOWN: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../LEGAL.md"));
const HELP_THIRD_PARTY_MARKDOWN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../THIRD_PARTY_NOTICES.md"
));

pub(in crate::workspace) const HELP_LEGAL_NOTICE_WIDTH: f32 = 760.0;
pub(in crate::workspace) const HELP_LEGAL_NOTICE_HEIGHT: f32 = 720.0;

impl WorkspaceApp {
    pub(in crate::workspace) fn settings_help_section(
        &mut self,
        section_index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match section_index {
            0 => self.help_diagnostics_card(cx),
            1 => self.help_legal_card(cx),
            _ => div().into_any_element(),
        }
    }

    pub(in crate::workspace) fn help_diagnostics_card(&self, cx: &mut Context<Self>) -> AnyElement {
        // MemoryDiagnosticsPanel and the keyboard-shortcut reference are Tauri-only Help blocks.
        // GPUI keeps diagnostics lightweight: file logging is always available, while verbose
        // debug output is opt-in so normal sessions do not generate oversized logs.
        self.plain_settings_card(vec![
            self.card_title("settings_view.help.diagnostics"),
            self.bool_row(
                "settings_view.help.debug_logs",
                "settings_view.help.debug_logs_hint",
                self.settings_store.settings().diagnostics.debug_logging,
                set_diagnostics_debug_logging,
                cx,
            ),
            self.card_separator(),
            self.bool_row(
                "settings_view.terminal.show_performance_overlay",
                "settings_view.terminal.show_performance_overlay_hint",
                self.settings_store.settings().terminal.show_fps_overlay,
                set_show_terminal_performance_overlay,
                cx,
            ),
            self.card_separator(),
            self.help_action_row(
                "settings_view.help.open_logs",
                "settings_view.help.open_logs_hint",
                self.i18n.t("settings_view.help.open"),
                LucideIcon::FolderOpen,
                |this, _event, _window, cx| this.open_help_log_directory(cx),
                cx,
            ),
        ])
    }

    pub(in crate::workspace) fn help_legal_card(&self, cx: &mut Context<Self>) -> AnyElement {
        let copyright = self.i18n_with(
            "settings_view.help.copyright",
            &[
                ("year", chrono::Local::now().format("%Y").to_string()),
                ("author", "AnalyseDeCircuit".to_string()),
            ],
        );

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(4.0))
            .text_size(px(self.tokens.metrics.ui_text_xs))
            .text_color(rgb(self.tokens.ui.text_muted))
            .child(self.render_selectable_text_scoped(
                "settings-help-legal",
                "copyright",
                copyright,
                self.tokens.ui.text_muted,
                cx,
            ))
            .child(self.render_selectable_text_scoped(
                "settings-help-legal",
                "license",
                self.i18n.t("settings_view.help.license"),
                self.tokens.ui.text_muted,
                cx,
            ))
            .child(self.help_outline_button(
                self.i18n.t("settings_view.help.third_party_notices"),
                LucideIcon::BookOpen,
                |this, _event, _window, cx| {
                    this.settings_legal_notice_scroll = MarkdownVirtualListScrollHandle::new();
                    this.overlay.update(cx, |overlay, cx| {
                        overlay.open_confirm(WorkspaceOverlayConfirmKind::ThirdPartyNotices, cx);
                    });
                },
                cx,
            ))
            .into_any_element()
    }

    pub(in crate::workspace) fn help_action_row(
        &self,
        label_key: &str,
        hint_key: &str,
        button_label: String,
        icon: LucideIcon,
        listener: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.setting_row(
            label_key,
            hint_key,
            self.help_outline_button(button_label, icon, listener, cx),
            cx,
        )
    }

    pub(in crate::workspace) fn help_outline_button(
        &self,
        label: String,
        icon: LucideIcon,
        listener: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.workspace_toolbar_action_button(
            label,
            Some(if matches!(icon, LucideIcon::LoaderCircle) {
                self.render_loading_icon("help-update-checking", 14.0, rgb(self.tokens.ui.text))
            } else {
                Self::render_lucide_icon(icon, 14.0, rgb(self.tokens.ui.text))
            }),
            ToolbarButtonOptions {
                button: ButtonOptions {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    radius: ButtonRadius::Md,
                    disabled: false,
                },
                icon_position: ToolbarButtonIconPosition::Leading,
                ..ToolbarButtonOptions::default()
            },
            cx.listener(move |this, event, window, cx| {
                listener(this, event, window, cx);
                cx.stop_propagation();
            }),
        )
        .into_any_element()
    }

    pub(in crate::workspace) fn help_outline_button_with_disabled(
        &self,
        label: String,
        icon: LucideIcon,
        disabled: bool,
        listener: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.workspace_toolbar_action_button(
            label,
            Some(Self::render_lucide_icon(icon, 14.0, rgb(self.tokens.ui.text)).into_any_element()),
            ToolbarButtonOptions {
                button: ButtonOptions {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    radius: ButtonRadius::Md,
                    disabled,
                },
                icon_position: ToolbarButtonIconPosition::Leading,
                ..ToolbarButtonOptions::default()
            },
            cx.listener(move |this, event, window, cx| {
                if !disabled {
                    listener(this, event, window, cx);
                }
                cx.stop_propagation();
            }),
        )
        .into_any_element()
    }

    pub(in crate::workspace) fn open_help_legal_notice(&mut self, cx: &mut Context<Self>) {
        self.settings_legal_notice_scroll = MarkdownVirtualListScrollHandle::new();
        self.overlay.update(cx, |overlay, cx| {
            overlay.open_confirm(WorkspaceOverlayConfirmKind::LegalNotice, cx);
        });
    }

    pub(in crate::workspace) fn close_help_legal_notice(&mut self, cx: &mut Context<Self>) {
        let delay = oxideterm_gpui_ui::motion::duration(
            &self.tokens,
            oxideterm_gpui_ui::motion::MotionDuration::Overlay,
        );
        self.overlay.update(cx, |overlay, cx| {
            overlay.begin_confirm_exit(false, delay, cx);
        });
    }

    pub(in crate::workspace) fn handle_help_legal_notice_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(snapshot) = self.overlay.read(cx).confirm_snapshot() else {
            return false;
        };
        if !matches!(
            snapshot.kind,
            WorkspaceOverlayConfirmKind::LegalNotice
                | WorkspaceOverlayConfirmKind::ThirdPartyNotices
        ) {
            return false;
        }
        if snapshot.phase == oxideterm_gpui_ui::motion::ExitPhase::Exiting {
            return true;
        }
        let key_action = self.overlay.update(cx, |overlay, cx| {
            overlay.handle_confirm_key(
                event.keystroke.key.as_str(),
                event.keystroke.modifiers.shift,
                event.keystroke.modifiers.platform || event.keystroke.modifiers.control,
                cx,
            )
        });
        match key_action {
            Some(
                WorkspaceOverlayConfirmKeyAction::Cancel
                | WorkspaceOverlayConfirmKeyAction::Confirm,
            ) => {
                self.close_help_legal_notice(cx);
                true
            }
            Some(WorkspaceOverlayConfirmKeyAction::Handled) => true,
            None => false,
        }
    }

    pub(in crate::workspace) fn render_help_legal_notice_dialog(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(snapshot) = self.overlay.read(cx).confirm_snapshot() else {
            return div().into_any_element();
        };
        if !matches!(
            snapshot.kind,
            WorkspaceOverlayConfirmKind::LegalNotice
                | WorkspaceOverlayConfirmKind::ThirdPartyNotices
        ) {
            return div().into_any_element();
        }
        let third_party = matches!(
            snapshot.kind,
            WorkspaceOverlayConfirmKind::ThirdPartyNotices
        );
        let mut options = self.localized_markdown_options();
        options.base_font_size = self.tokens.metrics.ui_text_sm;
        options.block_gap = 8.0;
        let code_actions = self.markdown_mermaid_actions(cx);

        let backdrop = dismissible_dialog_backdrop().on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _event, _window, cx| {
                this.close_help_legal_notice(cx);
                cx.stop_propagation();
            }),
        );
        let form = overlay_content_boundary(
            dialog_content(&self.tokens)
                .flex()
                .flex_col()
                .w(px(HELP_LEGAL_NOTICE_WIDTH))
                .max_w(relative(0.92))
                .h(px(HELP_LEGAL_NOTICE_HEIGHT))
                .max_h(relative(0.90))
                .child(
                    dialog_header(&self.tokens)
                        .child(dialog_title(
                            &self.tokens,
                            self.i18n.t(if third_party {
                                "settings_view.help.third_party_notices"
                            } else {
                                "settings_view.help.disclaimer"
                            }),
                        ))
                        .child(dialog_description(
                            &self.tokens,
                            self.i18n.t(if third_party {
                                "settings_view.help.third_party_notices_description"
                            } else {
                                "settings_view.help.legal_notice_description"
                            }),
                        )),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h(px(0.0))
                        .p(px(16.0))
                        .bg(rgb(self.tokens.ui.bg))
                        .text_color(rgb(self.tokens.ui.text))
                        .child(markdown_virtual_with_code_actions(
                            "settings-help-legal-notice-markdown",
                            &self.tokens,
                            if third_party {
                                HELP_THIRD_PARTY_MARKDOWN
                            } else {
                                HELP_LEGAL_MARKDOWN
                            },
                            &options,
                            &self.settings_legal_notice_scroll,
                            &code_actions,
                        )),
                )
                .child(
                    dialog_footer(&self.tokens).child(self.standard_footer_action_button(
                        self.i18n.t("settings_view.help.legal_notice_close"),
                        ButtonVariant::Secondary,
                        ConfirmDialogAction::Cancel,
                        false,
                        |this, _event, _window, cx| {
                            this.close_help_legal_notice(cx);
                        },
                        cx,
                    )),
                ),
        );
        settings_dialog_transition(
            &self.tokens,
            "help-legal-notice-form",
            backdrop,
            form,
            snapshot.phase,
        )
    }

    pub(in crate::workspace) fn open_help_log_directory(&mut self, cx: &mut Context<Self>) {
        let log_dir = self.help_log_directory();
        let opened = std::fs::create_dir_all(&log_dir)
            .and_then(|()| open_path_external(&log_dir))
            .map_err(|error| error.to_string());
        if let Err(error) = opened {
            self.push_ai_settings_toast(error, TerminalNoticeVariant::Error, cx);
            cx.notify();
        }
    }

    pub(in crate::workspace) fn help_log_directory(&self) -> std::path::PathBuf {
        // Tauri stores logs under the app data directory. Native settings use
        // the same data root, so derive logs beside settings.json.
        self.settings_store
            .path()
            .parent()
            .map(|parent| parent.join("logs"))
            .unwrap_or_else(|| std::path::PathBuf::from("logs"))
    }

    pub(in crate::workspace) fn language_label(&self, language: Language) -> String {
        match language {
            Language::De => "Deutsch",
            Language::En => "English",
            Language::EsEs => "Español (España)",
            Language::FrFr => "Français (France)",
            Language::It => "Italiano",
            Language::Ko => "한국어",
            Language::PtBr => "Português (Brasil)",
            Language::Vi => "Tiếng Việt",
            Language::Ja => "日本語",
            Language::ZhCn => "简体中文",
            Language::ZhTw => "繁體中文",
        }
        .to_string()
    }
}
