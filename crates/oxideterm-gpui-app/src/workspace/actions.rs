use super::ime::WorkspaceImeTarget;
use super::tabs::TabCloseConfirmKeyAction;
use super::*;
use oxideterm_gpui_ui::text_input::{text_caret, text_input_value_segments_with_color};
use oxideterm_quick_commands::{
    QuickCommandRisk, classify_command_risk as classify_quick_command_risk,
};
use zeroize::Zeroizing;

const TERMINAL_FONT_SIZE_MIN: i64 = 8;
const TERMINAL_FONT_SIZE_MAX: i64 = 32;
const TERMINAL_FONT_SIZE_DEFAULT: i64 = 14;
const TERMINAL_FONT_SIZE_HUD_DURATION: Duration = Duration::from_millis(1200);

fn adjusted_terminal_font_size(current: i64, delta: i64) -> Option<i64> {
    let next = (current + delta).clamp(TERMINAL_FONT_SIZE_MIN, TERMINAL_FONT_SIZE_MAX);
    (next != current).then_some(next)
}

#[derive(Default)]
pub(super) struct TerminalSearchState {
    pub(super) panes: HashMap<PaneId, SearchBarState>,
    pub(super) focused: Option<PaneId>,
}

impl TerminalSearchState {
    fn open(&mut self, pane_id: PaneId) {
        self.panes.entry(pane_id).or_default().visible = true;
        self.focused = Some(pane_id);
    }

    pub(super) fn blur(&mut self) -> bool {
        self.focused.take().is_some()
    }

    fn close(&mut self, pane_id: PaneId) -> bool {
        if let Some(search) = self.panes.get_mut(&pane_id) {
            search.visible = false;
            search.clear_match_state();
        }
        self.focused == Some(pane_id) && self.blur()
    }

    pub(super) fn replace_query(
        &mut self,
        pane_id: PaneId,
        range: Option<std::ops::Range<usize>>,
        text: &str,
    ) -> bool {
        let Some(search) = self.panes.get_mut(&pane_id).filter(|search| search.visible) else {
            return false;
        };
        oxideterm_editor_core::utf16::replace_utf16(&mut search.query, range, text);
        true
    }

    pub(super) fn remove(&mut self, pane_id: PaneId) {
        self.panes.remove(&pane_id);
        if self.focused == Some(pane_id) {
            self.focused = None;
        }
    }
}

#[derive(Default)]
pub(super) struct SearchBarState {
    pub(super) visible: bool,
    pub(super) query: String,
    pub(super) active_match: Option<usize>,
    pub(super) match_count: usize,
}

impl SearchBarState {
    pub(super) fn sync_from_terminal(&mut self, status: TerminalSearchStatus) {
        self.active_match = status.active_match;
        self.match_count = status.match_count;
    }

    fn clear_match_state(&mut self) {
        self.active_match = None;
        self.match_count = 0;
    }
}

fn terminal_tab_capture_keystroke(keystroke: &gpui::Keystroke) -> bool {
    let modifiers = keystroke.modifiers;
    // Plain Tab and Shift+Tab are terminal protocol keys, but some platforms
    // also treat them as focus traversal keys. Capture only that collision;
    // Ctrl+Tab and other chords stay owned by the normal keybinding registry.
    keystroke.key.as_str() == "tab" && !modifiers.platform && !modifiers.control && !modifiers.alt
}

#[cfg(test)]
mod terminal_search_tests {
    use super::*;

    #[test]
    fn pane_searches_keep_their_queries_and_matches_across_focus_and_close() {
        let mut searches = TerminalSearchState::default();
        let first = PaneId(10);
        let second = PaneId(20);
        searches.open(first);
        searches.replace_query(first, None, "错误🦀");
        searches
            .panes
            .get_mut(&first)
            .unwrap()
            .sync_from_terminal(TerminalSearchStatus {
                query: Some("错误🦀".into()),
                active_match: Some(2),
                match_count: 4,
            });
        searches.blur();
        assert!(searches.panes[&first].visible);
        assert_eq!(searches.focused, None);
        searches.open(second);
        searches.replace_query(second, None, "warning");
        searches
            .panes
            .get_mut(&second)
            .unwrap()
            .sync_from_terminal(TerminalSearchStatus {
                query: Some("warning".into()),
                active_match: Some(0),
                match_count: 1,
            });
        searches.open(first);
        assert_eq!(
            (
                &*searches.panes[&first].query,
                searches.panes[&first].active_match,
                searches.panes[&first].match_count
            ),
            ("错误🦀", Some(2), 4)
        );
        searches.replace_query(first, Some(2..4), "日志");
        assert_eq!(searches.panes[&first].query, "错误日志");
        assert_eq!(
            (
                &*searches.panes[&second].query,
                searches.panes[&second].active_match,
                searches.panes[&second].match_count
            ),
            ("warning", Some(0), 1)
        );
        searches.close(first);
        assert!(!searches.replace_query(first, None, "late input"));
        searches.open(first);
        assert_eq!(searches.panes[&first].query, "错误日志");
        searches.remove(first);
        assert_eq!(searches.focused, None);
        assert_eq!(
            searches.panes.keys().copied().collect::<Vec<_>>(),
            vec![second]
        );
    }
}

fn terminal_tab_capture_blocked_by_workspace_ui(active_ime_target: bool) -> bool {
    // Text inputs and command palettes own Tab semantics while they are active.
    // The terminal fallback only handles the platform focus-traversal path that
    // would otherwise swallow a real terminal Tab.
    active_ime_target
}

impl WorkspaceApp {
    pub(in crate::workspace) fn begin_node_disconnect_confirm_exit(
        &mut self,
        confirmed: bool,
        cx: &mut Context<Self>,
    ) -> (bool, Option<WorkspaceOverlayConfirmEffect>) {
        let delay = oxideterm_gpui_ui::motion::duration(
            &self.tokens,
            oxideterm_gpui_ui::motion::MotionDuration::Control,
        );
        self.overlay.update(cx, |overlay, cx| {
            overlay.begin_confirm_exit(confirmed, delay, cx)
        })
    }

    pub(in crate::workspace) fn begin_tab_close_confirm_exit(
        &mut self,
        confirmed: bool,
        cx: &mut Context<Self>,
    ) -> (bool, Option<TabCloseConfirm>) {
        let delay = oxideterm_gpui_ui::motion::duration(
            &self.tokens,
            oxideterm_gpui_ui::motion::MotionDuration::Control,
        );
        self.tab_host.update(cx, |tab_host, cx| {
            tab_host.begin_close_confirm_exit(confirmed, delay, cx)
        })
    }

    pub(in crate::workspace) fn begin_keybinding_reset_all_confirm_exit(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let delay = oxideterm_gpui_ui::motion::duration(
            &self.tokens,
            oxideterm_gpui_ui::motion::MotionDuration::Control,
        );
        self.settings_workspace.update(cx, |settings, cx| {
            settings.begin_keybinding_reset_confirm_exit(delay, cx)
        })
    }
    pub(super) fn search_visible(&self, cx: &App) -> bool {
        self.active_pane_id(cx)
            .and_then(|id| self.search.panes.get(&id))
            .is_some_and(|search| search.visible)
    }

