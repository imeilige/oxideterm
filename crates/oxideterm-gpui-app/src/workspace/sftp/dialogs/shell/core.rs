use super::*;

impl WorkspaceApp {
    pub(in crate::workspace) fn render_sftp_dialog(
        &self,
        dialog: SftpDialog,
        has_background: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dialog_visible = self.sftp_view().read(cx).dialog_presence.phase()
            == oxideterm_gpui_ui::motion::ExitPhase::Visible;
        let theme = self.tokens.ui;
        let (title, description, body, primary) = match dialog.clone() {
            SftpDialog::Drives => (
                self.i18n.t("sftp.dialogs.select_drive"),
                self.i18n.t("sftp.dialogs.select_drive_desc"),
                self.render_sftp_drives_dialog_body(has_background, cx),
                None,
            ),
            SftpDialog::Rename { .. } => (
                self.i18n.t("sftp.dialogs.rename"),
                self.i18n.t("sftp.dialogs.rename_desc"),
                self.render_sftp_dialog_input("sftp.dialogs.rename_desc", cx),
                Some(self.i18n.t("sftp.dialogs.rename")),
            ),
            SftpDialog::NewFolder { .. } => (
                self.i18n.t("sftp.dialogs.new_folder"),
                self.i18n.t("sftp.dialogs.new_folder_desc"),
                self.render_sftp_dialog_input("sftp.dialogs.new_folder_placeholder", cx),
                Some(self.i18n.t("sftp.dialogs.create")),
            ),
            SftpDialog::Delete { files, .. } => (
                self.i18n.t("sftp.dialogs.delete"),
                self.i18n
                    .t("sftp.dialogs.delete_confirm")
                    .replace("{{count}}", &files.len().to_string()),
                self.render_sftp_delete_dialog_body(files, has_background, cx),
                Some(self.i18n.t("sftp.dialogs.delete")),
            ),
            SftpDialog::Conflict => (
                self.i18n.t("sftp.conflict.title"),
                self.sftp_conflict_description(),
                self.render_sftp_conflict_body(has_background, cx),
                Some(self.i18n.t("sftp.conflict.keep_both")),
            ),
        };
        let width = match &dialog {
            SftpDialog::Drives => SFTP_DIALOG_WIDTH_XS,
            SftpDialog::Rename { .. }
            | SftpDialog::NewFolder { .. }
            | SftpDialog::Delete { .. } => SFTP_DIALOG_WIDTH_SM,
            SftpDialog::Conflict => SFTP_DIALOG_WIDTH_LG,
        };
        let header_py = 12.0;
        let show_description = !description.is_empty();
        let edge_owned_dialog = false;

        let outside_dialog = dialog.clone();
        dismissible_dialog_backdrop()
            .on_mouse_down(
                MouseButton::Left,
                self.sftp_listener(cx, move |this, _event, _window, cx| {
                    // Tauri SFTP dialogs are Radix Dialogs. Backdrop clicks map
                    // to their onOpenChange(false) close/cancel path.
                    match outside_dialog {
                        SftpDialog::Conflict => this.cancel_sftp_transfer_conflicts(cx),
                        _ => this.close_sftp_dialog(cx),
                    }
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .child(oxideterm_gpui_ui::motion::form_transition(
                &self.tokens,
                "sftp-dialog-presence",
                div()
                    .w(px(width))
                    .max_w(relative(0.9))
                    .max_h(relative(0.9))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(px(self.tokens.radii.md))
                    .border_1()
                    .border_color(rgb(theme.border))
                    .when(!edge_owned_dialog, |dialog| {
                        // Compact dialogs rely on DialogContent's background behind
                        // padded body content. Full-height preview/editor/diff shells
                        // let header, body, and footer own every edge so GPUI's
                        // rectangular overflow mask cannot expose a second corner color.
                        material_surface(&self.tokens, dialog, MaterialRole::Dialog)
                    })
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                        cx.stop_propagation();
                    })
                    .shadow(vec![gpui::BoxShadow {
                        inset: false,
                        color: rgba(SFTP_DIALOG_SHADOW_ALPHA).into_color(),
                        offset: gpui::point(px(0.0), px(16.0)),
                        blur_radius: px(32.0),
                        spread_radius: px(0.0),
                    }])
                    .child(
                        div()
                            .px(px(16.0))
                            .py(px(header_py))
                            .border_b_1()
                            .border_color(rgb(theme.border))
                            // Browser DialogContent clips this painted header
                            // into the rounded shell. GPUI needs the edge child
                            // to own the top corners to avoid rectangular leaks.
                            .rounded_t(px(rounded_shell_child_radius(self.tokens.radii.md)))
                            // Mirrors DialogHeader bg-theme-bg-panel, not the tab background alpha path.
                            .bg(rgb(theme.bg_panel))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .text_size(px(SFTP_TEXT_SM))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(theme.text_heading))
                                    .when(matches!(&dialog, SftpDialog::Conflict), |row| {
                                        row.child(Self::render_lucide_icon(
                                            LucideIcon::AlertTriangle,
                                            20.0,
                                            rgb(SFTP_YELLOW),
                                        ))
                                    })
                                    .child(self.render_selectable_text_scoped(
                                        "sftp-dialog-title",
                                        &title,
                                        title.clone(),
                                        theme.text_heading,
                                        cx,
                                    )),
                            )
                            .when(show_description, |header| {
                                header.child(
                                    div()
                                        .mt(px(6.0))
                                        .text_size(px(SFTP_TEXT_SM))
                                        .text_color(rgb(theme.text_muted))
                                        .when(matches!(&dialog, SftpDialog::Conflict), |desc| {
                                            let remaining = self.sftp_conflict_remaining_count(cx);
                                            desc.flex()
                                                .items_center()
                                                .gap(px(4.0))
                                                .child(self.render_selectable_text_scoped(
                                                    "sftp-dialog-description",
                                                    &title,
                                                    description.clone(),
                                                    theme.text_muted,
                                                    cx,
                                                ))
                                                .when(remaining > 0, |desc| {
                                                    desc.child(
                                                        div().text_color(rgb(SFTP_ORANGE)).child(
                                                            self.render_selectable_text_scoped(
                                                                "sftp-conflict-remaining",
                                                                remaining,
                                                                self.i18n
                                                                    .t("sftp.conflict.remaining")
                                                                    .replace(
                                                                        "{{count}}",
                                                                        &remaining.to_string(),
                                                                    ),
                                                                SFTP_ORANGE,
                                                                cx,
                                                            ),
                                                        ),
                                                    )
                                                })
                                        })
                                        .when(!matches!(&dialog, SftpDialog::Conflict), |desc| {
                                            desc.child(self.render_selectable_text_scoped(
                                                "sftp-dialog-description",
                                                &title,
                                                description,
                                                theme.text_muted,
                                                cx,
                                            ))
                                        }),
                                )
                            }),
                    )
                    .child(body)
                    .child(self.render_sftp_dialog_footer(
                        dialog.clone(),
                        primary,
                        has_background,
                        cx,
                    )),
                dialog_visible,
            ))
            .when(!dialog_visible, |backdrop| {
                // Retained exit payloads are visual-only and cannot receive
                // clicks, wheel input, or button activation while fading out.
                backdrop.child(
                    div()
                        .absolute()
                        .top_0()
                        .right_0()
                        .bottom_0()
                        .left_0()
                        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                            cx.stop_propagation();
                        })
                        .on_mouse_down(MouseButton::Right, |_event, _window, cx| {
                            cx.stop_propagation();
                        })
                        .on_scroll_wheel(|_event, _window, cx| cx.stop_propagation()),
                )
            })
            .into_any_element()
    }

    fn render_sftp_dialog_footer(
        &self,
        dialog: SftpDialog,
        primary: Option<String>,
        _has_background: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let (footer_px, footer_py) = (16.0, 12.0);
        let footer = div()
            .px(px(footer_px))
            .py(px(footer_py))
            .border_t_1()
            .border_color(rgb(theme.border))
            // Keep the footer background inside the dialog shell radius just
            // like Tauri's overflow-hidden DialogContent clipping.
            .rounded_b(px(rounded_shell_child_radius(self.tokens.radii.md)))
            // Mirrors DialogFooter bg-theme-bg-panel, not the tab background alpha path.
            .bg(rgb(theme.bg_panel))
            .flex()
            .flex_row()
            .flex_wrap()
            .justify_end()
            .gap(px(8.0));

        if matches!(dialog, SftpDialog::Conflict) {
            let source_newer = self
                .sftp_view()
                .read(cx)
                .conflict_state
                .as_ref()
                .and_then(|state| state.conflicts.get(state.current_index))
                .and_then(|conflict| Some(conflict.source_modified? > conflict.target_modified?));
            return footer
                .justify_between()
                .child(
                    div()
                        .flex()
                        .gap(px(8.0))
                        .child(self.render_sftp_button_variant(
                            self.i18n.t("sftp.conflict.skip"),
                            SftpButtonVariant::Ghost,
                            self.sftp_listener(cx, |this, _event, _window, cx| {
                                this.resolve_sftp_transfer_conflict(
                                    SftpConflictResolution::Skip,
                                    cx,
                                );
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        ))
                        .when(source_newer.is_some(), |actions| {
                            actions.child(self.render_sftp_button_variant(
                                self.i18n.t("sftp.conflict.skip_older"),
                                SftpButtonVariant::Ghost,
                                self.sftp_listener(cx, |this, _event, _window, cx| {
                                    this.resolve_sftp_transfer_conflict(
                                        SftpConflictResolution::SkipOlder,
                                        cx,
                                    );
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            ))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(8.0))
                        .child(self.render_sftp_button_variant(
                            self.i18n.t("sftp.conflict.keep_both"),
                            SftpButtonVariant::Default,
                            self.sftp_listener(cx, |this, _event, _window, cx| {
                                this.resolve_sftp_transfer_conflict(
                                    SftpConflictResolution::Rename,
                                    cx,
                                );
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        ))
                        .child(self.render_sftp_button_variant(
                            self.i18n.t("sftp.conflict.overwrite"),
                            SftpButtonVariant::Destructive,
                            self.sftp_listener(cx, |this, _event, _window, cx| {
                                this.resolve_sftp_transfer_conflict(
                                    SftpConflictResolution::Overwrite,
                                    cx,
                                );
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        )),
                )
                .into_any_element();
        }

        footer
            .child(self.render_sftp_text_button(
                self.i18n.t("sftp.dialogs.cancel"),
                false,
                self.sftp_listener(cx, |this, _event, _window, cx| {
                    this.close_sftp_dialog(cx);
                    cx.stop_propagation();
                    cx.notify();
                }),
            ))
            .when_some(primary, |footer, label| {
                footer.child(self.render_sftp_text_button(
                    label,
                    true,
                    self.sftp_listener(cx, |this, _event, _window, cx| {
                        this.accept_sftp_dialog(cx);
                        cx.stop_propagation();
                        cx.notify();
                    }),
                ))
            })
            .into_any_element()
    }
}
