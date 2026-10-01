use super::*;

impl WorkspaceApp {
    pub(in crate::workspace) fn knowledge_icon_button(
        &self,
        icon: LucideIcon,
        color: gpui::Rgba,
        hover_color: Option<gpui::Rgba>,
        listener: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Div {
        // The original local helper accepted a hover text color, but the icon
        // SVG is rendered with an explicit color. Keep the parameter until the
        // shared icon primitive grows a real hover-icon-color slot.
        let _ = hover_color;
        self.workspace_icon_action_button(
            icon,
            KNOWLEDGE_INLINE_ICON_SIZE,
            color,
            IconButtonOptions {
                hover_background: Some(rgba((0xffffff << 8) | KNOWLEDGE_ICON_BUTTON_HOVER_ALPHA)),
                ..IconButtonOptions::opaque_toolbar(KNOWLEDGE_ICON_BUTTON_SIZE, ButtonRadius::Sm)
            },
            listener,
            cx,
        )
    }

    pub(in crate::workspace) fn knowledge_format_date(&self, timestamp_millis: i64) -> String {
        let Some(datetime) = chrono::DateTime::from_timestamp_millis(timestamp_millis) else {
            return "-".to_string();
        };
        let datetime = datetime.with_timezone(&chrono::Local);
        match self.i18n.locale() {
            Locale::ZhCn | Locale::ZhTw => datetime.format("%Y年%-m月%-d日").to_string(),
            _ => datetime.format("%b %-d, %Y").to_string(),
        }
    }

    pub(in crate::workspace) fn knowledge_error_row(&self, error: &str) -> AnyElement {
        div()
            .rounded(px(self.tokens.radii.lg))
            .border_1()
            .border_color(rgba((self.tokens.ui.error << 8) | 0x4d))
            .bg(rgba((self.tokens.ui.error << 8) | 0x1a))
            .p(px(12.0))
            .text_size(px(self.tokens.metrics.ui_text_sm))
            .text_color(rgb(self.tokens.ui.error))
            .child(error.to_string())
            .into_any_element()
    }

    pub(in crate::workspace) fn knowledge_document_row(
        &self,
        document: oxideterm_ai::RagDocumentResponse,
        open_in_workspace: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let menu_document = document.clone();
        let delete_id = document.id.clone();
        let delete_name = document.title.clone();
        let edit_id = document.id.clone();
        let open_id = document.id.clone();
        let selected = open_in_workspace && self.is_knowledge_document_selected(&document.id, cx);
        let metadata = format!(
            "{} · {} {} · {}",
            document.format,
            document.chunk_count,
            self.i18n.t("settings_view.knowledge.chunks"),
            self.knowledge_format_date(document.indexed_at)
        );
        let editing_this = !open_in_workspace
            && self
                .ai_entity
                .read(cx)
                .knowledge_external_edit()
                .is_some_and(|edit| edit.doc_id == document.id);
        let mut options = oxideterm_gpui_ui::EntityListRowOptions::new()
            .active(selected)
            .has_background_image(self.background_surface_active("knowledge"));
        if open_in_workspace {
            options = options
                .compact()
                .hover_background(oxideterm_gpui_ui::color_for_background(
                    self.tokens.ui.bg_hover,
                    self.background_surface_active("knowledge"),
                    0x66,
                ));
        }
        let mut trailing = Vec::with_capacity(2);
        if !open_in_workspace {
            trailing.push(
                if editing_this {
                    self.knowledge_icon_button(
                        LucideIcon::RefreshCw,
                        rgb(self.tokens.ui.accent),
                        Some(rgb(self.tokens.ui.accent)),
                        |this, _event, _window, cx| {
                            this.knowledge_sync_external_edit(true, cx);
                            cx.stop_propagation();
                        },
                        cx,
                    )
                } else {
                    self.knowledge_icon_button(
                        LucideIcon::Pencil,
                        rgb(self.tokens.ui.text_muted),
                        Some(rgb(self.tokens.ui.text)),
                        move |this, _event, _window, cx| {
                            this.knowledge_open_external(edit_id.clone(), cx);
                            cx.stop_propagation();
                        },
                        cx,
                    )
                }
                .into_any_element(),
            );
        }
        trailing.push(
            self.knowledge_icon_button(
                LucideIcon::Trash2,
                rgb(self.tokens.ui.text_muted),
                Some(rgb(self.tokens.ui.error)),
                move |this, _event, _window, cx| {
                    this.ai_entity.update(cx, |entity, cx| {
                        entity.request_delete_knowledge_document(
                            delete_id.clone(),
                            delete_name.clone(),
                        );
                        cx.notify();
                    });
                    this.reset_standard_confirm_focus();
                    cx.stop_propagation();
                    cx.notify();
                },
                cx,
            )
            .into_any_element(),
        );
        let row = oxideterm_gpui_ui::entity_list_row(
            &self.tokens,
            options,
            Some(
                oxideterm_gpui_ui::file_icons::file_icon(&format!("note.{}", document.format))
                    .render(
                        if open_in_workspace {
                            KNOWLEDGE_INLINE_ICON_SIZE
                        } else {
                            KNOWLEDGE_ROW_ICON_SIZE
                        },
                        &self.tokens,
                    ),
            ),
            div()
                .min_w(px(0.0))
                .truncate()
                .text_size(px(if open_in_workspace {
                    self.tokens.metrics.ui_text_xs
                } else {
                    self.tokens.metrics.ui_text_sm
                }))
                .text_color(rgb(self.tokens.ui.text))
                .child(document.title)
                .into_any_element(),
            (!open_in_workspace).then(|| {
                div()
                    .min_w(px(0.0))
                    .truncate()
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .text_color(rgb(self.tokens.ui.text_muted))
                    .child(metadata)
                    .into_any_element()
            }),
            Vec::new(),
            trailing,
        )
        .id(format!("knowledge-document-row-{}", document.id))
        .when(open_in_workspace, |row| {
            row.h(px(KNOWLEDGE_WORKSPACE_SECTION_ESTIMATED_HEIGHT))
                .min_h(px(KNOWLEDGE_WORKSPACE_SECTION_ESTIMATED_HEIGHT))
                .py_0()
                .px(px(self.tokens.spacing.two))
                .rounded_none()
                .border_0()
                .bg(if selected {
                    rgba((self.tokens.ui.accent << 8) | 0x33)
                } else {
                    rgba(0x00000000)
                })
                .opacity(if self.knowledge_note_is_cut(&menu_document.id, cx) {
                    0.5
                } else {
                    1.0
                })
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                        this.open_knowledge_note_menu(
                            menu_document.clone(),
                            event.position,
                            window,
                            cx,
                        );
                        cx.stop_propagation();
                    }),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, window, cx| {
                        this.open_knowledge_workspace_tab(window, cx);
                        this.select_knowledge_document(open_id.clone(), cx);
                        cx.stop_propagation();
                    }),
                )
        });
        row.into_any_element()
    }

    pub(in crate::workspace) fn knowledge_empty_row(
        &self,
        icon: LucideIcon,
        label: String,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .px(px(16.0))
            .py(px(32.0))
            .text_center()
            .text_color(rgb(self.tokens.ui.text_muted))
            .child(Self::render_lucide_icon(
                icon,
                32.0,
                rgba((self.tokens.ui.text_muted << 8) | 0x66),
            ))
            .child(
                div()
                    .w_full()
                    .text_center()
                    .text_size(px(self.tokens.metrics.ui_text_sm))
                    .child(label),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn knowledge_document_format_label(
        &self,
        cx: &Context<Self>,
    ) -> String {
        match self.ai_entity.read(cx).knowledge_new_document_format() {
            "plaintext" => self.i18n.t("settings_view.knowledge.format_plain_text"),
            _ => self.i18n.t("settings_view.knowledge.format_markdown"),
        }
    }
}