    pub(super) fn focused_search_pane(&self, cx: &App) -> Option<PaneId> {
        let pane_id = self.search.focused?;
        if !self
            .search
            .panes
            .get(&pane_id)
            .is_some_and(|search| search.visible)
        {
            return None;
        }
        let host = self.tab_host.read(cx);
        let tab = host
            .tabs()
            .iter()
            .find(|tab| tab.active_pane_id == Some(pane_id))?;
        if self.active_tab_id(cx) != Some(tab.id) && !host.is_outside_main_window(tab.id) {
            return None;
        }
        let owner = host.detached_window_handle(tab.id).or_else(|| {
            self.window_registry
                .handle_for_role(window_registry::WindowRole::Main)
        });
        // A remembered search focus in another native window cannot claim this window's keys.
        if cx
            .active_window()
            .is_some_and(|active| owner.is_none_or(|owner| owner.window_id() != active.window_id()))
        {
            return None;
        }
        Some(pane_id)
    }

    fn focus_search_pane(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        let tab_id = self
            .tabs(cx)
            .iter()
            .find(|tab| {
                tab.root_pane
                    .as_ref()
                    .is_some_and(|root| root.contains_pane(pane_id))
            })
            .map(|tab| tab.id);
        let Some(tab_id) = tab_id else {
            return;
        };
        self.blur_text_inputs(cx);
        self.tab_host
            .update(cx, |host, _| host.set_active_pane(Some(tab_id), pane_id));
        if !self.tab_host.read(cx).is_outside_main_window(tab_id) {
            self.set_main_window_active_tab(Some(tab_id), cx);
        }
        self.search.focused = Some(pane_id);
    }

