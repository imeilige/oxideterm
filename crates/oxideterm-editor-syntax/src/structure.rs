// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use std::{collections::BTreeMap, ops::Range, sync::Arc};

use crate::indent_index::IndentGuideIndex;

use crate::{IndentGuide, SyntaxChange, SyntaxSession, folding, indent};

#[derive(Debug)]
struct StructureBlock {
    range: Range<usize>,
    line: usize,
    kind: u16,
    folds: Box<[(usize, usize)]>,
    guide_index: IndentGuideIndex,
}

#[derive(Debug, Default)]
pub struct StructureCache {
    owner: Option<Arc<()>>,
    revision: u64,
    tab_size: usize,
    blocks: Vec<StructureBlock>,
}

impl StructureCache {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn fold_lines(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.blocks.iter().flat_map(|block| {
            block
                .folds
                .iter()
                .map(move |&(start, end)| (block.line + start, block.line + end))
        })
    }

    pub fn fold_at_line(&self, line: usize) -> Option<(usize, usize)> {
        let index = self
            .blocks
            .partition_point(|block| block.line <= line)
            .checked_sub(1)?;
        let block = &self.blocks[index];
        let local = line - block.line;
        let index = block
            .folds
            .binary_search_by_key(&local, |&(start, _)| start)
            .ok()?;
        let (start, end) = block.folds[index];
        Some((block.line + start, block.line + end))
    }

    pub fn columns_for_line(&self, line: usize) -> Vec<usize> {
        // Guides exclude their header and include their closing line. A later
        // root child starting on that closing line must not hide the old guide.
        let Some(index) = self
            .blocks
            .partition_point(|block| block.line < line)
            .checked_sub(1)
        else {
            return Vec::new();
        };
        let block = &self.blocks[index];
        block.guide_index.columns_for_line(line - block.line)
    }

    pub fn update(
        &mut self,
        session: &SyntaxSession,
        source: &str,
        tab_size: usize,
        change: Option<&SyntaxChange>,
    ) {
        self.update_controlled(session, source, tab_size, change, None)
            .expect("uncontrolled structure updates cannot be cancelled");
    }

    pub fn update_controlled(
        &mut self,
        session: &SyntaxSession,
        source: &str,
        tab_size: usize,
        change: Option<&SyntaxChange>,
        work: Option<&crate::SyntaxWork>,
    ) -> Result<(), crate::SyntaxError> {
        crate::work::checkpoint(work)?;
        let tab_size = tab_size.max(1);
        let same_owner = self
            .owner
            .as_ref()
            .is_some_and(|owner| Arc::ptr_eq(owner, &session.cache_owner));
        if same_owner && self.revision == session.revision && self.tab_size == tab_size {
            return Ok(());
        }
        let reusable = same_owner
            && self.tab_size == tab_size
            && change.is_some_and(|change| {
                Arc::ptr_eq(&change.owner, &session.cache_owner)
                    && change.revision == session.revision
                    && self.revision.checked_add(1) == Some(session.revision)
            });
        let root = session.tree.root_node();
        let mut cursor = root.walk();
        // Shell root children are independent for these collectors.
        let nodes: Vec<_> = if true {
            root.children(&mut cursor)
                .filter(|node| node.end_position().row > node.start_position().row)
                .collect()
        } else {
            vec![root]
        };
        let mut blocks = Vec::with_capacity(nodes.len());
        for node in nodes {
            crate::work::checkpoint(work)?;
            let range = node.byte_range();
            let line = node.start_position().row;
            let old = change.filter(|_| reusable).and_then(|change| {
                let old_range = change.unchanged_old_range(range.clone())?;
                let i = self
                    .blocks
                    .partition_point(|block| block.range.start < old_range.start);
                self.blocks
                    .get_mut(i)
                    .filter(|block| block.range == old_range && block.kind == node.kind_id())
            });
            let (folds, guide_index) = if let Some(old) = old {
                (
                    std::mem::take(&mut old.folds),
                    std::mem::take(&mut old.guide_index),
                )
            } else {
                // The gutter needs one widest fold per header, not byte ranges
                // or duplicate function/body nodes describing the same fold.
                let mut folds = BTreeMap::<usize, usize>::new();
                for fold in folding::fold_ranges_controlled(node, work)? {
                    folds
                        .entry(fold.start_line - line)
                        .and_modify(|end| *end = (*end).max(fold.end_line - line))
                        .or_insert(fold.end_line - line);
                }
                let folds = folds.into_iter().collect::<Vec<_>>().into_boxed_slice();
                let guides = indent::indent_guides_controlled(node, source, tab_size, work)?
                    .into_iter()
                    .map(|guide| IndentGuide {
                        start_line: guide.start_line - line,
                        end_line: guide.end_line - line,
                        column: guide.column,
                    })
                    .collect::<Vec<_>>();
                let guide_index = IndentGuideIndex::new(guides);
                (folds, guide_index)
            };
            if !folds.is_empty() || !guide_index.is_empty() {
                blocks.push(StructureBlock {
                    range,
                    line,
                    kind: node.kind_id(),
                    folds,
                    guide_index,
                });
            }
        }
        self.blocks = blocks;
        self.owner = Some(session.cache_owner.clone());
        self.revision = session.revision;
        self.tab_size = tab_size;
        crate::work::checkpoint(work)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    fn normalized_folds(session: &SyntaxSession) -> Vec<(usize, usize)> {
        let mut folds = BTreeMap::<usize, usize>::new();
        for fold in session.fold_ranges() {
            folds
                .entry(fold.start_line)
                .and_modify(|end| *end = (*end).max(fold.end_line))
                .or_insert(fold.end_line);
        }
        folds.into_iter().collect()
    }
}
