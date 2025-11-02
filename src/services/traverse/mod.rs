//! Traversal dispatcher and strategy coordination layer.
//!
//! This module orchestrates filesystem traversal strategies across supported
//! platforms. The dispatcher selects the appropriate backend (legacy, Windows
//! optimized, POSIX optimized, etc.) based on runtime context and user
//! overrides while enforcing shared invariants:
//!
//! - Directory traversal MUST respect `ScanOptions` boundaries (max-depth,
//!   filesystem crossing, hardlink policy).
//! - Aggregated results MUST match legacy traversal within 1% or 10 MB.
//! - Progress emitters MUST remain monotonic and never regress when reported.

pub mod detect;
pub mod legacy;
pub mod posix;
pub mod progress;
pub mod strategy;
pub mod windows;

pub use legacy::TraversalContext;

use crate::ScanOptions;
use std::path::Path;
use strategy::TraversalStrategy;

/// Enumeration of available traversal strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrategyKind {
    /// Pre-optimization traversal used as fallback and regression oracle.
    #[default]
    Legacy,
    /// Windows NTFS optimized traversal leveraging large-fetch APIs.
    WindowsOptimized,
    /// POSIX optimized traversal leveraging `openat`/`getdents64`.
    PosixOptimized,
}

impl StrategyKind {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            StrategyKind::Legacy => "legacy",
            StrategyKind::WindowsOptimized => "windows",
            StrategyKind::PosixOptimized => "posix",
        }
    }

    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        match label.to_ascii_lowercase().as_str() {
            "legacy" => Some(StrategyKind::Legacy),
            "windows" | "ntfs" => Some(StrategyKind::WindowsOptimized),
            "posix" | "unix" => Some(StrategyKind::PosixOptimized),
            _ => None,
        }
    }
}

impl std::fmt::Display for StrategyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for StrategyKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        StrategyKind::from_label(s).ok_or_else(|| format!("unknown strategy '{s}'"))
    }
}

/// Traversal dispatcher responsible for selecting and executing the appropriate strategy.
#[derive(Debug)]
pub struct TraversalDispatcher {
    strategy: StrategyKind,
    explicit_override: bool,
    progress_interval: std::time::Duration,
}

impl Default for TraversalDispatcher {
    fn default() -> Self {
        Self {
            strategy: StrategyKind::Legacy,
            explicit_override: false,
            progress_interval: std::time::Duration::from_secs(2),
        }
    }
}

impl TraversalDispatcher {
    /// Construct a dispatcher using the provided strategy kind.
    #[must_use]
    pub fn with_strategy(strategy: StrategyKind, explicit_override: bool) -> Self {
        Self {
            strategy,
            explicit_override,
            progress_interval: std::time::Duration::from_secs(2),
        }
    }

    /// Derive the best-fit strategy for the current platform and options.
    #[must_use]
    pub fn for_platform(opts: &ScanOptions) -> Self {
        if let Some(override_kind) = opts.strategy_override {
            let mut dispatcher = Self::with_strategy(override_kind, true);
            dispatcher.progress_interval = opts.progress_interval;
            return dispatcher;
        }

        let default_strategy = detect::default_strategy();
        let mut dispatcher = Self::with_strategy(default_strategy, false);
        dispatcher.progress_interval = opts.progress_interval;
        dispatcher
    }

    /// Expose the currently selected strategy.
    #[must_use]
    pub fn active_strategy(&self) -> StrategyKind {
        self.strategy
    }

    /// Retrieve the configured progress emission interval.
    #[must_use]
    pub fn progress_interval(&self) -> std::time::Duration {
        self.progress_interval
    }

    /// Execute traversal for the supplied root path using the configured strategy.
    pub fn traverse<P: AsRef<Path>>(
        &self,
        root: P,
        context: &mut TraversalContext,
    ) -> std::io::Result<u64> {
        let root_ref = root.as_ref();
        let resolved = self.resolve_strategy(root_ref);

        if self.explicit_override && resolved != self.strategy {
            log::warn!(
                "Requested traversal strategy '{}' unsupported; falling back to '{}'",
                self.strategy,
                resolved,
            );
        }

        context.set_strategy(resolved);
        context.progress_interval = self.progress_interval;

        match resolved {
            StrategyKind::Legacy => legacy::traverse_directory(root_ref, context),
            StrategyKind::WindowsOptimized => {
                let strategy = windows::WindowsTraversal;
                strategy.traverse(root_ref, context)
            }
            StrategyKind::PosixOptimized => {
                let strategy = posix::PosixTraversal;
                strategy.traverse(root_ref, context)
            }
        }
    }

    fn resolve_strategy(&self, root: &Path) -> StrategyKind {
        if self.explicit_override {
            return Self::ensure_supported(self.strategy);
        }

        let fs_kind = detect::filesystem_kind_for_path(root);
        let preferred = detect::strategy_for_filesystem(fs_kind);
        Self::ensure_supported(preferred)
    }

    fn ensure_supported(kind: StrategyKind) -> StrategyKind {
        match kind {
            StrategyKind::WindowsOptimized if windows::WindowsTraversal::is_supported() => {
                StrategyKind::WindowsOptimized
            }
            StrategyKind::PosixOptimized if posix::PosixTraversal::is_supported() => {
                StrategyKind::PosixOptimized
            }
            _ => StrategyKind::Legacy,
        }
    }
}