    pub(super) fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pane_id) = self.active_pane_id(cx) else {
            return;
        };
        self.open_search_for_pane(pane_id, window, cx);
    }

    pub(super) fn open_search_for_pane(
        &mut self,
        pane_id: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_search_pane(pane_id, cx);
        self.search.open(pane_id);
        window.focus(&self.focus_handle, cx);
        self.update_search_query_for_pane(pane_id, false, cx);
        self.select_all_active_text_input(cx);
        cx.notify();
    }

    pub(super) fn hide_search(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        if self.search.close(pane_id) {
            self.ime_marked_text = None;
            self.clear_ime_selection();
        }
        if let Some(pane) = self.tab_host.read(cx).panes().get(&pane_id).cloned() {
            pane.update(cx, |pane, cx| pane.set_search_query(None, None, cx));
        }
        cx.notify();
    }

    pub(super) fn close_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.active_pane_id(cx) {
            self.hide_search(id, cx);
        }
        self.focus_active_pane(window, cx);
    }

    pub(super) fn update_search_query_for_pane(
        &mut self,
        pane_id: PaneId,
        reset_match: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(search) = self.search.panes.get_mut(&pane_id) else {
            return;
        };
        let query = (!search.query.is_empty()).then(|| search.query.clone());
        if reset_match {
            search.active_match = query.as_ref().map(|_| 0);
        }
        if let Some(pane) = self.tab_host.read(cx).panes().get(&pane_id).cloned() {
            let status = pane.read(cx).search_status();
            let status = if !reset_match && status.query == query {
                status
            } else {
                pane.update(cx, |pane, cx| {
                    pane.set_search_query(query, search.active_match, cx)
                })
            };
            search.sync_from_terminal(status);
        }
        cx.notify();
    }

    pub(super) fn search_next(&mut self, forward: bool, cx: &mut Context<Self>) {
        if let Some(id) = self.active_pane_id(cx) {
            self.search_next_for_pane(id, forward, cx);
        }
    }

    pub(super) fn search_next_for_pane(
        &mut self,
        pane_id: PaneId,
        forward: bool,
        cx: &mut Context<Self>,
    ) {
        if let Some(pane) = self.tab_host.read(cx).panes().get(&pane_id).cloned() {
            let status = pane.update(cx, |pane, cx| pane.select_next_search_result(forward, cx));
            if let Some(search) = self.search.panes.get_mut(&pane_id) {
                search.sync_from_terminal(status);
            }
            cx.notify();
        }
    }

    pub(super) fn copy(&mut self, cx: &mut Context<Self>) {
        if self.copy_remote_desktop(cx) {
            return;
        }
        if let Some(pane) = self.active_pane(cx) {
            let _ = pane.update(cx, |pane, cx| pane.copy_to_clipboard(cx));
        }
    }

    pub(super) fn paste(&mut self, cx: &mut Context<Self>) {
        if self.paste_remote_desktop(cx) {
            return;
        }
        if let Some(pane) = self.active_pane(cx) {
            let _ = pane.update(cx, |pane, cx| pane.paste_from_clipboard(cx));
        }
    }

    pub(super) fn clear_active_terminal_screen(&mut self, cx: &mut Context<Self>) -> bool {
        let terminal_active = self.active_tab(cx).is_some_and(|tab| {
            matches!(
                tab.kind,
                TabKind::LocalTerminal
                    | TabKind::SshTerminal
                    | TabKind::MoshTerminal
                    | TabKind::Workspace
            )
        });
        if !terminal_active {
            return false;
        }
        let Some(pane) = self.active_pane(cx) else {
            return false;
        };
        // Clear host-owned emulator state without writing control bytes into PTYs or serial
        // links.
        pane.update(cx, |pane, cx| pane.clear_buffer(cx));
        true
    }

    pub(super) fn cut(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(pane) = self.active_pane(cx) else {
            return false;
        };
        pane.update(cx, |pane, cx| pane.cut_to_clipboard(cx))
    }

    pub(super) fn toggle_zen_mode(&mut self, cx: &mut Context<Self>) {
        let settings = self.settings_store.settings_mut();
        let entering = !settings.sidebar_ui.zen_mode;
        settings.sidebar_ui.zen_mode = entering;
        if entering {
            // Zen mode hides chrome without changing the user's sidebar visibility choices.
            self.sidebar_motion.settle(0.0);
            self.context_sidebar_motion.settle(0.0);
            self.sidebar_motion_generation = self.sidebar_motion_generation.wrapping_add(1);
            self.context_sidebar_motion_generation =
                self.context_sidebar_motion_generation.wrapping_add(1);
            self.sidebar_rendered = false;
            self.context_sidebar_rendered = false;
            const ZEN_HINT_TTL: Duration = Duration::from_millis(2500);
            self.apply_workspace_overlay_intent(
                WorkspaceOverlayIntent::ShowZenHint { ttl: ZEN_HINT_TTL },
                cx,
            );
        } else {
            self.sidebar_motion_generation = self.sidebar_motion_generation.wrapping_add(1);
            self.context_sidebar_motion_generation =
                self.context_sidebar_motion_generation.wrapping_add(1);
            self.sidebar_rendered = !self.sidebar_collapsed;
            self.context_sidebar_rendered = self.context_sidebar_visible();
            self.apply_workspace_overlay_intent(WorkspaceOverlayIntent::ClearZenHint, cx);
        }
        cx.notify();
    }

    pub(super) fn adjust_terminal_font_size(&mut self, delta: i64, cx: &mut Context<Self>) {
        let current = self.settings_store.settings().terminal.font_size;
        let Some(next) = adjusted_terminal_font_size(current, delta) else {
            return;
        };
        self.edit_settings(|settings| settings.terminal.font_size = next, cx);
        self.show_terminal_font_size_hud(next, cx);
    }

    pub(super) fn reset_terminal_font_size(&mut self, cx: &mut Context<Self>) {
        self.edit_settings(
            |settings| settings.terminal.font_size = TERMINAL_FONT_SIZE_DEFAULT,
            cx,
        );
        self.show_terminal_font_size_hud(TERMINAL_FONT_SIZE_DEFAULT, cx);
    }

    fn show_terminal_font_size_hud(&mut self, font_size: i64, cx: &mut Context<Self>) {
        self.apply_workspace_overlay_intent(
            WorkspaceOverlayIntent::ShowTerminalFontSizeHud {
                font_size,
                ttl: TERMINAL_FONT_SIZE_HUD_DURATION,
            },
            cx,
        );
    }

    pub(super) fn dispatch_registered_keybinding(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((definition, combo)) = crate::keybindings::matched_action_for_keystroke(
            &event.keystroke,
            &self.settings_store.settings().keybindings.overrides,
        ) else {
            return false;
        };

        let terminal_active = self.active_tab(cx).is_some_and(|tab| {
            matches!(
                tab.kind,
                TabKind::LocalTerminal
                    | TabKind::SshTerminal
                    | TabKind::MoshTerminal
                    | TabKind::Workspace
            )
        });
        if matches!(
            definition.scope,
            crate::keybindings::ActionScope::Terminal | crate::keybindings::ActionScope::Split
        ) && !terminal_active
        {
            return false;
        }

        let terminal_panel_open =
            self.focused_search_pane(cx).is_some() || self.context_sidebar_visible();
        if !crate::keybindings::action_allowed_by_terminal_behavior(
            definition,
            &combo,
            terminal_active,
            terminal_panel_open,
        ) {
            return false;
        }

        self.dispatch_keybinding_action(&definition.id, window, cx)
    }

    pub(super) fn dispatch_keybinding_action(
        &mut self,
        action_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match action_id {
            "app.newTerminal" => {
                let _ = self.create_local_terminal_tab(window, cx);
            }
            "app.shellLauncher" => self.open_local_shell_launcher(cx),
            "app.closeTab" => self.request_close_active_tab(window, cx),
            "app.closeOtherTabs" => self.request_close_other_tabs_or_active_pane(window, cx),
            "app.newConnection" => self.open_new_connection_form(window, cx),
            "app.settings" => self.open_settings(window, cx),
            "app.toggleSidebar" => self.toggle_sidebar(cx),
            "app.commandPalette" => self.open_command_palette(window, cx),
            "app.zenMode" => self.toggle_zen_mode(cx),
            "app.nextTab" => self.next_tab(true, window, cx),
            "app.prevTab" => self.next_tab(false, window, cx),
            "app.navBack" => self.navigate_tab_history(false, window, cx),
            "app.navForward" => self.navigate_tab_history(true, window, cx),
            "app.goToTab1" => self.go_to_tab(0, window, cx),
            "app.goToTab2" => self.go_to_tab(1, window, cx),
            "app.goToTab3" => self.go_to_tab(2, window, cx),
            "app.goToTab4" => self.go_to_tab(3, window, cx),
            "app.goToTab5" => self.go_to_tab(4, window, cx),
            "app.goToTab6" => self.go_to_tab(5, window, cx),
            "app.goToTab7" => self.go_to_tab(6, window, cx),
            "app.goToTab8" => self.go_to_tab(7, window, cx),
            "app.goToTab9" => self.go_to_tab(8, window, cx),
            "app.fontIncrease" => self.adjust_terminal_font_size(1, cx),
            "app.fontDecrease" => self.adjust_terminal_font_size(-1, cx),
            "app.fontReset" => self.reset_terminal_font_size(cx),
            "app.showShortcuts" => self.open_shortcuts_modal(cx),
            "terminal.search" => self.open_search(window, cx),
            "terminal.copy" => self.copy(cx),
            "terminal.cut" => {
                let _ = self.cut(cx);
            }
            "terminal.paste" => self.paste(cx),
            "terminal.clearScreen" => {
                self.clear_active_terminal_screen(cx);
            }
            "terminal.toggleFreeTypeMode" => self.toggle_free_type_mode(cx),
            "terminal.closePanel" => self.close_terminal_panel(window, cx),
            "split.horizontal" => self.split_active_pane(SplitDirection::Horizontal, window, cx),
            "split.vertical" => self.split_active_pane(SplitDirection::Vertical, window, cx),
            "split.closePane" => self.close_active_pane(window, cx),
            "split.navLeft" => self.focus_adjacent_pane(false, window, cx),
            "split.navRight" => self.focus_adjacent_pane(true, window, cx),
            _ => return false,
        }
        true
    }

    pub(super) fn toggle_free_type_mode(&mut self, cx: &mut Context<Self>) {
        let enabled = !self.settings_store.settings().terminal.free_type_mode;
        // Route through the shared settings path so every open terminal receives
        // the new preference without changing terminal or SSH session ownership.
        self.edit_settings(|settings| settings.terminal.free_type_mode = enabled, cx);
        self.push_command_palette_toast(
            self.i18n.t("settings_view.terminal.free_type_mode"),
            Some(self.i18n.t(if enabled {
                "common.enabled"
            } else {
                "common.disabled"
            })),
            TerminalNoticeVariant::Default,
            cx,
        );
    }

    fn close_terminal_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_terminal_command_overlays(cx) {
            return;
        }
        if self.search_visible(cx) {
            self.close_search(window, cx);
            return;
        }
        if self.context_sidebar_visible() {
            self.collapse_context_sidebar(cx);
            self.focus_active_pane(window, cx);
        }
    }

    pub(in crate::workspace) fn close_terminal_command_overlays(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.dismiss_terminal_highlight_popover() {
            cx.notify();
            return true;
        }
        if self.close_terminal_cwd_picker(cx) {
            cx.notify();
            return true;
        }

        if self.close_terminal_git_branch_picker(cx) {
            cx.notify();
            return true;
        }

        if self.close_terminal_project_panel(cx) {
            cx.notify();
            return true;
        }

        if self
            .terminal_command_sender
            .update(cx, |sender, _cx| sender.dismiss_compact_suggestions())
        {
            cx.notify();
            return true;
        }

        false
    }

    pub(super) fn handle_terminal_command_overlay_escape(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if event.keystroke.key.as_str() != "escape" || event.keystroke.modifiers.platform {
            return false;
        }

        if self.close_terminal_command_overlays(cx) {
            return true;
        }
        false
    }

    pub(super) fn handle_workspace_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.session_sort_menu_open {
            if event.keystroke.key == "escape" {
                self.session_sort_menu_open = false;
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }

        if self.terminal_command_sender_editor_focused(window, cx) {
            // Child editor handlers own the bubble path while focused.
            return;
        }
        if active_ime_should_defer_input_key(
            self.active_ime_target(cx).is_some(),
            self.ime_marked_text.is_some(),
            &event.keystroke,
        ) {
            // The capture handler deliberately lets platform text input own text
            // and IME composition keys; the bubble fallback must follow the same
            // rule so inputs do not append or activate once per key path.
            return;
        }

        if self.active_ime_target(cx) == Some(ime::WorkspaceImeTarget::ActiveSessionSearch) {
            if event.keystroke.key == "escape" {
                self.session_search_query.clear();
                self.session_search_open = false;
                self.begin_disclosure_motion("session-search".into(), false, cx);
                self.clear_ime_selection();
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }

        if self.connection_form_state(cx).form.is_some() {
            let _ = self.handle_new_connection_key(event, window, cx);
            return;
        }

        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;

        if self.handle_settings_confirm_key(event, cx) {
            return;
        }

        if self.handle_oxide_dialog_footer_key(event, cx) {
            return;
        }

        let connection_monitor_keys_visible = self.context_sidebar_visible()
            && self.active_context_sidebar_panel == ContextSidebarPanel::HostTools
            && matches!(
                self.host_tools.read(cx).active_tool(),
                ContextSidebarTool::Monitor
                    | ContextSidebarTool::Gpu
                    | ContextSidebarTool::Processes
                    | ContextSidebarTool::Services
                    | ContextSidebarTool::Logs
                    | ContextSidebarTool::Tmux
                    | ContextSidebarTool::Docker
                    | ContextSidebarTool::Ports
                    | ContextSidebarTool::Schedules
                    | ContextSidebarTool::Filesystems
                    | ContextSidebarTool::Packages
            );
        if connection_monitor_keys_visible && self.handle_connection_monitor_select_key(event, cx) {
            return;
        }

        if self.handle_host_process_search_key(event, cx) {
            return;
        }
        if self.handle_host_docker_search_key(event, cx) {
            return;
        }
        if self.handle_host_service_search_key(event, cx) {
            return;
        }
        if self.handle_host_log_search_key(event, cx) {
            return;
        }
        if self.handle_host_tmux_search_key(event, cx) {
            return;
        }
        if self.handle_host_port_search_key(event, cx) {
            return;
        }
        if self.handle_host_schedule_search_key(event, cx) {
            return;
        }
        if self.handle_host_filesystem_search_key(event, cx) {
            return;
        }
        if self.handle_host_package_search_key(event, cx) {
            return;
        }
        if self.handle_host_tmux_input_dialog_key(event, cx) {
            return;
        }

        if self.active_surface == ActiveSurface::Settings && self.open_settings_select.is_some() {
            if self.open_settings_select == Some(SettingsSelect::AppearanceTheme)
                && self.handle_appearance_theme_select_key(event, cx)
            {
                window.prevent_default();
                cx.stop_propagation();
                return;
            }
            if key == "escape" && !modifiers.platform {
                self.close_settings_select();
                cx.notify();
            }
            return;
        }

        if self.focused_settings_input.is_some()
            || self
                .settings_workspace
                .read(cx)
                .settings_entity_focused_input()
                .is_some()
        {
            let _ = self.handle_settings_input_key(event, cx);
            return;
        }

        if self.handle_terminal_cwd_picker_key(event, window, cx) {
            return;
        }

        if self.handle_terminal_git_branch_picker_key(event, cx) {
            return;
        }

        if self.handle_terminal_project_panel_key(event, cx) {
            return;
        }

        if self.handle_terminal_command_overlay_escape(event, cx) {
            return;
        }

        if self.active_session_manager_input(cx).is_some() {
            let _ = self.handle_session_manager_key(event, cx);
            return;
        }

        if self.sftp_view().read(cx).focused_input().is_some()
            || self
                .active_tab(cx)
                .is_some_and(|tab| tab.kind == TabKind::Sftp)
        {
            let _ = self.handle_sftp_key(event, window, cx);
            return;
        }

        if self
            .active_tab(cx)
            .is_some_and(|tab| tab.kind == TabKind::Graphics)
            && self.graphics.read(cx).focused_input().is_some()
        {
            let _ = self.handle_graphics_key(event, cx);
            return;
        }

        let close_panel_shortcut = crate::keybindings::keystroke_matches_action(
            &event.keystroke,
            "terminal.closePanel",
            &self.settings_store.settings().keybindings.overrides,
        );

        if close_panel_shortcut && self.focused_search_pane(cx).is_some() {
            self.close_search(window, cx);
            return;
        }

        if close_panel_shortcut && self.context_sidebar_visible() {
            self.collapse_context_sidebar(cx);
            self.focus_active_pane(window, cx);
            return;
        }

        if self.active_surface == ActiveSurface::Settings && key == "escape" && !modifiers.platform
        {
            self.close_settings(window, cx);
            return;
        }

        if self.focused_search_pane(cx).is_some() && !modifiers.platform {
            match key {
                "escape" => self.close_search(window, cx),
                "enter" => self.search_next(!modifiers.shift, cx),
                "backspace" => {
                    self.handle_active_text_input_delete_selection(&event.keystroke, cx);
                }
                _ => {}
            }
            return;
        }

        if self.forward_unhandled_key_to_active_terminal(event, window, cx) {
            return;
        }
    }

    pub(super) fn forward_terminal_tab_from_capture(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !terminal_tab_capture_keystroke(&event.keystroke) {
            return false;
        }

        if terminal_tab_capture_blocked_by_workspace_ui(self.active_ime_target(cx).is_some()) {
            return false;
        }

        self.forward_unhandled_key_to_active_terminal(event, window, cx)
    }

    fn forward_unhandled_key_to_active_terminal(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let terminal_active = self.active_tab(cx).is_some_and(|tab| {
            matches!(
                tab.kind,
                TabKind::LocalTerminal
                    | TabKind::SshTerminal
                    | TabKind::MoshTerminal
                    | TabKind::Workspace
            )
        });
        if !terminal_active {
            return false;
        }

        let Some(pane) = self.active_pane(cx) else {
            return false;
        };
        let handled = pane.update(cx, |pane, cx| pane.handle_unfocused_key(event, cx));
        if handled {
            // The pane encoder wrote a terminal control sequence. Stop here so
            // GPUI focus traversal or default widget handling cannot also run.
            window.prevent_default();
            cx.stop_propagation();
        }
        handled
    }

    pub(super) fn standard_confirm_focus(&self) -> Option<ConfirmDialogAction> {
        self.standard_confirm_focused_action
    }

    pub(super) fn standard_confirm_focus_owner(&self) -> Option<ConfirmDialogAction> {
        self.standard_confirm_focused_action
    }

    pub(super) fn reset_standard_confirm_focus(&mut self) {
        // Tauri useConfirm does not paint a default footer button highlight.
        // Keyboard activation still falls back to Cancel inside
        // handle_standard_confirm_key; visible focus appears only after an
        // explicit Tab/arrow navigation writes an action owner.
        self.standard_confirm_focused_action = None;
    }

    pub(super) fn set_standard_confirm_focus(&mut self, action: ConfirmDialogAction) {
        self.standard_confirm_focused_action = Some(action);
    }

    pub(super) fn clear_standard_confirm_focus(&mut self) {
        self.standard_confirm_focused_action = None;
    }

    pub(super) fn handle_standard_confirm_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> Option<ConfirmKeyboardAction> {
        if event.keystroke.modifiers.platform || event.keystroke.modifiers.control {
            return None;
        }

        match browser_behavior::modal_footer_key_action(
            event.keystroke.key.as_str(),
            event.keystroke.modifiers.shift,
            &CONFIRM_DIALOG_FOOTER_ACTIONS,
            self.standard_confirm_focused_action,
            ConfirmDialogAction::Cancel,
        ) {
            Some(browser_behavior::ModalFooterKeyAction::Cancel) => {
                self.clear_standard_confirm_focus();
                Some(ConfirmKeyboardAction::Cancel)
            }
            Some(browser_behavior::ModalFooterKeyAction::Focus(action)) => {
                self.standard_confirm_focused_action = Some(action);
                cx.notify();
                Some(ConfirmKeyboardAction::Handled)
            }
            Some(browser_behavior::ModalFooterKeyAction::Activate(action)) => {
                self.clear_standard_confirm_focus();
                Some(match action {
                    ConfirmDialogAction::Cancel => ConfirmKeyboardAction::Cancel,
                    ConfirmDialogAction::Confirm => ConfirmKeyboardAction::Confirm,
                })
            }
            None => None,
        }
    }

    pub(super) fn handle_settings_confirm_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if self
            .settings_workspace
            .read(cx)
            .ssh_config_import_dialog_open()
        {
            if event.keystroke.key.as_str() == "escape" {
                self.close_settings_ssh_config_import_dialog(cx);
            }
            // The import dialog owns keyboard input while it is mounted.
            true
        } else if self.handle_keybinding_reset_confirm_key(event, cx) {
            true
        } else {
            self.handle_settings_data_directory_confirm_key(event, cx)
        }
    }

    pub(super) fn handle_keybinding_reset_confirm_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self
            .settings_workspace
            .read(cx)
            .keybinding_reset_confirm_snapshot()
            .is_some_and(|snapshot| snapshot.phase == oxideterm_gpui_ui::motion::ExitPhase::Visible)
        {
            return false;
        }
        let key_action = self.settings_workspace.update(cx, |settings, cx| {
            settings.handle_keybinding_reset_confirm_key(
                event.keystroke.key.as_str(),
                event.keystroke.modifiers.shift,
                event.keystroke.modifiers.platform || event.keystroke.modifiers.control,
                cx,
            )
        });
        match key_action {
            Some(settings::KeybindingResetConfirmKeyAction::Cancel) => {
                self.begin_keybinding_reset_all_confirm_exit(cx);
                cx.notify();
                true
            }
            Some(settings::KeybindingResetConfirmKeyAction::Confirm) => {
                if self.begin_keybinding_reset_all_confirm_exit(cx) {
                    self.reset_all_keybindings(cx);
                }
                true
            }
            Some(settings::KeybindingResetConfirmKeyAction::Handled) => true,
            None => false,
        }
    }

    pub(super) fn handle_settings_data_directory_confirm_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self
            .settings_workspace
            .read(cx)
            .data_directory_confirm_is_visible()
        {
            return false;
        }
        match self.handle_standard_confirm_key(event, cx) {
            Some(ConfirmKeyboardAction::Cancel) => {
                self.cancel_settings_data_directory_confirm(cx);
                true
            }
            Some(ConfirmKeyboardAction::Confirm) => {
                self.confirm_settings_data_directory(cx);
                true
            }
            Some(ConfirmKeyboardAction::Handled) => true,
            None => false,
        }
    }




    pub(super) fn handle_settings_reset_confirm_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let key_action = self.overlay.update(cx, |overlay, cx| {
            overlay.handle_confirm_key(
                event.keystroke.key.as_str(),
                event.keystroke.modifiers.shift,
                event.keystroke.modifiers.platform || event.keystroke.modifiers.control,
                cx,
            )
        });
        match key_action {
            Some(WorkspaceOverlayConfirmKeyAction::Cancel) => {
                self.begin_settings_reset_confirm_exit(false, cx);
                true
            }
            Some(WorkspaceOverlayConfirmKeyAction::Confirm) => {
                self.begin_settings_reset_confirm_exit(true, cx);
                true
            }
            Some(WorkspaceOverlayConfirmKeyAction::Handled) => true,
            None => false,
        }
    }

    pub(super) fn handle_tab_close_confirm_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(snapshot) = self.tab_host.read(cx).close_confirm_snapshot() else {
            return false;
        };
        if snapshot.phase == oxideterm_gpui_ui::motion::ExitPhase::Exiting {
            return true;
        }
        let key_action = self.tab_host.update(cx, |tab_host, cx| {
            tab_host.handle_close_confirm_key(
                event.keystroke.key.as_str(),
                event.keystroke.modifiers.shift,
                event.keystroke.modifiers.platform || event.keystroke.modifiers.control,
                cx,
            )
        });
        match key_action {
            Some(TabCloseConfirmKeyAction::Cancel) => {
                self.cancel_tab_close_confirm(cx);
                true
            }
            Some(TabCloseConfirmKeyAction::Confirm) => {
                self.confirm_tab_close_confirm(window, cx);
                true
            }
            Some(TabCloseConfirmKeyAction::Handled) => true,
            None => false,
        }
    }

    pub(super) fn handle_node_disconnect_confirm_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(snapshot) = self.overlay.read(cx).confirm_snapshot() else {
            return false;
        };
        if !matches!(
            snapshot.kind,
            WorkspaceOverlayConfirmKind::NodeDisconnect { .. }
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
            Some(WorkspaceOverlayConfirmKeyAction::Cancel) => {
                self.cancel_node_disconnect_confirm(cx);
                true
            }
            Some(WorkspaceOverlayConfirmKeyAction::Confirm) => {
                self.confirm_node_disconnect_confirm(window, cx);
                true
            }
            Some(WorkspaceOverlayConfirmKeyAction::Handled) => true,
            None => false,
        }
    }

    pub(super) fn handle_keybinding_recording_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        let definitions = self.keybinding_definitions();
        let overrides = &self.settings_store.settings().keybindings.overrides;
        let action = self.settings_workspace.update(cx, |settings, cx| {
            settings.handle_keybinding_recording_key(event, overrides, &definitions, cx)
        });
        if action == Some(settings::KeybindingRecordingKeyAction::Confirm) {
            self.confirm_keybinding_recording(cx);
        }
    }

    pub(super) fn activate_keybinding_recording_footer_action(
        &mut self,
        action: settings::KeybindingRecordingFooterAction,
        cx: &mut Context<Self>,
    ) {
        let should_confirm = self.settings_workspace.update(cx, |settings, cx| {
            settings.activate_keybinding_recording_footer(action, cx)
        });
        if should_confirm {
            self.confirm_keybinding_recording(cx);
        }
    }

    pub(super) fn confirm_keybinding_recording(&mut self, cx: &mut Context<Self>) {
        let Some(commit) = self.settings_workspace.update(cx, |settings, cx| {
            settings.take_keybinding_recording_commit(cx)
        }) else {
            return;
        };
        let Some(definition) = self.keybinding_definition(&commit.action_id) else {
            return;
        };

        let side = crate::keybindings::KeybindingSide::current();
        let previous = crate::keybindings::effective_combo(
            &definition,
            &self.settings_store.settings().keybindings.overrides,
            side,
        );
        let runtime_bindings = crate::keybindings::runtime_rebind_key_bindings(
            &commit.action_id,
            previous.as_ref(),
            Some(&commit.combo),
        );

        self.edit_settings(
            move |settings| {
                crate::keybindings::set_definition_override(
                    &mut settings.keybindings.overrides,
                    &definition,
                    side,
                    commit.combo,
                );
            },
            cx,
        );
        Self::apply_runtime_key_bindings(runtime_bindings, cx);
    }

    pub(super) fn cancel_keybinding_recording(&mut self, cx: &mut Context<Self>) {
        self.settings_workspace.update(cx, |settings, cx| {
            settings.stop_keybinding_recording(cx);
        });
    }

    pub(super) fn reset_keybinding(&mut self, action_id: &str, cx: &mut Context<Self>) {
        let Some(definition) = self.keybinding_definition(action_id) else {
            return;
        };
        let side = crate::keybindings::KeybindingSide::current();
        let previous = crate::keybindings::effective_combo(
            &definition,
            &self.settings_store.settings().keybindings.overrides,
            side,
        );
        let next = definition.default_combo(side);
        let runtime_bindings = crate::keybindings::runtime_rebind_key_bindings(
            action_id,
            previous.as_ref(),
            Some(next),
        );
        self.edit_settings(
            |settings| {
                crate::keybindings::reset_override(
                    &mut settings.keybindings.overrides,
                    action_id,
                    side,
                );
            },
            cx,
        );
        self.cancel_keybinding_recording(cx);
        Self::apply_runtime_key_bindings(runtime_bindings, cx);
    }

    pub(super) fn unbind_keybinding(&mut self, action_id: &str, cx: &mut Context<Self>) {
        let Some(definition) = self.keybinding_definition(action_id) else {
            return;
        };
        let side = crate::keybindings::KeybindingSide::current();
        let previous = crate::keybindings::effective_combo(
            &definition,
            &self.settings_store.settings().keybindings.overrides,
            side,
        );
        let runtime_bindings =
            crate::keybindings::runtime_rebind_key_bindings(action_id, previous.as_ref(), None);
        self.edit_settings(
            |settings| {
                crate::keybindings::set_unbound_override(
                    &mut settings.keybindings.overrides,
                    action_id,
                    side,
                );
            },
            cx,
        );
        self.cancel_keybinding_recording(cx);
        Self::apply_runtime_key_bindings(runtime_bindings, cx);
    }

    pub(super) fn reset_all_keybindings(&mut self, cx: &mut Context<Self>) {
        let side = crate::keybindings::KeybindingSide::current();
        let runtime_bindings = {
            let overrides = &self.settings_store.settings().keybindings.overrides;
            crate::keybindings::ACTION_DEFINITIONS
                .iter()
                .flat_map(|definition| {
                    let previous = crate::keybindings::effective_combo(definition, overrides, side);
                    crate::keybindings::runtime_rebind_key_bindings(
                        &definition.id,
                        previous.as_ref(),
                        Some(definition.default_combo(side)),
                    )
                })
                .collect::<Vec<_>>()
        };
        self.edit_settings(|settings| settings.keybindings.overrides.clear(), cx);
        self.cancel_keybinding_recording(cx);
        Self::apply_runtime_key_bindings(runtime_bindings, cx);
    }

    pub(super) fn export_keybindings(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(SharedString::from(
                self.i18n.t("settings_view.keybindings.export"),
            )),
        });
        let selection = async move {
            match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            }
        };
        let overrides = self.settings_store.settings().keybindings.overrides.clone();
        let runtime = self.forwarding_runtime.handle().clone();
        self.settings_workspace.update(cx, |settings, cx| {
            settings.start_keybinding_export(selection, overrides, runtime, cx);
        });
    }

    pub(super) fn import_keybindings(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from(
                self.i18n.t("settings_view.keybindings.import"),
            )),
        });
        let selection = async move {
            match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            }
        };
        let runtime = self.forwarding_runtime.handle().clone();
        self.settings_workspace.update(cx, |settings, cx| {
            settings.start_keybinding_import(selection, runtime, cx);
        });
    }

    pub(in crate::workspace) fn apply_runtime_key_bindings(
        bindings: Vec<gpui::KeyBinding>,
        cx: &mut App,
    ) {
        if !bindings.is_empty() {
            // The keymap belongs to the app; the calling window may already be updating.
            cx.bind_keys(bindings);
        }
    }

    pub(super) fn switch_locale(&mut self, locale: Locale, cx: &mut Context<Self>) {
        // Route language changes through the same settings mutation path as the
        // settings UI so native plugin language/settings subscriptions observe
        // menu-triggered locale switches too.
        self.edit_settings(
            |settings| settings.general.language = settings_language_from_locale(locale),
            cx,
        );
    }

    pub(super) fn sync_tab_titles(&mut self, cx: &mut App) {
        // Localized tab titles are derived only when the locale changes. Keeping
        // this work out of render avoids allocating every translated title on
        // unrelated terminal repaint frames.
        let i18n = &self.i18n;
        self.tab_host.update(cx, |tab_host, _| {
            tab_host.sync_tab_titles(|key| i18n.t(key))
        });
    }

    pub(super) fn render_search_bar(&self, pane_id: PaneId, cx: &mut Context<Self>) -> AnyElement {
        const SEARCH_PANEL_BG_ALPHA: u32 = 0xf5; // Tauri bg-theme-bg-elevated translated to native opacity.
        const SEARCH_PANEL_BORDER_ALPHA: u32 = 0xcc; // Tauri border-theme-border.

        let theme = self.tokens.ui;
        let Some(search) = self.search.panes.get(&pane_id) else {
            return div().into_any_element();
        };
        let focused = self.focused_search_pane(cx) == Some(pane_id);
        let target = WorkspaceImeTarget::Search(pane_id);
        let has_query = !search.query.is_empty();
        let marked_text = self.marked_text_for_target(target, cx);
        let selected_range = focused
            .then(|| self.ime_selected_range_for_target(target, cx))
            .flatten();
        let input_range = selected_range.filter(|_| has_query && marked_text.is_none());
        let selection_range = input_range.clone().filter(|range| range.start < range.end);
        let caret_offset = input_range
            .as_ref()
            .filter(|range| range.start == range.end)
            .map(|range| range.start);
        let shows_selection = selection_range.is_some();
        let shows_positioned_caret = caret_offset.is_some() && !shows_selection;
        let query = if has_query {
            search.query.clone()
        } else {
            self.i18n.t("search.placeholder")
        };
        let match_count = search.match_count;
        let active_match = search.active_match.filter(|index| *index < match_count);
        let navigation_disabled = !has_query || match_count == 0;
        let result_label = if !has_query {
            String::new()
        } else if match_count == 0 {
            self.i18n.t("search.no_results")
        } else {
            format!("{}/{}", active_match.unwrap_or(0) + 1, match_count)
        };

        div()
            .id(("terminal-search", pane_id.0))
            .absolute()
            .top(px(12.0))
            .right(px(12.0))
            .w(px(420.0))
            .max_w(relative(0.92))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgba((theme.border << 8) | SEARCH_PANEL_BORDER_ALPHA))
            .bg(rgba((theme.bg_elevated << 8) | SEARCH_PANEL_BG_ALPHA))
            .shadow_lg()
            .text_size(px(13.0))
            .text_color(rgb(theme.text))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .h(px(44.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(rgba((theme.border << 8) | 0x99))
                    .child(Self::render_lucide_icon(
                        LucideIcon::Search,
                        15.0,
                        rgb(theme.text_muted),
                    ))
                    .child(
                        self.text_input_with_workspace_ime(
                            target,
                            div()
                                .h(px(28.0))
                                .flex_1()
                                .min_w(px(0.0))
                                .flex()
                                .items_center()
                                .overflow_hidden()
                                .rounded(px(self.tokens.radii.sm))
                                .px(px(2.0))
                                .cursor_text()
                                .text_color(if has_query {
                                    rgb(theme.text)
                                } else {
                                    rgb(theme.text_muted)
                                })
                                .when(focused && !has_query && marked_text.is_none(), |input| {
                                    input
                                        .child(text_caret(&self.tokens, self.input_caret.visible()))
                                })
                                .child(if has_query {
                                    text_input_value_segments_with_color(
                                        &self.tokens,
                                        &query,
                                        false,
                                        selection_range,
                                        caret_offset,
                                        self.input_caret.visible(),
                                        Some(theme.text),
                                    )
                                    .into_any_element()
                                } else {
                                    div().child(query).into_any_element()
                                })
                                .when_some(marked_text, |input, marked| {
                                    input.child(
                                        div()
                                            .underline()
                                            .text_color(rgb(theme.text))
                                            .child(marked.to_string()),
                                    )
                                })
                                .when(
                                    focused
                                        && has_query
                                        && !shows_selection
                                        && !shows_positioned_caret,
                                    |input| {
                                        input.child(text_caret(
                                            &self.tokens,
                                            self.input_caret.visible(),
                                        ))
                                    },
                                ),
                            move |this, cx| this.focus_search_pane(pane_id, cx),
                            cx,
                        ),
                    )
                    .when(has_query, |row| {
                        row.child(
                            div()
                                .flex_none()
                                .min_w(px(48.0))
                                .text_size(px(12.0))
                                .text_color(rgb(theme.text_muted))
                                .child(result_label),
                        )
                    })
                    .child(
                        div()
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(self.tokens.radii.md))
                            .cursor_pointer()
                            .hover(move |style| {
                                if navigation_disabled {
                                    style
                                } else {
                                    style.bg(rgb(theme.bg_hover))
                                }
                            })
                            .child(Self::render_lucide_icon(
                                LucideIcon::ArrowUp,
                                14.0,
                                if navigation_disabled {
                                    rgba((theme.text_muted << 8) | 0x66)
                                } else {
                                    rgb(theme.text_muted)
                                },
                            ))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _event, _window, cx| {
                                    this.search_next_for_pane(pane_id, false, cx);
                                    cx.stop_propagation();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(self.tokens.radii.md))
                            .cursor_pointer()
                            .hover(move |style| {
                                if navigation_disabled {
                                    style
                                } else {
                                    style.bg(rgb(theme.bg_hover))
                                }
                            })
                            .child(Self::render_lucide_icon(
                                LucideIcon::ArrowDown,
                                14.0,
                                if navigation_disabled {
                                    rgba((theme.text_muted << 8) | 0x66)
                                } else {
                                    rgb(theme.text_muted)
                                },
                            ))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _event, _window, cx| {
                                    this.search_next_for_pane(pane_id, true, cx);
                                    cx.stop_propagation();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(self.tokens.radii.md))
                            .cursor_pointer()
                            .hover(move |style| style.bg(rgb(theme.bg_hover)))
                            .child(Self::render_lucide_icon(
                                LucideIcon::X,
                                14.0,
                                rgb(theme.text_muted),
                            ))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _event, window, cx| {
                                    let restore_focus = this.search.focused == Some(pane_id)
                                        || this.active_pane_id(cx) == Some(pane_id);
                                    this.hide_search(pane_id, cx);
                                    if restore_focus
                                        && let Some(pane) =
                                            this.tab_host.read(cx).panes().get(&pane_id).cloned()
                                    {
                                        pane.update(cx, |pane, cx| pane.focus(window, cx));
                                    }
                                    cx.stop_propagation();
                                }),
                            ),
                    ),
            )
            .child(
                div()
                    .px(px(12.0))
                    .py(px(7.0))
                    .text_size(px(11.0))
                    .text_color(rgb(theme.text_muted))
                    .child(self.i18n.t("search.visible_terminal_hint")),
            )
            .into_any_element()
    }
}

