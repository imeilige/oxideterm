use super::*;

pub(in crate::workspace) const AI_MCP_DIALOG_WIDTH: f32 = 672.0; // Tauri DialogContent sm:max-w-2xl.
pub(in crate::workspace) const AI_MCP_DIALOG_CONTENT_PX: f32 = 16.0; // Tauri px-4.
pub(in crate::workspace) const AI_MCP_DIALOG_CONTENT_PY: f32 = 8.0; // Tauri py-2.
pub(in crate::workspace) const AI_MCP_FORM_GAP: f32 = 16.0; // Tauri space-y-4.
pub(in crate::workspace) const AI_MCP_FIELD_GAP: f32 = 8.0; // Tauri space-y-2 / gap-2.
pub(in crate::workspace) const AI_MCP_ARGS_TEXTAREA_MIN_H: f32 = 84.0; // Tauri textarea-sized MCP args field.

impl WorkspaceApp {
    pub(in crate::workspace) fn render_ai_mcp_add_server_dialog(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let (can_add, transport, auth_header_mode, dialog_presence) = {
            let ai_workspace = self.ai_entity.read(cx);
            if !ai_workspace.mcp_dialog_is_open() {
                return None;
            }
            (
                ai_workspace.mcp_draft_is_valid(self.settings_store.settings()),
                ai_workspace
                    .mcp_transport()
                    .unwrap_or(oxideterm_ai::McpTransport::Stdio),
                ai_workspace
                    .mcp_auth_mode()
                    .unwrap_or(oxideterm_ai::McpAuthHeaderMode::Bearer),
                ai_workspace.mcp_dialog_presence(),
            )
        };
        let transport_label = ai_mcp_transport_label(transport);
        let auth_mode_label = match auth_header_mode {
            oxideterm_ai::McpAuthHeaderMode::Bearer => {
                self.i18n.t("settings_view.mcp.auth_header_mode_bearer")
            }
            oxideterm_ai::McpAuthHeaderMode::Raw => {
                self.i18n.t("settings_view.mcp.auth_header_mode_raw")
            }
            oxideterm_ai::McpAuthHeaderMode::None => {
                self.i18n.t("settings_view.mcp.auth_header_mode_none")
            }
        };

        let backdrop = dismissible_dialog_backdrop().on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _event, _window, cx| {
                // Tauri McpServersPanel binds Add Server Dialog
                // onOpenChange to setShowAddDialog(false).
                this.close_ai_mcp_add_dialog(cx);
                cx.stop_propagation();
                cx.notify();
            }),
        );
        let form = dialog_content(&self.tokens)
            .w(px(AI_MCP_DIALOG_WIDTH))
            .max_w(relative(0.92))
            .max_h(relative(0.86))
            .shadow_lg()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            })
            .child(
                dialog_header(&self.tokens)
                    .child(dialog_title(
                        &self.tokens,
                        self.i18n.t("settings_view.mcp.add_server_title"),
                    ))
                    .child(dialog_description(
                        &self.tokens,
                        self.i18n.t("settings_view.mcp.add_server_description"),
                    )),
            )
            .child(
                div()
                    .id("ai-mcp-add-server-scroll")
                    .flex_1()
                    .min_h(px(0.0))
                    .selectable_overflow_y_scrollbar(
                        &self.selectable_text_scroll_handle("ai-mcp-add-server-scroll"),
                    )
                    .px(px(AI_MCP_DIALOG_CONTENT_PX))
                    .py(px(AI_MCP_DIALOG_CONTENT_PY))
                    .flex()
                    .flex_col()
                    .gap(px(AI_MCP_FORM_GAP))
                    .child(self.ai_mcp_labeled_input(
                        "settings_view.mcp.server_name",
                        SettingsInput::AiMcpName,
                        "my-mcp-server".to_string(),
                        cx,
                    ))
                    .child(self.ai_mcp_labeled_select(
                        "settings_view.mcp.transport",
                        SettingsSelect::AiMcpTransport,
                        transport_label,
                        cx,
                    ))
                    .children(self.ai_mcp_transport_fields(transport, auth_mode_label, cx)),
            )
            .child(
                dialog_footer(&self.tokens)
                    .child(self.standard_footer_action_button(
                        self.i18n.t("settings_view.mcp.cancel"),
                        ButtonVariant::Outline,
                        ConfirmDialogAction::Cancel,
                        false,
                        |this, _event, _window, cx| {
                            this.close_ai_mcp_add_dialog(cx);
                        },
                        cx,
                    ))
                    .child(self.standard_footer_action_button(
                        self.i18n.t("settings_view.mcp.add"),
                        ButtonVariant::Default,
                        ConfirmDialogAction::Confirm,
                        !can_add,
                        |this, _event, _window, cx| {
                            this.submit_ai_mcp_add_dialog(cx);
                        },
                        cx,
                    )),
            );
        Some(settings_dialog_transition(
            &self.tokens,
            "ai-mcp-dialog-form",
            backdrop,
            form,
            dialog_presence.phase(),
        ))
    }

    pub(in crate::workspace) fn handle_ai_mcp_add_dialog_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let can_add = {
            let ai_workspace = self.ai_entity.read(cx);
            if !ai_workspace.mcp_dialog_is_open() {
                return false;
            }
            ai_workspace.mcp_draft_is_valid(self.settings_store.settings())
        };
        if self.open_settings_select.is_some()
            || self.focused_settings_input.is_some()
            || self.ai_entity.read(cx).focused_settings_input().is_some()
        {
            return false;
        }

        let key = event.keystroke.key.as_str();
        let footer_focused = self.standard_confirm_focus_owner().is_some();
        if matches!(key, "enter" | "space" | " ") && !footer_focused {
            return false;
        }

        match self.handle_standard_confirm_key(event, cx) {
            Some(ConfirmKeyboardAction::Cancel) => {
                self.close_ai_mcp_add_dialog(cx);
                true
            }
            Some(ConfirmKeyboardAction::Confirm) => {
                if can_add {
                    self.submit_ai_mcp_add_dialog(cx);
                } else {
                    // Disabled primary buttons remain in the dialog; restore
                    // focus to the first footer action like a browser footer loop.
                    self.reset_standard_confirm_focus();
                    cx.notify();
                }
                true
            }
            Some(ConfirmKeyboardAction::Handled) => true,
            None => false,
        }
    }

    pub(in crate::workspace) fn ai_mcp_transport_fields(
        &self,
        transport: oxideterm_ai::McpTransport,
        auth_mode_label: String,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        if transport == oxideterm_ai::McpTransport::Stdio {
            return vec![
                self.ai_mcp_labeled_input(
                    "settings_view.mcp.command",
                    SettingsInput::AiMcpCommand,
                    "npx".to_string(),
                    cx,
                ),
                self.ai_textarea_row(
                    SettingsInput::AiMcpArgs,
                    self.i18n.t("settings_view.mcp.args"),
                    String::new(),
                    "-y @modelcontextprotocol/server-example".to_string(),
                    String::new(),
                    AI_MCP_ARGS_TEXTAREA_MIN_H,
                    cx,
                ),
                self.ai_mcp_key_value_editor(true, cx),
            ];
        }
        vec![
            self.ai_mcp_labeled_input(
                "settings_view.mcp.url",
                SettingsInput::AiMcpUrl,
                "http://localhost:3000".to_string(),
                cx,
            ),
            div()
                .grid()
                .grid_cols(2)
                .gap(px(12.0))
                .child(self.ai_mcp_labeled_input(
                    "settings_view.mcp.auth_header_name",
                    SettingsInput::AiMcpAuthHeaderName,
                    "Authorization".to_string(),
                    cx,
                ))
                .child(self.ai_mcp_labeled_select(
                    "settings_view.mcp.auth_header_mode",
                    SettingsSelect::AiMcpAuthMode,
                    auth_mode_label,
                    cx,
                ))
                .into_any_element(),
            self.ai_mcp_auth_token_input(cx),
            self.ai_mcp_key_value_editor(false, cx),
            self.ai_mcp_retry_row(cx),
        ]
    }

    pub(in crate::workspace) fn ai_mcp_text_input_control(
        &self,
        input: SettingsInput,
        placeholder: String,
        secret: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let target = WorkspaceImeTarget::Settings(input);
        let input_control = {
            let ai_workspace = self.ai_entity.read(cx);
            let focused = ai_workspace.focused_settings_input() == Some(input);
            text_input(
                &self.tokens,
                TextInputView {
                    value: ai_workspace.settings_input_value(input).unwrap_or_default(),
                    placeholder,
                    focused,
                    caret_visible: self.input_caret.visible(),
                    secret,
                    selected_all: false,
                    selected_range: self.ime_selected_range_for_target(target, cx),
                    marked_text: self.marked_text_for_target(target, cx),
                },
            )
        };
        self.text_input_with_workspace_ime(
            target,
            input_control.w_full().min_w(px(0.0)),
            move |this, cx| {
                this.focus_settings_input(input, String::new(), cx);
            },
            cx,
        )
        .into_any_element()
    }

    pub(in crate::workspace) fn ai_mcp_labeled_input(
        &self,
        label_key: &str,
        input: SettingsInput,
        placeholder: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(px(AI_MCP_FIELD_GAP))
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_sm))
                    .text_color(rgb(self.tokens.ui.text))
                    .child(self.render_selectable_display_text(
                        "ai-mcp-field-label",
                        label_key,
                        self.i18n.t(label_key),
                        self.tokens.ui.text,
                        cx,
                    )),
            )
            .child(self.ai_mcp_text_input_control(input, placeholder, false, cx))
            .into_any_element()
    }

    pub(in crate::workspace) fn ai_mcp_labeled_select(
        &self,
        label_key: &str,
        select_id: SettingsSelect,
        value: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(px(AI_MCP_FIELD_GAP))
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_sm))
                    .text_color(rgb(self.tokens.ui.text))
                    .child(self.render_selectable_display_text(
                        "ai-mcp-select-label",
                        label_key,
                        self.i18n.t(label_key),
                        self.tokens.ui.text,
                        cx,
                    )),
            )
            .child(self.settings_select_control(select_id, value, false, None, cx))
            .into_any_element()
    }

    pub(in crate::workspace) fn ai_mcp_key_value_editor(
        &self,
        env: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let entry_count = self.ai_entity.read(cx).mcp_record_len(env);
        let title = if env {
            self.i18n.t("settings_view.mcp.env_vars")
        } else {
            self.i18n.t("settings_view.mcp.extra_headers")
        };
        let add_label = if env {
            self.i18n.t("settings_view.mcp.add_env_var")
        } else {
            self.i18n.t("settings_view.mcp.add_header")
        };
        let mut rows = div().flex().flex_col().gap(px(AI_MCP_FIELD_GAP));
        for index in 0..entry_count {
            let key_input = if env {
                SettingsInput::AiMcpEnvKey(index)
            } else {
                SettingsInput::AiMcpHeaderKey(index)
            };
            let value_input = if env {
                SettingsInput::AiMcpEnvValue(index)
            } else {
                SettingsInput::AiMcpHeaderValue(index)
            };
            rows = rows.child(
                div()
                    .flex()
                    .gap(px(AI_MCP_FIELD_GAP))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(self.ai_mcp_text_input_control(
                                key_input,
                                if env {
                                    self.i18n.t("settings_view.mcp.env_key_placeholder")
                                } else {
                                    self.i18n.t("settings_view.mcp.header_key_placeholder")
                                },
                                false,
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(self.ai_mcp_text_input_control(
                                value_input,
                                if env {
                                    self.i18n.t("settings_view.mcp.env_value_placeholder")
                                } else {
                                    self.i18n.t("settings_view.mcp.header_value_placeholder")
                                },
                                false,
                                cx,
                            )),
                    )
                    .child(self.ai_icon_button(
                        LucideIcon::Trash2,
                        false,
                        move |this, _event, _window, cx| {
                            this.ai_entity.update(cx, |ai, cx| {
                                ai.remove_mcp_record_entry(env, index, cx);
                            });
                            cx.stop_propagation();
                        },
                        cx,
                    )),
            );
        }
        rows = rows.child(self.workspace_toolbar_action_button(
            add_label,
            Some(Self::render_lucide_icon(
                LucideIcon::Plus,
                14.0,
                rgb(self.tokens.ui.text),
            )),
            ToolbarButtonOptions {
                button: ButtonOptions {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    radius: ButtonRadius::Md,
                    disabled: false,
                },
                icon_gap: Some(6.0),
                ..ToolbarButtonOptions::default()
            },
            cx.listener(move |this, _event, _window, cx| {
                this.ai_entity.update(cx, |ai, cx| {
                    ai.add_mcp_record_entry(env, cx);
                });
                cx.stop_propagation();
            }),
        ));
        div()
            .flex()
            .flex_col()
            .gap(px(AI_MCP_FIELD_GAP))
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_sm))
                    .text_color(rgb(self.tokens.ui.text))
                    .child(title),
            )
            .child(rows)
            .when(!env, |section| {
                section.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(self.tokens.ui.text_muted))
                        .child(self.i18n.t("settings_view.mcp.extra_headers_hint")),
                )
            })
            .into_any_element()
    }

    pub(in crate::workspace) fn ai_mcp_auth_token_input(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let secret = !self.ai_entity.read(cx).mcp_auth_token_visible();
        let input = SettingsInput::AiMcpAuthToken;
        div()
            .flex()
            .flex_col()
            .gap(px(AI_MCP_FIELD_GAP))
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_sm))
                    .text_color(rgb(self.tokens.ui.text))
                    .child(self.i18n.t("settings_view.mcp.auth_token")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(AI_MCP_FIELD_GAP))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(self.ai_mcp_text_input_control(
                                input,
                                self.i18n.t("settings_view.mcp.auth_token_placeholder"),
                                secret,
                                cx,
                            )),
                    )
                    .child(self.ai_icon_button(
                        if secret {
                            LucideIcon::Eye
                        } else {
                            LucideIcon::EyeOff
                        },
                        false,
                        |this, _event, _window, cx| {
                            this.ai_entity.update(cx, |ai, cx| {
                                ai.toggle_mcp_auth_token_visibility(cx);
                            });
                            cx.stop_propagation();
                        },
                        cx,
                    )),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn ai_mcp_retry_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let checked = self.ai_entity.read(cx).mcp_retry_enabled();
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_sm))
                    .text_color(rgb(self.tokens.ui.text))
                    .child(self.i18n.t("settings_view.mcp.retry_on_disconnect")),
            )
            .child(
                checkbox(&self.tokens, String::new(), checked)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event, _window, cx| {
                            this.ai_entity.update(cx, |ai, cx| {
                                ai.toggle_mcp_retry(cx);
                            });
                            cx.stop_propagation();
                        }),
                    )
                    .into_any_element(),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn close_ai_mcp_add_dialog(&mut self, cx: &mut Context<Self>) {
        self.close_settings_select();
        self.clear_standard_confirm_focus();
        self.ime_marked_text = None;
        self.clear_ime_selection();
        let delay = oxideterm_gpui_ui::motion::duration(
            &self.tokens,
            oxideterm_gpui_ui::motion::MotionDuration::Overlay,
        );
        self.ai_entity.update(cx, |ai, cx| {
            ai.begin_mcp_dialog_exit(false, delay, HashSet::new(), cx);
        });
        cx.notify();
    }

    pub(in crate::workspace) fn submit_ai_mcp_add_dialog(&mut self, cx: &mut Context<Self>) {
        self.close_settings_select();
        self.clear_standard_confirm_focus();
        self.ime_marked_text = None;
        self.clear_ime_selection();
        let delay = oxideterm_gpui_ui::motion::duration(
            &self.tokens,
            oxideterm_gpui_ui::motion::MotionDuration::Overlay,
        );
        let configured_names = ai_mcp_configs(self.settings_store.settings())
            .into_iter()
            .map(|config| config.name)
            .collect();
        self.ai_entity.update(cx, |ai, cx| {
            ai.begin_mcp_dialog_exit(true, delay, configured_names, cx);
        });
        cx.notify();
    }
}
