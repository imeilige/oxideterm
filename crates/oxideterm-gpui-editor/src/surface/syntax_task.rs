// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use super::*;
use oxideterm_editor_syntax::{SyntaxError, SyntaxWork};
use std::sync::atomic::Ordering;

// Probe measurements establish cooperative slices, not a hard lexer deadline.
const SYNTAX_SLICE: Duration = Duration::from_millis(2);

pub(super) struct SyntaxRequest {
    generation: u64,
    version: u64,
    language: LanguageId,
    edit: Option<SyntaxEdit>,
    reset: bool,
    tab_size: usize,
}

struct SyntaxState {
    version: Option<u64>,
    syntax: Option<SyntaxSession>,
    highlights: HighlightCache,
    structure: StructureCache,
    brackets: BracketIndex,
}

// A completion can be dropped by a closing view or a cancelled foreground
// receiver. Release large trees and indexes on the background executor either way.
struct OwnedSyntax {
    state: Option<SyntaxState>,
    executor: gpui::BackgroundExecutor,
}

impl Drop for OwnedSyntax {
    fn drop(&mut self) {
        if let Some(state) = self.state.take() {
            if state.syntax.is_some() {
                self.executor
                    .spawn_dedicated(move |_| async move {
                        drop(state);
                    })
                    .detach();
            }
        }
    }
}

impl TextEditorView {
    fn take_syntax_state(&mut self) -> OwnedSyntax {
        OwnedSyntax {
            state: Some(SyntaxState {
                version: self.syntax_version.take(),
                syntax: self.syntax.take(),
                highlights: std::mem::take(&mut self.highlight_spans),
                structure: std::mem::take(&mut self.structure_cache),
                brackets: std::mem::take(&mut self.bracket_index),
            }),
            executor: self.syntax_executor.clone(),
        }
    }

    pub(super) fn request_syntax(
        &mut self,
        edit: Option<SyntaxEdit>,
        reset: bool,
        cx: &mut Context<Self>,
    ) {
        let generation = self.syntax_generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.highlight_chunk_cache.borrow_mut().clear();
        let Some(language) = self.language.filter(|_| !self.is_large_file()) else {
            self.pending_syntax = None;
            drop(self.take_syntax_state());
            return;
        };
        let request = SyntaxRequest {
            generation,
            version: self.buffer.version(),
            language,
            edit,
            reset,
            tab_size: self.settings.tab_size,
        };
        if self.syntax_task.is_some() {
            // Keep only metadata while busy; overwritten requests must not copy text.
            self.pending_syntax = Some(request);
        } else {
            let runtime = self.take_syntax_state();
            self.start_syntax(request, runtime, cx);
        }
    }

    fn start_syntax(
        &mut self,
        request: SyntaxRequest,
        runtime: OwnedSyntax,
        cx: &mut Context<Self>,
    ) {
        let generation = request.generation;
        let version = request.version;
        let language = request.language;
        let token = self.syntax_generation.clone();
        let text = self.buffer.text_snapshot();
        let background = self.syntax_executor.spawn_dedicated(move |_| async move {
            let work = SyntaxWork::new(token, generation, SYNTAX_SLICE);
            compute(runtime, &request, &text, &work)
        });
        self.syntax_task = Some(cx.spawn(async move |weak, cx| {
            let result = background.await;
            let _ = weak.update(cx, |this, cx| {
                this.syntax_task = None;
                if let Err(error) = &result {
                    if !matches!(error, SyntaxError::ParseCancelled) {
                        tracing::warn!(%error, "Editor syntax calculation failed");
                    }
                }
                if let Some(pending) = this.pending_syntax.take() {
                    let runtime = result.unwrap_or_else(|_| this.take_syntax_state());
                    this.start_syntax(pending, runtime, cx);
                    return;
                }
                if let Ok(runtime) = result {
                    this.publish_syntax(generation, version, language, runtime, cx);
                }
            });
        }));
    }

    fn publish_syntax(
        &mut self,
        generation: u64,
        version: u64,
        language: LanguageId,
        mut runtime: OwnedSyntax,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.syntax_generation.load(Ordering::Acquire) != generation
            || self.buffer.version() != version
            || self.language != Some(language)
        {
            return false;
        }
        let state = runtime
            .state
            .take()
            .expect("completed syntax owns its state");
        self.syntax = state.syntax;
        self.syntax_version = state.version;
        self.highlight_spans = state.highlights;
        self.structure_cache = state.structure;
        self.bracket_index = state.brackets;
        self.highlight_chunk_cache.borrow_mut().clear();
        if !self.folded_ranges.is_empty() {
            self.refresh_foldable_ranges();
        }
        // Completion must repaint even when the user has stopped typing.
        cx.notify();
        true
    }
}

impl Drop for TextEditorView {
    fn drop(&mut self) {
        self.syntax_generation.fetch_add(1, Ordering::AcqRel);
        drop(self.take_syntax_state());
    }
}

fn compute(
    mut runtime: OwnedSyntax,
    request: &SyntaxRequest,
    text: &str,
    work: &SyntaxWork,
) -> Result<OwnedSyntax, SyntaxError> {
    let result = (|| -> Result<(), SyntaxError> {
        work.checkpoint()?;
        let state = runtime.state.as_mut().expect("worker owns syntax state");
        let same_language = state
            .syntax
            .as_ref()
            .is_some_and(|syntax| syntax.language_id() == request.language);
        let change =
            if !request.reset && same_language && state.version == Some(request.version) {
                None
            } else if !request.reset
                && same_language
                && state.version.and_then(|version| version.checked_add(1)) == Some(request.version)
                && let Some(edit) = request.edit
            {
                Some(state.syntax.as_mut().unwrap().apply_edit_controlled(
                    text,
                    edit,
                    Some(work),
                )?)
            } else {
                state.syntax = Some(SyntaxSession::parse_controlled(
                    request.language,
                    text,
                    Some(work),
                )?);
                None
            };
        let syntax = state.syntax.as_ref().unwrap();
        state
            .highlights
            .update_controlled(syntax, text, change.as_ref(), Some(work))?;
        state.structure.update_controlled(
            syntax,
            text,
            request.tab_size,
            change.as_ref(),
            Some(work),
        )?;
        state.brackets = syntax.bracket_index_controlled(text, Some(work))?;
        state.version = Some(request.version);
        work.checkpoint()
    })();
    // A cancelled parse may have edited its tree or partially moved cache blocks.
    // Discard that private state rather than reusing it for a different input.
    match result {
        Ok(()) => Ok(runtime),
        Err(error) => {
            // Already on the dedicated worker: finish teardown before starting
            // another request, so cancelled jobs cannot accumulate cleanup work.
            drop(runtime.state.take());
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    
    





}