fn terminal_command_executable(command: &str) -> Option<String> {
    let segment = command
        .trim()
        .split("&&")
        .flat_map(|part| part.split("||"))
        .flat_map(|part| part.split(';'))
        .find(|part| !part.trim().is_empty())?;
    let tokens = shell_words(segment);
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index].trim();
        if token.is_empty()
            || token.starts_with('-')
            || token
                .split_once('=')
                .is_some_and(|(name, _)| is_shell_assignment_name(name))
        {
            index += 1;
            continue;
        }
        if matches!(token, "sudo" | "command" | "exec" | "env") {
            index += 1;
            continue;
        }
        return token.rsplit('/').next().map(|name| name.to_lowercase());
    }
    None
}

fn shell_words(segment: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for ch in segment.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn is_shell_assignment_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

#[cfg(test)]
mod terminal_command_bar_behavior_tests {
    use super::*;

    fn tab_keystroke_with(modifiers: gpui::Modifiers) -> gpui::Keystroke {
        gpui::Keystroke {
            key: "tab".to_string(),
            modifiers,
            ..Default::default()
        }
    }

    #[test]
    fn terminal_tab_capture_matches_terminal_tab_chords_only() {
        assert!(terminal_tab_capture_keystroke(&tab_keystroke_with(
            gpui::Modifiers::default()
        )));
        assert!(terminal_tab_capture_keystroke(&tab_keystroke_with(
            gpui::Modifiers {
                shift: true,
                ..Default::default()
            }
        )));
        assert!(!terminal_tab_capture_keystroke(&tab_keystroke_with(
            gpui::Modifiers {
                control: true,
                ..Default::default()
            }
        )));
        assert!(!terminal_tab_capture_keystroke(&tab_keystroke_with(
            gpui::Modifiers {
                platform: true,
                ..Default::default()
            }
        )));
    }

    #[test]
    fn terminal_tab_capture_defers_to_workspace_text_ui() {
        assert!(!terminal_tab_capture_blocked_by_workspace_ui(false));
        assert!(terminal_tab_capture_blocked_by_workspace_ui(true));
    }

    #[test]
    fn command_executable_supports_focus_handoff_detection() {
        assert_eq!(
            terminal_command_executable("vim src/main.rs").as_deref(),
            Some("vim")
        );
        assert_eq!(
            terminal_command_executable("FOO=1 sudo /usr/bin/nvim").as_deref(),
            Some("nvim")
        );
        assert_eq!(terminal_command_executable("A=1 B=2").as_deref(), None);
    }
}

pub(super) fn classify_command_risk(command: &str) -> Option<&'static str> {
    // Completion suggestions still store presentation labels as strings, so
    // adapt the domain result at the existing app boundary.
    match classify_quick_command_risk(command) {
        Some(QuickCommandRisk::High) => Some("high"),
        Some(QuickCommandRisk::Medium) => Some("medium"),
        None => None,
    }
}

