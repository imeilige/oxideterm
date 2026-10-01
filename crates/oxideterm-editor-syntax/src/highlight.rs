// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use std::{cell::Cell, ops::Range};

use oxideterm_editor_core::{BufferOffset, TextRange};
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

use crate::{HighlightSpan, LanguageId, SyntaxScope};

pub(crate) fn highlight_spans(
    _language_id: LanguageId,
    tree: &Tree,
    highlight_query: &Query,
    _markdown_inline_query: Option<&Query>,
    source: &str,
    requested: Range<usize>,
    work: Option<&crate::SyntaxWork>,
) -> Result<Vec<HighlightSpan>, crate::SyntaxError> {
    let requested = requested.start..requested.end.min(source.len());
    if requested.start >= requested.end {
        return Ok(Vec::new());
    }
    let mut cursor = QueryCursor::new();
    // Query from the root so predicates and captures keep their ancestor context.
    cursor.set_byte_range(requested.clone());
    let paused = Cell::new(false);
    let mut pause = |_: &tree_sitter::QueryCursorState| {
        let stop = work.is_some_and(crate::SyntaxWork::should_pause);
        paused.set(stop);
        stop
    };
    let mut captures = cursor.captures_with_options(
        highlight_query,
        tree.root_node(),
        source.as_bytes(),
        tree_sitter::QueryCursorOptions::new().progress_callback(&mut pause),
    );
    let names = highlight_query.capture_names();
    let mut spans = Vec::new();

    loop {
        crate::work::checkpoint(work)?;
        captures.advance();
        if captures.get().is_none() {
            if paused.replace(false) {
                continue;
            }
            break;
        }
        let Some((query_match, capture_index)) = captures.get() else {
            continue;
        };
        let Some(capture) = query_match.captures.get(*capture_index).copied() else {
            continue;
        };
        let Some(capture_name) = names.get(capture.index as usize).copied() else {
            continue;
        };
        let Some(scope) = scope_for_capture(capture_name, capture.node.kind()) else {
            continue;
        };
        let range = capture.node.byte_range();
        if range.start < range.end
            && range.end <= source.len()
            && range.start < requested.end
            && requested.start < range.end
        {
            spans.push(HighlightSpan {
                range: TextRange::new(BufferOffset(range.start), BufferOffset(range.end)),
                scope,
            });
        }
    }


    crate::work::checkpoint(work)?;
    Ok(normalize_highlight_spans(spans))
}

fn scope_for_capture(capture: &str, node_kind: &str) -> Option<SyntaxScope> {
    if matches!(node_kind, "integer_literal" | "float_literal") {
        return Some(SyntaxScope::Number);
    }
    if capture == "punctuation.delimiter"
        && matches!(
            node_kind,
            "code_span_delimiter" | "fenced_code_block_delimiter"
        )
    {
        // Markdown code markers belong to the literal token visually. Keeping
        // them in the muted punctuation palette makes valid code look open.
        return Some(SyntaxScope::String);
    }
    match capture {
        // Tauri loads `@codemirror/lang-markdown`, whose Lezer tags include
        // heading/link/literal punctuation. The native editor maps those
        // Markdown-specific captures onto the existing syntax palette so `.md`
        // files are highlighted without adding a parallel color system.
        "text.title" => return Some(SyntaxScope::Keyword),
        "text.uri" => return Some(SyntaxScope::Function),
        "text.literal" => return Some(SyntaxScope::String),
        "text.reference" => return Some(SyntaxScope::Type),
        "text.emphasis" | "text.strong" => return Some(SyntaxScope::Variable),
        _ => {}
    }

    let root = capture.split('.').next().unwrap_or(capture);
    match root {
        "attribute" => Some(SyntaxScope::Attribute),
        "comment" => Some(SyntaxScope::Comment),
        "constant" => Some(SyntaxScope::Constant),
        "function" => Some(SyntaxScope::Function),
        "keyword" => Some(SyntaxScope::Keyword),
        "module" | "namespace" => Some(SyntaxScope::Namespace),
        "number" => Some(SyntaxScope::Number),
        "operator" => Some(SyntaxScope::Operator),
        "property" | "field" => Some(SyntaxScope::Property),
        "punctuation" => Some(SyntaxScope::Punctuation),
        "string" | "character" => Some(SyntaxScope::String),
        "type" | "constructor" => Some(SyntaxScope::Type),
        "variable" | "parameter" => Some(SyntaxScope::Variable),
        _ => None,
    }
}




fn normalize_highlight_spans(mut spans: Vec<HighlightSpan>) -> Vec<HighlightSpan> {
    spans.sort_by(|left, right| {
        left.range
            .start
            .cmp(&right.range.start)
            .then_with(|| left.range.end.cmp(&right.range.end))
    });
    spans.dedup_by(|left, right| left.range == right.range && left.scope == right.scope);
    spans
}
