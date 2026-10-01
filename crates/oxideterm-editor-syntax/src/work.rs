// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use crate::SyntaxError;
use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// Cooperative slices for an owned background worker. Lexers and sorts can
/// exceed a slice; publication cancellation must not depend on their latency.
pub struct SyntaxWork {
    generation: Arc<AtomicU64>,
    expected: u64,
    slice: Duration,
    deadline: Cell<Instant>,
}

impl SyntaxWork {
    pub fn new(generation: Arc<AtomicU64>, expected: u64, slice: Duration) -> Self {
        Self {
            generation,
            expected,
            slice,
            deadline: Cell::new(Instant::now() + slice),
        }
    }

    pub fn cancelled(&self) -> bool {
        self.generation.load(Ordering::Acquire) != self.expected
    }
    pub(crate) fn should_pause(&self) -> bool {
        self.cancelled() || Instant::now() >= self.deadline.get()
    }

    pub fn checkpoint(&self) -> Result<(), SyntaxError> {
        if self.cancelled() {
            return Err(SyntaxError::ParseCancelled);
        }
        if Instant::now() >= self.deadline.get() {
            std::thread::yield_now();
            self.deadline.set(Instant::now() + self.slice);
        }
        if self.cancelled() {
            return Err(SyntaxError::ParseCancelled);
        }
        Ok(())
    }
}

pub(crate) fn checkpoint(work: Option<&SyntaxWork>) -> Result<(), SyntaxError> {
    if let Some(work) = work {
        work.checkpoint()?;
    }
    Ok(())
}

pub(crate) fn parse(
    parser: &mut tree_sitter::Parser,
    source: &str,
    old: Option<&tree_sitter::Tree>,
    work: Option<&SyntaxWork>,
) -> Result<tree_sitter::Tree, SyntaxError> {
    loop {
        checkpoint(work)?;
        let mut pause = |_: &tree_sitter::ParseState| work.is_some_and(SyntaxWork::should_pause);
        if let Some(tree) = parser.parse_with_options(
            &mut |offset, _| source.as_bytes().get(offset..).unwrap_or_default(),
            old,
            Some(tree_sitter::ParseOptions::new().progress_callback(&mut pause)),
        ) {
            return Ok(tree);
        }
        if work.is_none() {
            return Err(SyntaxError::ParseCancelled);
        }
    }
}

#[cfg(test)]
mod tests {
    
    

}