#[cfg(test)]
mod keybinding_update_tests {
    use super::*;
    use gpui::{FocusHandle, KeyBinding, TestAppContext};

    struct ShortcutTarget {
        focus: FocusHandle,
        actions: usize,
        input: Vec<String>,
    }

    impl Render for ShortcutTarget {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .key_context("Workspace")
                .track_focus(&self.focus)
                .on_action(cx.listener(|this, _: &NewTerminal, _, _| this.actions += 1))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, _| {
                    this.input.push(event.keystroke.unparse());
                }))
        }
    }

    #[gpui::test]
    fn runtime_rebind_releases_old_shortcut_during_window_update(cx: &mut TestAppContext) {
        use crate::keybindings::{KeybindingSide, action_definition, runtime_rebind_key_bindings};

        cx.update(|cx| cx.bind_keys([KeyBinding::new("ctrl-t", NewTerminal, Some("Workspace"))]));
        let previous = action_definition("app.newTerminal")
            .unwrap()
            .default_combo(KeybindingSide::Other)
            .clone();
        let mut next = previous.clone();
        next.key = "p".into();
        let (target, cx) = cx.add_window_view(|window, cx| {
            let focus = cx.focus_handle();
            focus.focus(window, cx);
            ShortcutTarget {
                focus,
                actions: 0,
                input: Vec::new(),
            }
        });
        cx.simulate_keystrokes("ctrl-t");
        target.read_with(cx, |target, _| assert_eq!(target.actions, 1));
        for (label, previous, next, expected_input, expected_actions) in [
            ("rebind", Some(&previous), Some(&next), vec!["ctrl-t"], 1),
            ("unbind", Some(&next), None, vec!["ctrl-t", "ctrl-p"], 0),
            ("restore", None, Some(&previous), vec!["ctrl-p"], 1),
        ] {
            target.update(cx, |target, _| {
                target.actions = 0;
                target.input.clear();
            });
            // Settings callbacks already hold the window when updating the keymap.
            cx.update(|_window, cx| {
                WorkspaceApp::apply_runtime_key_bindings(
                    runtime_rebind_key_bindings("app.newTerminal", previous, next),
                    cx,
                );
            });
            cx.simulate_keystrokes("ctrl-t ctrl-p");
            target.read_with(cx, |target, _| {
                assert_eq!(target.input, expected_input, "{label}");
                assert_eq!(target.actions, expected_actions, "{label}");
            });
        }
    }
}
#[cfg(all(test, unix))]
mod terminal_command_dispatch_audit_tests {
    use super::*;
    use gpui::{AppContext, IntoElement, Render, TestAppContext, div};
    use oxideterm_audit::{AuditContext, AuditOutcome, AuditQuery, AuditService, AuditSource};

