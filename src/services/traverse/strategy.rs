//! Strategy trait and registry helpers for filesystem traversal backends.

use super::{StrategyKind, TraversalContext};
use crate::ScanOptions;
use std::io;
use std::path::Path;

/// Common interface implemented by filesystem-specific traversal strategies.
pub trait TraversalStrategy {
    /// Identify the strategy for logging and diagnostics.
    fn kind(&self) -> StrategyKind;

    /// Determine whether the strategy may run under the supplied options.
    fn is_eligible(&self, opts: &ScanOptions) -> bool;

    /// Execute traversal for the provided root path, returning total byte size.
    fn traverse(&self, root: &Path, context: &mut TraversalContext) -> io::Result<u64>;
}

/// Placeholder registry type; will collect platform strategies in later phases.
#[derive(Default)]
pub struct StrategyRegistry<'a> {
    strategies: Vec<&'a dyn TraversalStrategy>,
}

impl<'a> StrategyRegistry<'a> {
    /// Construct an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }

    /// Register a strategy implementation.
    pub fn register(&mut self, strategy: &'a dyn TraversalStrategy) {
        self.strategies.push(strategy);
    }

    /// Select the first eligible strategy, falling back to legacy if needed.
    #[must_use]
    pub fn select(&'a self, opts: &ScanOptions) -> Option<&'a dyn TraversalStrategy> {
        self.strategies
            .iter()
            .copied()
            .find(|strategy| strategy.is_eligible(opts))
    }
}
