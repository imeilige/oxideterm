#[cfg(feature = "bench")]
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TerminalPlaybackUpdateTimings {
    pub terminal_lock: Duration,
    pub parse_and_grid: Duration,
    pub event_extraction: Duration,
    pub gpui_state_update: Duration,
}

impl TerminalPane {
    /// Injects bytes into the session without a live PTY. The name predates the
    /// removal of the recording feature; it is the general output path shared by
    /// benchmark harnesses and pane tests, not a recording playback control.
    pub fn feed_recording_output(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        {
            let mut terminal = self.terminal.lock();
            terminal.feed_recording_output(bytes);
            let _ = terminal.take_events();
        }
        // Match live PTY output: multiple updates may coalesce before the visible pane builds one
        // latest incremental snapshot during render.
        self.snapshot_dirty = true;
        self.mark_terminal_content_changed(cx);
        cx.notify();
    }

    #[cfg(feature = "bench")]
    #[doc(hidden)]
    pub fn feed_benchmark_output(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        // Recording playback intentionally discards events; command-aware benchmarks need the
        // same shell integration event handling as live output before building the next frame.
        let events = {
            let mut terminal = self.terminal.lock();
            terminal.feed_recording_output(bytes);
            terminal.take_events()
        };
        for event in events {
            self.handle_terminal_event(event, cx);
        }
        self.snapshot_dirty = true;
        self.mark_terminal_content_changed(cx);
        cx.notify();
    }

    #[cfg(feature = "bench")]
    #[doc(hidden)]
    pub fn enable_benchmark_performance_metrics(&mut self) {
        // Benchmark collection is separate from the visible overlay so diagnostic chrome does
        // not change the scene whose layout and paint cost is being measured.
        self.benchmark_performance_metrics_enabled = true;
    }

    #[cfg(feature = "bench")]
    #[doc(hidden)]
    pub fn benchmark_render_stage_micros(&self) -> (u64, u64, u64, u64, u64) {
        let performance = self.layout_cache.lock().performance();
        (
            self.render_stats.snapshot_micros,
            self.benchmark_backend_snapshot_micros,
            self.benchmark_snapshot_state_micros,
            performance.layout_micros,
            performance.paint_micros,
        )
    }

    #[cfg(feature = "bench")]
    #[doc(hidden)]
    pub fn benchmark_layout_cache_hit_percent(&self) -> u8 {
        self.layout_cache.lock().performance().cache_hit_percent
    }

    #[cfg(feature = "bench")]
    #[doc(hidden)]
    pub fn benchmark_semantic_stage_micros(&self) -> (u64, u64, usize) {
        let performance = self.layout_cache.lock().performance();
        (
            performance.semantic_roles_micros,
            performance.highlights_micros,
            performance.semantic_command_lines,
        )
    }
}