    struct Root;
    impl Render for Root {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    struct Keys;
    impl oxideterm_audit::AuditKeyProvider for Keys {
        fn load(&self, _: &str) -> Result<Zeroizing<Vec<u8>>, oxideterm_audit::AuditError> {
            Ok(Zeroizing::new(vec![7; 32]))
        }
        fn create(&self, id: &str) -> Result<Zeroizing<Vec<u8>>, oxideterm_audit::AuditError> {
            self.load(id)
        }
    }

    /// A pane that refuses input must record a failed dispatch rather than a sent
    /// one, and every record must be attributed to its own terminal session.
    #[gpui::test]
    fn locked_pane_records_a_failed_command_dispatch(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        let directory = tempfile::tempdir().unwrap();
        oxideterm_audit::AuditStore::open(&directory.path().join("audit.db"), &Keys)
            .unwrap()
            .set_policy(oxideterm_audit::AuditPolicy {
                enabled: true,
                ..Default::default()
            })
            .unwrap();
        let service =
            AuditService::with_key_provider(directory.path().join("audit.db"), Keys).unwrap();
        let _registration = AuditContext::new(service.client(), AuditSource::User).install();
        let (_, cx) = cx.add_window_view(|_, _| Root);
        let panes = cx.update(|window, cx| {
            (0..3)
                .map(|_| {
                    let config = oxideterm_terminal::LocalPtyConfig {
                        shell: Some(oxideterm_terminal::ShellInfo::new(
                            "test-cat", "cat", "/bin/cat",
                        )),
                        load_profile: false,
                        ..Default::default()
                    };
                    cx.new(|cx| {
                        TerminalPane::new_local_with_config_and_preferences(
                            config,
                            Default::default(),
                            window,
                            cx,
                        )
                        .unwrap()
                    })
                })
                .collect::<Vec<_>>()
        });
        let sessions = panes
            .iter()
            .map(|pane| {
                pane.read_with(cx, |pane, _| {
                    pane.audit_context().unwrap().session_id.unwrap()
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(sessions.iter().collect::<HashSet<_>>().len(), 3);
        panes[2].update(cx, |pane, cx| pane.set_input_locked(true, cx));
        for (pane, command) in panes.iter().zip(["one", "two", "three"]) {
            pane.update(cx, |pane, cx| {
                pane.send_command_line_with_mark(
                    command,
                    TerminalCommandMarkDetectionSource::UserInputObserved,
                    None,
                    cx,
                )
            });
        }

        let page = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(service.client().query(AuditQuery {
                limit: 80,
                ..Default::default()
            }))
            .unwrap();
        let dispatches = page
            .records
            .iter()
            .filter_map(|record| record.details.operation.as_ref())
            .filter(|operation| {
                operation.action == "command_dispatch"
                    && operation.phase == Some(oxideterm_audit::AuditPhase::Result)
            })
            .collect::<Vec<_>>();
        assert_eq!(dispatches.len(), 3);
        for (session, expected) in
            sessions
                .iter()
                .zip([AuditOutcome::Sent, AuditOutcome::Sent, AuditOutcome::Failed])
        {
            let dispatch = dispatches
                .iter()
                .find(|operation| operation.session_id.as_deref() == Some(session.as_str()))
                .unwrap();
            assert_eq!(dispatch.outcome, expected);
        }
    }
}
