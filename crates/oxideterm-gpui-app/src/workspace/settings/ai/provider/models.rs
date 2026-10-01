use super::*;

impl WorkspaceApp {
    pub(in crate::workspace) fn ai_provider_has_key(&self, provider_id: &str, cx: &App) -> bool {
        self.ai_provider_has_key_cached(provider_id, cx)
    }
}
