use super::*;

impl WorkspaceApp {
    pub(in crate::workspace) fn refresh_ai_skill_registry(&mut self) {
        let disabled_paths = self
            .settings_store
            .settings()
            .ai
            .skills
            .disabled_paths
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
        let registry =
            oxideterm_skills::SkillRegistry::discover(&oxideterm_skills::SkillDiscoveryOptions {
                workspace_root: self.skill_workspace_root.clone(),
                settings_path: Some(self.settings_store.path().to_path_buf()),
                plugin_roots: Vec::new(),
                disabled_paths,
            });
        *self.skill_registry.write() = registry;
        // A disabled or replaced skill must be loaded again before resources
        // can be read in any existing conversation.
        self.loaded_conversation_skills.clear();
    }

    pub(in crate::workspace) fn ai_icon_button(
        &self,
        icon: LucideIcon,
        disabled: bool,
        on_click: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let icon_color = if matches!(icon, LucideIcon::Trash2) {
            rgb(self.tokens.ui.error)
        } else {
            rgb(self.tokens.ui.text_muted)
        };

        self.workspace_icon_action_button(
            icon,
            15.0,
            icon_color,
            IconButtonOptions {
                disabled,
                hover_background: Some(rgba((self.tokens.ui.bg_hover << 8) | 0x80)),
                // Tauri AI action buttons are fully opaque until disabled; the
                // workspace wrapper owns the disabled activation guard.
                ..IconButtonOptions::opaque_toolbar(30.0, ButtonRadius::Md)
            },
            on_click,
            cx,
        )
        .into_any_element()
    }

    pub(in crate::workspace) fn ai_textarea_row(
        &self,
        input: SettingsInput,
        label: String,
        hint: String,
        placeholder: String,
        value: String,
        min_height: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let entity_owned = ai_state::AiWorkspaceEntity::owns_settings_input(input);
        let focused = if entity_owned {
            self.ai_entity.read(cx).focused_settings_input() == Some(input)
        } else {
            self.focused_settings_input == Some(input)
        };
        let target = WorkspaceImeTarget::Settings(input);
        let workspace = cx.entity();
        let marked_text = self
            .marked_text_for_target(target, cx)
            .map(|marked| marked.to_string());
        let caret = focused
            .then(|| text_caret(&self.tokens, self.input_caret.visible()).into_any_element());
        let textarea = if entity_owned {
            let ai_workspace = self.ai_entity.read(cx);
            settings_ai_textarea_surface(
                &self.tokens,
                min_height,
                focused,
                ai_workspace.settings_input_value(input).unwrap_or_default(),
                &placeholder,
                marked_text,
                caret,
            )
        } else {
            let display_value = if focused {
                self.settings_input_draft.as_str()
            } else {
                value.as_str()
            };
            settings_ai_textarea_surface(
                &self.tokens,
                min_height,
                focused,
                display_value,
                &placeholder,
                marked_text,
                caret,
            )
        }
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                let current = (!ai_state::AiWorkspaceEntity::owns_settings_input(input))
                    .then(|| this.current_settings_input_value(input, cx))
                    .unwrap_or_default();
                this.focus_settings_input(input, current, cx);
                this.ime_marked_text = None;
                window.focus(&this.focus_handle, cx);
                this.begin_ime_selection_from_mouse_down(target, event, window, cx);
                cx.stop_propagation();
            }),
        )
        .on_mouse_move(
            cx.listener(|this, event: &gpui::MouseMoveEvent, window, cx| {
                this.update_ime_selection_drag_from_mouse_move(event, window, cx);
            }),
        );

        let control =
            text_input_anchor_probe(target.anchor_id(), textarea, move |anchor, _window, cx| {
                let _ = workspace.update(cx, |this, cx| {
                    this.update_text_input_anchor(anchor, cx);
                });
            });

        settings_ai_textarea_row(&self.tokens, label, control.into_any_element(), hint)
    }
}

