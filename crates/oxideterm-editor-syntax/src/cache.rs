// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use std::{collections::HashMap, ops::Range, sync::Arc};

const SPANS_PER_INDEX_ENTRY: usize = 64;

use oxideterm_editor_core::{BufferOffset, TextRange};

use crate::{HighlightSpan, SyntaxChange, SyntaxScope, SyntaxSession};

// Tree-sitter's node byte offsets are uint32_t, including on 64-bit hosts.
// Keep that width in retained spans; expand only at the public range boundary.
#[derive(Debug)]
struct CachedHighlight {
    start: u32,
    end: u32,
    scope: SyntaxScope,
}

impl CachedHighlight {
    fn relative(span: HighlightSpan, base: usize) -> Self {
        Self {
            start: u32::try_from(span.range.start.0 - base).expect("tree-sitter byte offset"),
            end: u32::try_from(span.range.end.0 - base).expect("tree-sitter byte offset"),
            scope: span.scope,
        }
    }
}

#[derive(Debug)]
struct HighlightBlock {
    kind_id: u16,
    range: Range<usize>,
    spans: Box<[CachedHighlight]>,
}

/// Relative spans retain their storage across edits; only block positions move.
#[derive(Debug, Default)]
pub struct HighlightCache {
    owner: Option<Arc<()>>,
    revision: u64,
    blocks: Vec<HighlightBlock>,
    span_ends: HashMap<usize, Box<[usize]>>,
}

