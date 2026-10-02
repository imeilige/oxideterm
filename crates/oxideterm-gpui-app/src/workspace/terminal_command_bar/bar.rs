// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use super::*;
use gpui::StatefulInteractiveElement;
use oxideterm_gpui_ui::dropdown_menu::{
    DropdownMenuItemKind, dropdown_menu_content, dropdown_menu_item,
};

const TERMINAL_RECORDING_MENU_WIDTH: f32 = 220.0;
const TERMINAL_RECORDING_MENU_BOTTOM: f32 = 30.0;

impl WorkspaceApp {
    pub(super) fn render_terminal_command_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        const COMMAND_BAR_BG_ALPHA: u32 = 0xf2; // Tauri bg-theme-bg/95
        const COMMAND_BAR_BORDER_ALPHA: u32 = 0xb3; // Tauri border-theme-border/70

        let theme = self.tokens.ui;
        let command_bar_background = rgba((theme.bg << 8) | COMMAND_BAR_BG_ALPHA);
        let workspace = cx.entity();
        // The visible chip and completion providers share Tauri's target-label
        // inference so local shells that are currently inside SSH show the
        // remote identity consistently in both places.
        let target_label = self.terminal_command_active_target_label(cx);
        let cwd_display_enabled = self.terminal_current_directory_awareness_enabled()
            && self
                .settings_store
                .settings()
                .terminal
                .command_bar
                .show_current_directory;
        let cwd_snapshot = cwd_display_enabled
            .then(|| self.active_terminal_cwd_snapshot(cx))
            .flatten();
        let cwd_supported =
            cwd_display_enabled && self.active_terminal_cwd_scope_and_pane(cx).is_some();
        let git_snapshot = self.active_terminal_git_snapshot(cx);
        let project_tasks_enabled = self.terminal_project_tasks_enabled();
        let project_snapshot = project_tasks_enabled
            .then(|| self.active_terminal_project_snapshot(cx))
            .flatten();
        let active_pane_id = self.active_pane_id(cx);
        let is_local_terminal = self.active_terminal_kind(cx)
            == Some(oxideterm_terminal::TerminalSessionKind::LocalPty);
        let split_controls_visible = matches!(
            self.active_terminal_kind(cx),
            Some(
                oxideterm_terminal::TerminalSessionKind::LocalPty
                    | oxideterm_terminal::TerminalSessionKind::SshPty
            )
        );
        let can_configure_remote_integration = self.active_ssh_terminal_node_id(cx).is_some();
        let remote_integration_pending = self.remote_shell_integration_pending(cx);
        let remote_integration_tooltip_id = "terminal-command-configure-directory-tracking";
        let remote_integration_tooltip_title = self
            .i18n
            .t("settings_view.connections.shell_integration.toolbar_action");
        let target_indicator_is_local =
            is_local_terminal && target_label == self.i18n.t("terminal.command_bar.local_shell");
        let can_split = self.can_split_active_pane(cx);
        let highlight_override_active = self.active_terminal_highlight_override(cx);
        let bar = div()
            .relative()
            .flex_none()
            .border_t_1()
            .border_color(rgba((theme.border << 8) | COMMAND_BAR_BORDER_ALPHA))
            .bg(command_bar_background)
            .px(px(12.0))
            .py(px(4.0))
            .shadow_lg()
            .when(self.terminal_highlight_popover_open, |bar| {
                bar.child(self.render_terminal_highlight_popover(cx))
            })
            .when(self.terminal.read(cx).git_panel_open(), |bar| {
                bar.child(self.render_terminal_git_branch_picker(cx))
            })
            .when(
                cwd_display_enabled && self.terminal.read(cx).cwd_picker_open(),
                |bar| bar.child(self.render_terminal_cwd_picker(cx)),
            )
            .when(
                project_tasks_enabled && self.terminal.read(cx).project_panel_open(),
                |bar| bar.child(self.render_terminal_project_panel(cx)),
            )
            .child(
                div()
                    .w_full()
                    .min_w(px(0.0))
                    .min_h(px(24.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .child(self.render_terminal_target_indicator(
                                target_label,
                                target_indicator_is_local,
                                cx,
                            ))
                            .when(cwd_supported, |row| {
                                row.child(self.terminal_command_context_chip_slot(
                                    TERMINAL_COMMAND_CONTEXT_CHIP_MAX_WIDTH,
                                    self.render_terminal_cwd_chip(cwd_snapshot, cx),
                                ))
                            })
                            .when_some(git_snapshot, |row, snapshot| {
                                row.child(self.terminal_command_context_chip_slot(
                                    TERMINAL_COMMAND_CONTEXT_CHIP_MAX_WIDTH,
                                    self.render_terminal_git_chip(snapshot, cx),
                                ))
                            })
                            .when_some(project_snapshot, |row, snapshot| {
                                row.child(self.terminal_command_context_chip_slot(
                                    TERMINAL_COMMAND_PROJECT_CHIP_MAX_WIDTH,
                                    self.render_terminal_project_chip(snapshot, cx),
                                ))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap(px(4.0))
                            .when(split_controls_visible, |actions| {
                                // Transport readiness controls the disabled state; keeping SSH
                                // actions visible makes their placement match local terminals.
                                actions
                                    .child(self.terminal_command_action_button(
                                        LucideIcon::SplitSquareHorizontal,
                                        rgb(theme.text_muted),
                                        !can_split,
                                        None,
                                        "terminal-command-split-horizontal",
                                        self.i18n.t("command_palette.cmd_split_horizontal"),
                                        |this, _event, window, cx| {
                                            this.split_active_pane(
                                                SplitDirection::Horizontal,
                                                window,
                                                cx,
                                            );
                                            cx.stop_propagation();
                                        },
                                        cx,
                                    ))
                                    .child(self.terminal_command_action_button(
                                        LucideIcon::SplitSquareVertical,
                                        rgb(theme.text_muted),
                                        !can_split,
                                        None,
                                        "terminal-command-split-vertical",
                                        self.i18n.t("command_palette.cmd_split_vertical"),
                                        |this, _event, window, cx| {
                                            this.split_active_pane(
                                                SplitDirection::Vertical,
                                                window,
                                                cx,
                                            );
                                            cx.stop_propagation();
                                        },
                                        cx,
                                    ))
                            })
                            .when(can_configure_remote_integration, |actions| {
                                actions.child(self.terminal_command_action_button(
                                    LucideIcon::FolderSync,
                                    rgb(theme.text_muted),
                                    remote_integration_pending,
                                    None,
                                    remote_integration_tooltip_id,
                                    remote_integration_tooltip_title,
                                    |this, _event, _window, cx| {
                                        this.open_remote_shell_integration_confirm(cx);
                                        cx.stop_propagation();
                                    },
                                    cx,
                                ))
                            })
                            .when_some(active_pane_id, |actions, pane_id| {
                                // Capture the visible pane so the shortcut cannot retarget after a tab switch.
                                actions.child(self.terminal_command_action_button(
                                    LucideIcon::Activity,
                                    rgb(theme.text_muted),
                                    false,
                                    None,
                                    "terminal-command-session-triggers",
                                    self.i18n.t("terminal.command_selection.manage_triggers"),
                                    move |this, _event, window, cx| {
                                        this.open_terminal_trigger_settings_for_pane(
                                            pane_id, window, cx,
                                        );
                                        cx.stop_propagation();
                                    },
                                    cx,
                                ))
                            })
                            .child(select_anchor_probe(
                                SelectAnchorId::TerminalHighlightRuleSet,
                                self.terminal_command_action_button(
                                    LucideIcon::Hash,
                                    if highlight_override_active {
                                        rgb(theme.accent)
                                    } else {
                                        rgb(theme.text_muted)
                                    },
                                    false,
                                    Some(if highlight_override_active {
                                        rgba((theme.accent << 8) | 0x26)
                                    } else {
                                        rgba(0x00000000)
                                    }),
                                    "terminal-command-highlight-rules",
                                    self.i18n.t("terminal.highlight_override.title"),
                                    |this, _event, _window, cx| {
                                        this.toggle_terminal_highlight_popover(cx);
                                        cx.stop_propagation();
                                    },
                                    cx,
                                )
                                .relative(),
                                {
                                    let workspace = workspace.clone();
                                    move |anchor, _window, cx| {
                                        let _ = workspace.update(cx, |this, cx| {
                                            this.update_select_anchor(anchor, cx);
                                        });
                                    }
                                },
                            ))
                            .child(self.terminal_command_action_button(
                                LucideIcon::Search,
                                if self.search_visible(cx) {
                                    rgb(theme.accent)
                                } else {
                                    rgb(theme.text_muted)
                                },
                                false,
                                Some(if self.search_visible(cx) {
                                    rgba((theme.accent << 8) | 0x26)
                                } else {
                                    rgba(0x00000000)
                                }),
                                "terminal-command-search",
                                self.i18n.t("search.placeholder"),
                                |this, _event, window, cx| {
                                    if this.search_visible(cx) {
                                        this.close_search(window, cx);
                                    } else {
                                        this.open_search(window, cx);
                                    }
                                    cx.stop_propagation();
                                },
                                cx,
                            ))
                    ),
            );
        select_anchor_probe(
            SelectAnchorId::TerminalCommandBar,
            bar,
            move |anchor, _window, cx| {
                let _ = workspace.update(cx, |this, cx| {
                    this.update_select_anchor(anchor, cx);
                });
            },
        )
        .into_any_element()
    }

    /// Detached-window titlebar action. The name predates its only remaining
    /// caller: it styles a titlebar control, not terminal sync groups.
    pub(in crate::workspace) fn terminal_sync_action_button(
        &self,
        label: String,
        enabled: bool,
        listener: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        self.workspace_toolbar_action_button(
            label,
            None,
            oxideterm_gpui_ui::button::ToolbarButtonOptions {
                button: oxideterm_gpui_ui::button::ButtonOptions {
                    variant: oxideterm_gpui_ui::button::ButtonVariant::Ghost,
                    disabled: !enabled,
                    ..Default::default()
                },
                height: Some(22.0),
                padding_x: Some(6.0),
                ..Default::default()
            },
            cx.listener(listener),
        )
    }
}
