use super::*;

impl WorkspaceApp {
    pub(in crate::workspace) fn handle_terminal_context_action_request_for_pane(
        &mut self,
        pane_id: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(source_pane) = self.tab_host.read(cx).panes().get(&pane_id).cloned() else {
            return false;
        };
        let Some(action) = source_pane.update(cx, |pane, _cx| pane.take_context_action_request())
        else {
            return false;
        };

        match action {
            TerminalContextAction::OpenSearch => {
                self.open_search(window, cx);
                true
            }
            TerminalContextAction::OpenSessionTriggers => {
                self.open_terminal_trigger_settings_for_pane(pane_id, window, cx);
                true
            }
        }
    }
}
