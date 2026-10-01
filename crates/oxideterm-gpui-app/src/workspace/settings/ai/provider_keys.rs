use super::*;

impl WorkspaceApp {
    pub(in crate::workspace) fn ai_provider_has_key_cached(
        &self,
        provider_id: &str,
        cx: &App,
    ) -> bool {
        self.ai_entity.read(cx).provider_has_key(provider_id)
    }

    pub(in crate::workspace) fn ensure_ai_provider_key_statuses(&mut self, cx: &mut Context<Self>) {
        let provider_views = ai_provider_views(self.settings_store.settings());
        self.ensure_ai_provider_key_statuses_for_views(&provider_views, cx);
    }

    pub(in crate::workspace) fn ensure_ai_provider_key_statuses_for_views(
        &mut self,
        provider_views: &[AiProviderView],
        cx: &mut Context<Self>,
    ) {
        // Rendering OxideSens already derives provider views, so reuse that
        // snapshot when available instead of parsing the same JSON again.
        let provider_jobs: Vec<_> = provider_views
            .iter()
            .filter(|provider| {
                ai_provider_key_display_state(&provider.provider_type, false).shows_key_control()
            })
            .map(|provider| provider.id.clone())
            .collect();

        self.ai_entity.update(cx, |ai, _cx| {
            ai.request_provider_key_statuses(provider_jobs);
        });
    }

    pub(in crate::workspace) fn remove_ai_provider_api_key(
        &mut self,
        _index: usize,
        provider_id: &str,
        cx: &mut Context<Self>,
    ) {
        self.ai_entity.update(cx, |ai, cx| {
            ai.remove_provider_key(provider_id.to_string(), cx);
        });
        cx.notify();
    }
}
