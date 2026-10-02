use super::*;

pub(in crate::workspace) const SIDEBAR_RESIZE_DIVIDER_WIDTH: f32 = 1.0;
pub(in crate::workspace) const SIDEBAR_RESIZE_HOTZONE_WIDTH: f32 = 9.0;
const EMBEDDED_SFTP_SPLIT_HANDLE_SIZE: f32 = 7.0;
const EMBEDDED_SFTP_SPLIT_LINE_SIZE: f32 = 1.0;
const EMBEDDED_SFTP_SPLIT_HOVER_ALPHA: u32 = 0x1f;

pub(in crate::workspace) fn context_sidebar_frame_chrome(
    total_width: f32,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id("context-right-sidebar-frame")
        .relative()
        .flex_none()
        .w(px(total_width))
        .h_full()
        .min_w_0()
        .flex()
        .flex_row()
}

pub(in crate::workspace) fn context_sidebar_region_chrome() -> gpui::Div {
    div().relative().flex_1().min_w(px(0.0)).h_full().min_h_0()
}

pub(in crate::workspace) fn sidebar_resize_hotzone_chrome(
    element_id: &'static str,
    line_color: gpui::Rgba,
    divider_at_right: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(element_id)
        .absolute()
        .w(px(SIDEBAR_RESIZE_HOTZONE_WIDTH))
        .cursor_col_resize()
        // Keep the full drag target inside the sidebar so terminal column zero
        // remains selectable even when the terminal has no padding.
        .occlude()
        .bg(rgba(0x00000000))
        .child(
            div()
                .absolute()
                .when(divider_at_right, |divider| divider.right_0())
                .when(!divider_at_right, |divider| divider.left_0())
                .top_0()
                .bottom_0()
                .w(px(SIDEBAR_RESIZE_DIVIDER_WIDTH))
                // Keep the resize cursor when the pointer lands on the painted line itself.
                .cursor_col_resize()
                .bg(line_color),
        )
}

pub(in crate::workspace) fn sidebar_resize_hotzone_origin(seam: f32) -> f32 {
    seam - SIDEBAR_RESIZE_HOTZONE_WIDTH
}

impl WorkspaceApp {
    pub(in crate::workspace) fn render_animated_sidebar_region(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let expanded_width = self.sidebar_panel_width();
        self.sidebar_motion.retarget(if self.sidebar_collapsed {
            0.0
        } else {
            expanded_width
        });
        let content = div()
            .flex_none()
            .w(px(expanded_width))
            .h_full()
            .child(self.render_sidebar_region(window, cx));
        self.sidebar_motion.animate(
            &self.tokens,
            "workspace-left-sidebar-motion",
            div().h_full().flex_none().overflow_hidden().child(content),
            |viewport, width| viewport.w(px(width)),
        )
    }

    pub(in crate::workspace) fn render_animated_context_sidebar_frame(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let expanded_width = self.context_sidebar_width;
        self.context_sidebar_motion
            .retarget(if self.context_sidebar_visible() {
                expanded_width
            } else {
                0.0
            });
        let content = div()
            .flex_none()
            .w(px(expanded_width))
            .h_full()
            .child(self.render_context_right_sidebar_frame(cx));
        self.context_sidebar_motion.animate(
            &self.tokens,
            "workspace-right-sidebar-motion",
            div().h_full().flex_none().overflow_hidden().child(content),
            |viewport, width| viewport.w(px(width)),
        )
    }

    pub(in crate::workspace) fn render_sidebar_region(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .relative()
            .w(px(self.sidebar_panel_width()))
            .h_full()
            .child(self.render_sidebar(window, cx))
            .into_any_element()
    }

    pub(in crate::workspace) fn render_context_right_sidebar_frame(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        context_sidebar_frame_chrome(self.context_sidebar_width)
            .child(self.render_context_right_sidebar_region(cx))
            .into_any_element()
    }