impl HighlightCache {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(|block| block.spans.is_empty())
    }

    pub fn spans_in_range(&self, range: Range<usize>) -> impl Iterator<Item = HighlightSpan> + '_ {
        let first = self
            .blocks
            .partition_point(|block| block.range.end <= range.start);
        self.blocks[first..]
            .iter()
            .take_while(move |block| range.start < range.end && block.range.start < range.end)
            .flat_map(move |block| {
                let start = range.start.saturating_sub(block.range.start);
                let end = range.end.saturating_sub(block.range.start);
                let first = self.span_ends.get(&block.range.start).map_or(0, |ends| {
                    ends.partition_point(|last| *last <= start) * SPANS_PER_INDEX_ENTRY
                });
                let last = block
                    .spans
                    .partition_point(|span| (span.start as usize) < end);
                block.spans[first.min(last)..last]
                    .iter()
                    .filter(move |span| (span.end as usize) > start)
                    .map(move |span| HighlightSpan {
                        range: TextRange::new(
                            BufferOffset(block.range.start + span.start as usize),
                            BufferOffset(block.range.start + span.end as usize),
                        ),
                        scope: span.scope,
                    })
            })
    }

    pub fn update(&mut self, session: &SyntaxSession, source: &str, change: Option<&SyntaxChange>) {
        self.update_controlled(session, source, change, None)
            .expect("uncontrolled cache updates cannot be cancelled");
    }

    pub fn update_controlled(
        &mut self,
        session: &SyntaxSession,
        source: &str,
        change: Option<&SyntaxChange>,
        work: Option<&crate::SyntaxWork>,
    ) -> Result<(), crate::SyntaxError> {
        crate::work::checkpoint(work)?;
        if change.is_none()
            && self.revision == session.revision
            && self
                .owner
                .as_ref()
                .is_some_and(|owner| Arc::ptr_eq(owner, &session.cache_owner))
        {
            return Ok(());
        }
        // The shell queries have no source_file or cross-root patterns, so an
        // edit can be reapplied to the partition it touched.
        let partitioned = (0..session.queries.highlight.pattern_count()).all(|i| {
            session.queries.highlight.is_pattern_rooted(i)
                && !session.queries.highlight.is_pattern_non_local(i)
        });
        let mut reusable = partitioned
            && change.is_some_and(|change| {
                self.owner
                    .as_ref()
                    .is_some_and(|owner| Arc::ptr_eq(owner, &session.cache_owner))
                    && Arc::ptr_eq(&change.owner, &session.cache_owner)
                    && self.revision.checked_add(1) == Some(session.revision)
                    && change.revision == session.revision
            });
        if reusable && let Some(change) = change {
            let changed_bytes = change
                .structural_ranges()
                .map(|range| range.len())
                .sum::<usize>()
                .max(change.edit.new_end_byte - change.edit.start_byte);
            // Broad invalidations are cheaper as one query than thousands of
            // per-root queries. Repartition its result without mixing old spans.
            if changed_bytes > source.len() / 2 {
                reusable = false;
            }
        }
        if !reusable {
            // No old block can contribute to this result. Release it before
            // allocating the full query output and the replacement cache.
            self.clear();
        }
        let mut span_ends = HashMap::new();
        if !partitioned {
            self.blocks = vec![HighlightBlock {
                kind_id: 0,
                range: 0..source.len(),
                spans: session
                    .highlights_controlled(
                        source,
                        TextRange::new(BufferOffset(0), BufferOffset(source.len())),
                        work,
                    )?
                    .into_iter()
                    .map(|span| CachedHighlight::relative(span, 0))
                    .collect::<Vec<_>>()
                    .into(),
            }];
            if let Some(index) = index_span_ends(&self.blocks[0].spans) {
                span_ends.insert(0, index);
            }
        } else {
            let root = session.tree.root_node();
            let mut cursor = root.walk();
            let mut blocks = Vec::with_capacity(root.child_count());
            // Initial/full refresh uses one query rather than one query per node.
            let mut full = if reusable {
                None
            } else {
                Some(
                    session
                        .highlights_controlled(
                            source,
                            TextRange::new(BufferOffset(0), BufferOffset(source.len())),
                            work,
                        )?
                        .into_iter(),
                )
            };
            for node in root.children(&mut cursor) {
                crate::work::checkpoint(work)?;
                let range = node.byte_range();
                if range.is_empty() {
                    continue;
                }
                let old_block = change.filter(|_| reusable).and_then(|change| {
                    let old_start = change.unchanged_old_range(range.clone())?.start;
                    let index = self
                        .blocks
                        .partition_point(|block| block.range.start < old_start);
                    self.blocks.get_mut(index).filter(|block| {
                        block.range.start == old_start
                            && block.kind_id == node.kind_id()
                            && block.range.len() == range.len()
                    })
                });
                let spans = if let Some(block) = old_block {
                    if let Some(index) = self.span_ends.remove(&block.range.start) {
                        span_ends.insert(range.start, index);
                    }
                    std::mem::take(&mut block.spans)
                } else {
                    let spans: Box<[_]> = if let Some(full) = &mut full {
                        let count = full
                            .as_slice()
                            .partition_point(|span| span.range.start.0 < range.end);
                        full.by_ref()
                            .take(count)
                            .map(|span| CachedHighlight::relative(span, range.start))
                            .collect::<Vec<_>>()
                            .into()
                    } else {
                        session
                            .highlights_controlled(
                                source,
                                TextRange::new(BufferOffset(range.start), BufferOffset(range.end)),
                                work,
                            )?
                            .into_iter()
                            .map(|span| CachedHighlight::relative(span, range.start))
                            .collect::<Vec<_>>()
                            .into()
                    };
                    if let Some(index) = index_span_ends(&spans) {
                        span_ends.insert(range.start, index);
                    }
                    spans
                };
                blocks.push(HighlightBlock {
                    kind_id: node.kind_id(),
                    range,
                    spans,
                });
            }
            self.blocks = blocks;
        }
        self.span_ends = span_ends;
        self.owner = Some(session.cache_owner.clone());
        self.revision = session.revision;
        crate::work::checkpoint(work)
    }
}

// Prefix maxima preserve captures enclosing later spans. Index groups rather
// than every token to keep full-query languages compact as well.
fn index_span_ends(spans: &[CachedHighlight]) -> Option<Box<[usize]>> {
    if spans.len() <= SPANS_PER_INDEX_ENTRY {
        return None;
    }
    let mut maximum = 0;
    Some(
        spans
            .chunks(SPANS_PER_INDEX_ENTRY)
            .map(|chunk| {
                maximum = maximum.max(
                    chunk
                        .iter()
                        .map(|span| span.end as usize)
                        .max()
                        .unwrap_or(0),
                );
                maximum
            })
            .collect::<Vec<_>>()
            .into(),
    )
}

#[cfg(test)]
mod tests {
    
    




}