    pub(in crate::workspace) fn render_context_right_sidebar_region(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let (title_key, title_role, icon) = match self.active_context_sidebar_panel {
            ContextSidebarPanel::Assistant => {
                ("sidebar.panels.ai", "assistant", LucideIcon::Sparkles)
            }
            ContextSidebarPanel::HostTools => (
                "sidebar.panels.host_tools",
                "host-tools",
                LucideIcon::Wrench,
            ),
        };
        context_sidebar_region_chrome()
            .child(
                div()
                    .size_full()
                    .min_w_0()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .flex_none()
                            // The right sidebar header sits beside the main
                            // tabbar, so keep both chrome rows exactly aligned.
                            .h(px(self.tokens.metrics.tabbar_height))
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .gap(px(8.0))
                            .px_3()
                            // Match the center tab bar's chrome opacity instead
                            // of inheriting the more transparent sidebar body.
                            .bg(self.workspace_chrome_background(theme.bg))
                            // The context-sidebar titlebar is fixed chrome.
                            // Give it its own hitbox so wheel/drag events
                            // cannot fall through to a scrollable tool body.
                            .occlude()
                            .border_b_1()
                            .border_color(self.workspace_chrome_divider())
                            // Keep the title and collapse button in one real
                            // horizontal flex row. The region width is owned by
                            // the parent frame, so this row must never infer a
                            // smaller hand-derived width from the title text.
                            .child(self.render_context_sidebar_panel_title(
                                title_key, title_role, icon, cx,
                            ))
                            .child(
                                div()
                                    .id("context-sidebar-collapse")
                                    .flex_none()
                                    .size(px(28.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(self.tokens.radii.md))
                                    .cursor_pointer()
                                    .hover(move |button| button.bg(rgb(theme.bg_hover)))
                                    .child(Self::render_lucide_icon(
                                        LucideIcon::PanelRightClose,
                                        self.tokens.metrics.sidebar_collapse_icon_size,
                                        rgb(theme.text_muted),
                                    ))
                                    .on_mouse_move(cx.listener({
                                        let label = self.i18n.t("sidebar.tooltips.collapse");
                                        move |this, event: &MouseMoveEvent, _window, cx| {
                                            this.queue_workspace_tooltip(
                                                "context-sidebar-collapse",
                                                label.clone(),
                                                f32::from(event.position.x) + 12.0,
                                                f32::from(event.position.y) + 16.0,
                                                cx,
                                            );
                                        }
                                    }))
                                    .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                                        if !*hovered {
                                            this.clear_workspace_tooltip(
                                                "context-sidebar-collapse",
                                                cx,
                                            );
                                        }
                                    }))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _event, _window, cx| {
                                            this.collapse_context_sidebar(cx);
                                        }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            // Keep the sidebar tint below the titlebar so the
                            // translucent chrome is composited exactly once.
                            .bg(self.workspace_sidebar_background(theme.bg))
                            .child({
                                let content = match self.active_context_sidebar_panel {
                                    ContextSidebarPanel::Assistant => {
                                        self.render_ai_sidebar_content(cx)
                                    }
                                    ContextSidebarPanel::HostTools => {
                                        self.render_host_tools_context_panel(cx)
                                    }
                                };
                                oxideterm_gpui_ui::motion::fade_in(
                                    &self.tokens,
                                    match self.active_context_sidebar_panel {
                                        ContextSidebarPanel::Assistant => "context-panel-assistant",
                                        ContextSidebarPanel::HostTools => {
                                            "context-panel-host-tools"
                                        }
                                    },
                                    div().size_full().child(content),
                                    oxideterm_gpui_ui::motion::MotionDuration::Micro,
                                )
                            }),
                    ),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn render_left_sidebar_resize_hotzone(
        &mut self,
        top_offset: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let activity_width = self.tokens.metrics.activity_bar_width;
        let hotzone = sidebar_resize_hotzone_chrome(
            "workspace-left-sidebar-resize-hotzone",
            if self.sidebar_resizing {
                rgb(theme.accent)
            } else {
                rgba(0x00000000)
            },
            true,
        )
        .top(px(top_offset))
        .bottom_0()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                this.start_sidebar_resize(event, window, cx);
                window.prevent_default();
                cx.stop_propagation();
            }),
        )
        .on_hover(cx.listener(|this, hovered, _window, cx| {
            if this.sidebar_resize_hotzone_hovered != *hovered {
                this.sidebar_resize_hotzone_hovered = *hovered;
                cx.notify();
            }
        }));
        self.sidebar_motion.animate(
            &self.tokens,
            "left-sidebar-edge-motion",
            hotzone,
            move |edge, width| edge.left(px(sidebar_resize_hotzone_origin(activity_width + width))),
        )
    }

    pub(in crate::workspace) fn render_context_right_sidebar_resize_hotzone(
        &mut self,
        top_offset: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let hotzone = sidebar_resize_hotzone_chrome(
            "context-right-sidebar-resize-hotzone",
            if self.context_sidebar_resizing {
                rgb(theme.accent)
            } else {
                self.workspace_chrome_divider()
            },
            false,
        )
        .top(px(top_offset))
        .bottom_0()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                this.start_ai_sidebar_resize(event, window, cx);
                window.prevent_default();
                cx.stop_propagation();
            }),
        )
        .on_hover(cx.listener(|this, hovered, _window, cx| {
            if this.sidebar_resize_hotzone_hovered != *hovered {
                this.sidebar_resize_hotzone_hovered = *hovered;
                cx.notify();
            }
        }));
        self.context_sidebar_motion.animate(
            &self.tokens,
            "right-sidebar-edge-motion",
            hotzone,
            |edge, width| edge.right(px(sidebar_resize_hotzone_origin(width))),
        )
    }

    pub(in crate::workspace) fn render_context_sidebar_panel_title(
        &self,
        title_key: &'static str,
        title_role: &'static str,
        icon: LucideIcon,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        self.render_window_drag_content_region(
            "context-sidebar-titlebar-title",
            div()
                .w_full()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(Self::render_lucide_icon(icon, 16.0, rgb(theme.accent)))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(theme.text))
                        .child(self.render_display_text_with_role(
                            SelectableTextRole::NonSelectable,
                            "context-sidebar-title",
                            title_role,
                            self.i18n.t(title_key),
                            theme.text,
                            cx,
                        )),
                )
                .into_any_element(),
            cx,
        )
    }

    pub(in crate::workspace) fn render_sidebar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(self.workspace_chrome_divider())
            .child(self.render_sidebar_header(cx))
            .when(
                self.effective_sidebar_panel_section() == SidebarSection::Sessions
                    && self
                        .disclosure_motions
                        .retained("session-search", self.session_search_open),
                |panel| {
                    panel.child(
                        self.disclosure_motions.render(
                            "session-search",
                            &self.tokens,
                            div()
                                .flex_none()
                                .overflow_hidden()
                                .child(self.render_session_search_input(cx)),
                            Some(36.0),
                        ),
                    )
                },
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .flex_col()
                    // Match the activity bar and context sidebar base surface
                    // so the full-height navigation body does not read as a card.
                    .bg(self.workspace_sidebar_background(theme.bg))
                    .child(self.render_sidebar_content(window, cx)),
            )
            .when(
                self.effective_sidebar_panel_section() == SidebarSection::Sessions,
                |sidebar| sidebar.child(self.render_active_sessions_footer(cx)),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn render_sidebar_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.tokens.ui;
        let panel_section = self.effective_sidebar_panel_section();
        let title_key = match panel_section {
            SidebarSection::Forwards => "forwards.table.title",
            _ => "sidebar.panels.sessions",
        };
        let title = self.i18n.t(title_key).to_uppercase();
        let mut header = div()
            // Align sidebar titles with the neighboring workspace tab bar.
            .h(px(self.tokens.metrics.tabbar_height))
            .flex_none()
            .flex()
            .flex_row()
            .items_center()
            // Use the same image-background opacity as the adjacent tab bar
            // without stacking it over the sidebar body's translucent tint.
            .bg(self.workspace_chrome_background(theme.bg))
            .border_b_1()
            .border_color(self.workspace_chrome_divider())
            .px_2()
            .child(
                self.render_window_drag_content_region(
                    "sidebar-header-title-drag-region",
                    div()
                        .flex()
                        .items_center()
                        .truncate()
                        .text_size(px(self.tokens.metrics.sidebar_title_font_size))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(theme.text_muted))
                        // A title inside the native drag region must never start
                        // read-only text selection or retain pointer ownership.
                        .child(self.render_display_text_with_role(
                            SelectableTextRole::NonSelectable,
                            "sidebar-header-title",
                            title_key,
                            title,
                            theme.text_muted,
                            cx,
                        ))
                        .into_any_element(),
                    cx,
                ),
            );
        if panel_section == SidebarSection::Sessions {
            let (view_icon, view_action) = match self.active_session_sidebar_view_mode {
                ActiveSessionSidebarViewMode::Tree => {
                    (LucideIcon::Folder, SidebarActionKind::ToggleSessionView)
                }
                ActiveSessionSidebarViewMode::Focus => {
                    (LucideIcon::ListChecks, SidebarActionKind::ToggleSessionView)
                }
            };
            header = header
                .child(self.render_session_search_button(cx))
                .child(self.render_session_sort_button(cx))
                .child(self.render_sidebar_action(view_icon, view_action, cx))
                .child(self.render_sidebar_action(
                    LucideIcon::Plus,
                    SidebarActionKind::NewConnection,
                    cx,
                ));
        }
        header.into_any_element()
    }

    pub(in crate::workspace) fn render_sidebar_action(
        &self,
        icon: LucideIcon,
        action: SidebarActionKind,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let label = match action {
            SidebarActionKind::ToggleSessionView => match self.active_session_sidebar_view_mode {
                ActiveSessionSidebarViewMode::Tree => self.i18n.t("sidebar.tooltips.switch_focus"),
                ActiveSessionSidebarViewMode::Focus => self.i18n.t("sidebar.tooltips.switch_tree"),
            },
            SidebarActionKind::NewConnection => self.i18n.t("sidebar.tooltips.new_connection"),
        };

        let toggle_focus_active = action == SidebarActionKind::ToggleSessionView
            && self.active_session_sidebar_view_mode == ActiveSessionSidebarViewMode::Focus;

        // Tauri sidebar header actions are icon buttons with title tooltips.
        // The view-mode action is the old Folder/ListChecks toggle from
        // Sidebar.tsx; keep its active "secondary" chrome only in focus mode.
        div()
            .ml_1()
            .child(self.workspace_tooltip_icon_button(
                icon,
                self.tokens.metrics.sidebar_action_icon_size,
                rgb(theme.text),
                IconButtonOptions {
                    has_background: toggle_focus_active,
                    background: toggle_focus_active.then_some(rgb(theme.bg_hover)),
                    hover_background: Some(rgb(theme.bg_hover)),
                    ..IconButtonOptions::opaque_toolbar(
                        self.tokens.metrics.sidebar_action_size,
                        ButtonRadius::Md,
                    )
                },
                label,
                "sidebar-action",
                false,
                cx.listener(move |this, _event, window, cx| {
                    match action {
                        SidebarActionKind::ToggleSessionView => {
                            this.toggle_active_session_sidebar_view(cx)
                        }
                        SidebarActionKind::NewConnection => {
                            this.open_new_connection_form(window, cx)
                        }
                    }
                    cx.stop_propagation();
                }),
                cx.entity(),
            ))
            .into_any_element()
    }

    pub(in crate::workspace) fn render_sidebar_content(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let panel_section = self.effective_sidebar_panel_section();
        if panel_section == SidebarSection::Sessions {
            let sessions = self.render_active_sessions_sidebar_content(cx);
            if self.embedded_sftp_node_id.is_none() {
                return sessions;
            }
            let sftp = self.render_sftp_sidebar_surface(window, cx);
            let session_fraction = self
                .settings_store
                .settings()
                .sftp
                .sidebar_session_fraction
                .clamp(
                    EMBEDDED_SFTP_MIN_SESSION_FRACTION,
                    EMBEDDED_SFTP_MAX_SESSION_FRACTION,
                );
            let file_fraction = 1.0 - session_fraction;
            let split_line_top =
                (EMBEDDED_SFTP_SPLIT_HANDLE_SIZE - EMBEDDED_SFTP_SPLIT_LINE_SIZE) / 2.0;
            let split_line_color = if self.embedded_sftp_sidebar_resizing {
                rgb(self.tokens.ui.accent)
            } else {
                rgb(self.tokens.ui.border)
            };
            let split_hover_bg =
                rgba((self.tokens.ui.accent << 8) | EMBEDDED_SFTP_SPLIT_HOVER_ALPHA);
            // Embedded SFTP is a subordinate surface of Active Sessions. The
            // upper navigator and lower file browser scroll independently,
            // while the divider keeps them visually in one sidebar shell.
            return div()
                .flex_1()
                .min_h(px(0.0))
                .w_full()
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(
                    div()
                        .flex_grow_0()
                        .flex_shrink_1()
                        .flex_basis(relative(session_fraction))
                        .min_h(px(0.0))
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .child(sessions),
                )
                .child(
                    div()
                        .id("embedded-sftp-sidebar-splitter")
                        .flex_none()
                        .relative()
                        .h(px(EMBEDDED_SFTP_SPLIT_HANDLE_SIZE))
                        .w_full()
                        .cursor(CursorStyle::ResizeRow)
                        .hover(move |handle| handle.bg(split_hover_bg))
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(px(split_line_top))
                                .h(px(EMBEDDED_SFTP_SPLIT_LINE_SIZE))
                                .bg(split_line_color),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseDownEvent, window, cx| {
                                this.start_embedded_sftp_sidebar_resize(event, window, cx);
                                window.prevent_default();
                                cx.stop_propagation();
                            }),
                        ),
                )
                .child(
                    div()
                        .flex_grow_0()
                        .flex_shrink_1()
                        .flex_basis(relative(file_fraction))
                        .min_h(px(0.0))
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .child(sftp),
                )
                .into_any_element();
        }
        if panel_section == SidebarSection::Forwards {
            // Tauri only persists these command-palette section keys here; it
            // does not reuse the Sessions empty state for their sidebar body.
            return self.render_blank_sidebar_content();
        }
        self.render_empty_sessions_sidebar_content(cx)
    }

    pub(in crate::workspace) fn render_blank_sidebar_content(&self) -> AnyElement {
        div().flex_1().w_full().into_any_element()
    }

    pub(in crate::workspace) fn render_empty_sessions_sidebar_content(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        div()
            .flex_1()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .px(px(self.tokens.metrics.empty_sidebar_padding_x))
            .text_color(rgb(theme.text_muted))
            .child(
                div()
                    .w_full()
                    .h(px(self.tokens.metrics.empty_sidebar_height))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .child(div().mb_3().child(Self::render_lucide_icon(
                        LucideIcon::Server,
                        self.tokens.metrics.empty_sidebar_icon_size,
                        rgba((theme.text_muted << 8) | 0x4d),
                    )))
                    .child(
                        div()
                            .w_full()
                            .text_center()
                            .text_size(px(self.tokens.metrics.empty_sidebar_title_font_size))
                            .text_color(rgb(theme.text_muted))
                            // Empty-state guidance is interface chrome, not
                            // user data that should start a text selection.
                            .child(self.render_display_text_with_role(
                                SelectableTextRole::NonSelectable,
                                "sessions-sidebar-empty-title",
                                (),
                                self.i18n.t("sessions.tree.no_sessions"),
                                theme.text_muted,
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .mt_1()
                            .w_full()
                            .text_center()
                            .text_size(px(self.tokens.metrics.empty_sidebar_subtitle_font_size))
                            .text_color(rgb(theme.text_muted))
                            .child(self.render_display_text_with_role(
                                SelectableTextRole::NonSelectable,
                                "sessions-sidebar-empty-subtitle",
                                (),
                                self.i18n.t("sessions.tree.click_to_add"),
                                theme.text_muted,
                                cx,
                            )),
                    ),
            )
            .into_any_element()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::workspace) enum SidebarActionKind {
    ToggleSessionView,
    NewConnection,
}

#[cfg(test)]
mod sidebar_resize_region_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        Context, CursorStyle, IntoElement, Modifiers, MouseButton, ParentElement, Point, Render,
        Styled, TestAppContext, Window, canvas, div, px, size,
    };

    struct TestContextSidebarChrome {
        total_width: f32,
        resize_started: Rc<Cell<bool>>,
        resize_moved: Rc<Cell<bool>>,
        resizing: bool,
    }

    struct TestLeftSidebarChrome {
        total_width: f32,
        resize_started: Rc<Cell<bool>>,
        hotzone_hovered: bool,
    }

    impl Render for TestLeftSidebarChrome {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let resize_started = self.resize_started.clone();
            div()
                .relative()
                .size_full()
                .child(
                    div()
                        .w(px(self.total_width))
                        .h_full()
                        .debug_selector(|| "left-frame".to_string())
                        // Simulate loaded sidebar content owning the full visible surface.
                        .child(div().absolute().size_full().occlude())
                        // Simulate a custom-painted child requesting a window-wide cursor.
                        .child(canvas(
                            |_, _, _| (),
                            |_, _, window, _| {
                                window.set_window_cursor_style(CursorStyle::Arrow);
                            },
                        )),
                )
                .child(
                    sidebar_resize_hotzone_chrome("left-hotzone-element", rgba(0x000000ff), true)
                        .left(px(sidebar_resize_hotzone_origin(self.total_width)))
                        .top_0()
                        .bottom_0()
                        .debug_selector(|| "left-hotzone".to_string())
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |_, _event, _window, _cx| {
                                resize_started.set(true);
                            }),
                        )
                        .on_hover(cx.listener(|this, hovered, _window, cx| {
                            if this.hotzone_hovered != *hovered {
                                this.hotzone_hovered = *hovered;
                                cx.notify();
                            }
                        })),
                )
                .when(self.hotzone_hovered, |root| {
                    root.child(canvas(
                        |_, _, _| (),
                        |_, _, window, _| {
                            // The resize handle must beat earlier window-wide cursor requests.
                            window.set_window_cursor_style(CursorStyle::ResizeColumn);
                        },
                    ))
                })
        }
    }

    impl Render for TestContextSidebarChrome {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let resize_started = self.resize_started.clone();
            let resize_moved = self.resize_moved.clone();
            let seam = f32::from(window.viewport_size().width) - self.total_width;
            div()
                .relative()
                .size_full()
                .flex()
                .justify_end()
                .child(
                    context_sidebar_frame_chrome(self.total_width)
                        .debug_selector(|| "context-frame".to_string())
                        .child(
                            context_sidebar_region_chrome()
                                .debug_selector(|| "context-region".to_string())
                                .child(
                                    div()
                                        .size_full()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        // Simulate loaded Host Tools content owning a blocking hitbox.
                                        .child(div().absolute().size_full().occlude())
                                        .child(
                                            div()
                                                .w_full()
                                                .min_w(px(0.0))
                                                .flex_none()
                                                .h(px(42.0))
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .justify_between()
                                                .gap(px(8.0))
                                                .px_3()
                                                .debug_selector(|| "context-titlebar".to_string())
                                                .child(
                                                    div()
                                                        .h_full()
                                                        .flex_1()
                                                        .min_w(px(0.0))
                                                        .debug_selector(|| {
                                                            "context-title-drag".to_string()
                                                        }),
                                                )
                                                .child(
                                                    div()
                                                        .flex_none()
                                                        .size(px(28.0))
                                                        .debug_selector(|| {
                                                            "context-collapse".to_string()
                                                        }),
                                                ),
                                        ),
                                ),
                        ),
                )
                .child(
                    sidebar_resize_hotzone_chrome(
                        "context-hotzone-element",
                        rgba(0x000000ff),
                        false,
                    )
                    .left(px(seam))
                    .top_0()
                    .bottom_0()
                    .debug_selector(|| "context-hotzone".to_string())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event, _window, cx| {
                            this.resizing = true;
                            resize_started.set(true);
                            cx.notify();
                        }),
                    ),
                )
                .when(self.resizing, |root| {
                    root.child(
                        div()
                            .absolute()
                            .size_full()
                            .occlude()
                            .on_mouse_move(cx.listener(
                                move |this, event: &MouseMoveEvent, window, cx| {
                                    // Root capture owns movement after the pointer leaves the hotzone.
                                    this.total_width = (f32::from(window.viewport_size().width)
                                        - f32::from(event.position.x))
                                    .max(0.0);
                                    resize_moved.set(true);
                                    cx.notify();
                                },
                            )),
                    )
                })
        }
    }

    pub(in crate::workspace) fn right_edge(bounds: &gpui::Bounds<gpui::Pixels>) -> f32 {
        f32::from(bounds.origin.x) + f32::from(bounds.size.width)
    }

    pub(in crate::workspace) fn assert_close(label: &str, actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.5,
            "{label}: expected {expected}, got {actual}"
        );
    }

    #[gpui::test]
    pub(in crate::workspace) fn left_sidebar_resize_hotzone_overlays_loaded_content(
        cx: &mut TestAppContext,
    ) {
        let total_width = 280.0;
        let resize_started = Rc::new(Cell::new(false));
        let (_, cx) = cx.add_window_view(|_, _| TestLeftSidebarChrome {
            total_width,
            resize_started: resize_started.clone(),
            hotzone_hovered: false,
        });
        cx.simulate_resize(size(px(700.0), px(180.0)));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });

        let frame = cx.debug_bounds("left-frame").expect("left frame bounds");
        let hotzone = cx
            .debug_bounds("left-hotzone")
            .expect("left hotzone bounds");
        assert_close("left frame width", f32::from(frame.size.width), total_width);
        assert_close(
            "left hotzone width",
            f32::from(hotzone.size.width),
            SIDEBAR_RESIZE_HOTZONE_WIDTH,
        );
        assert_close(
            "left hotzone ends before terminal content",
            right_edge(&hotzone),
            right_edge(&frame),
        );

        cx.simulate_mouse_down(
            Point::new(
                frame.origin.x + frame.size.width + px(0.5),
                frame.origin.y + px(20.0),
            ),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert!(
            !resize_started.get(),
            "column zero must not start sidebar resize"
        );
        cx.simulate_mouse_move(
            Point::new(
                frame.origin.x + frame.size.width - px(3.0),
                frame.origin.y + px(20.0),
            ),
            None,
            Modifiers::default(),
        );
        assert_eq!(
            cx.update(|window, _cx| window.cursor_style_for_test()),
            CursorStyle::ResizeColumn,
            "hovering the left resize hotzone should apply the column-resize cursor"
        );
        cx.simulate_mouse_down(
            Point::new(
                frame.origin.x + frame.size.width - px(3.0),
                frame.origin.y + px(20.0),
            ),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert!(
            resize_started.get(),
            "left resize hotzone should receive mouse down above loaded content"
        );
    }

    #[gpui::test]
    pub(in crate::workspace) fn context_sidebar_resize_hotzone_has_no_gap_after_content_load(
        cx: &mut TestAppContext,
    ) {
        let total_width = 620.0;
        let resize_started = Rc::new(Cell::new(false));
        let resize_moved = Rc::new(Cell::new(false));

        let (_, cx) = cx.add_window_view(|_, _| TestContextSidebarChrome {
            total_width,
            resize_started: resize_started.clone(),
            resize_moved: resize_moved.clone(),
            resizing: false,
        });
        cx.simulate_resize(size(px(700.0), px(180.0)));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });

        let frame = cx.debug_bounds("context-frame").expect("frame bounds");
        let region = cx.debug_bounds("context-region").expect("region bounds");
        let titlebar = cx
            .debug_bounds("context-titlebar")
            .expect("titlebar bounds");
        let collapse = cx
            .debug_bounds("context-collapse")
            .expect("collapse bounds");
        let hotzone = cx.debug_bounds("context-hotzone").expect("hotzone bounds");

        assert_close("frame width", f32::from(frame.size.width), total_width);
        assert_close(
            "region origin",
            f32::from(region.origin.x) - f32::from(frame.origin.x),
            0.0,
        );
        assert_close("region width", f32::from(region.size.width), total_width);
        assert_close(
            "titlebar width",
            f32::from(titlebar.size.width),
            f32::from(region.size.width),
        );

        // The collapse control should be at the right chrome edge, allowing for
        // the titlebar padding. This catches regressions where the titlebar row
        // shrinks to the intrinsic "OxideSens" title width.
        let right_padding = right_edge(&titlebar) - right_edge(&collapse);
        assert_close("collapse right padding", right_padding, 12.0);

        assert_close(
            "hotzone origin",
            f32::from(hotzone.origin.x) - f32::from(frame.origin.x),
            0.0,
        );
        assert_close(
            "hotzone width",
            f32::from(hotzone.size.width),
            SIDEBAR_RESIZE_HOTZONE_WIDTH,
        );

        cx.simulate_mouse_down(
            Point::new(frame.origin.x - px(0.5), frame.origin.y + px(20.0)),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert!(
            !resize_started.get(),
            "terminal edge must not start sidebar resize"
        );
        cx.simulate_mouse_move(
            Point::new(frame.origin.x + px(3.0), frame.origin.y + px(20.0)),
            None,
            Modifiers::default(),
        );
        assert_eq!(
            cx.update(|window, _cx| window.cursor_style_for_test()),
            CursorStyle::ResizeColumn,
            "hovering the context-sidebar hotzone should apply the column-resize cursor"
        );
        cx.simulate_mouse_down(
            Point::new(frame.origin.x + px(3.0), frame.origin.y + px(20.0)),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert!(
            resize_started.get(),
            "frame-local resize hotzone should receive mouse down above loaded content"
        );
        cx.simulate_mouse_move(
            Point::new(frame.origin.x - px(40.0), frame.origin.y + px(20.0)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        assert!(
            resize_moved.get(),
            "root capture should continue the frame-local hotzone drag"
        );
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        let resized_frame = cx
            .debug_bounds("context-frame")
            .expect("resized frame bounds");
        assert_close(
            "resized frame width",
            f32::from(resized_frame.size.width),
            660.0,
        );
    }
}
