//! Disk Usage Analysis Library
//!
//! This library provides functionality to analyze disk usage for directory trees,
//! with support for different size bases (logical vs physical), hardlink deduplication,
//! and snapshot persistence via Parquet format.

pub mod cli;
pub mod io;
pub mod models;
pub mod services;

pub use models::{DirectoryEntry, ErrorItem, ProgressSnapshot, SnapshotMeta};
pub use services::traverse::progress::ProgressThrottler;
pub use services::traverse::strategy::{StrategyRegistry, TraversalStrategy};
pub use services::traverse::{StrategyKind, TraversalContext, TraversalDispatcher};

use crate::services::sink::SinkFinish;
use crate::services::sink::parquet::ParquetStreamSink;
use crate::services::traverse::progress::DEFAULT_BYTE_TRIGGER;
use std::path::Path;
use std::result;
use std::sync::Arc;
use std::time::Duration;

/// Shared notifier type used for reporting traversal progress snapshots.
pub type ProgressNotifier = Arc<dyn Fn(&ProgressSnapshot) + Send + Sync + 'static>;

/// Custom error type for the library
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    InvalidInput(String),
    PartialFailure { completed: usize, failed: usize },
    System(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
            Error::PartialFailure { completed, failed } => {
                write!(f, "Partial failure: {completed} completed, {failed} failed")
            }
            Error::System(msg) => write!(f, "System error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

pub type Result<T> = result::Result<T, Error>;

/// Options for scanning a directory
#[derive(Clone)]
pub struct ScanOptions {
    pub basis: SizeBasis,
    pub max_depth: Option<u16>,
    pub hardlink_policy: HardlinkPolicy,
    pub follow_symlinks: bool,
    pub cross_filesystem: bool,
    pub strategy_override: Option<StrategyKind>,
    pub progress_interval: Duration,
    pub progress_notifier: Option<ProgressNotifier>,
    pub progress_byte_trigger: u64,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            basis: SizeBasis::Physical,
            max_depth: None,
            hardlink_policy: HardlinkPolicy::Dedupe,
            follow_symlinks: false,
            cross_filesystem: false,
            strategy_override: None,
            progress_interval: Duration::from_secs(2),
            progress_notifier: None,
            progress_byte_trigger: DEFAULT_BYTE_TRIGGER,
        }
    }
}

impl std::fmt::Debug for ScanOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScanOptions")
            .field("basis", &self.basis)
            .field("max_depth", &self.max_depth)
            .field("hardlink_policy", &self.hardlink_policy)
            .field("follow_symlinks", &self.follow_symlinks)
            .field("cross_filesystem", &self.cross_filesystem)
            .field("strategy_override", &self.strategy_override)
            .field("progress_interval", &self.progress_interval)
            .field(
                "progress_notifier",
                &self.progress_notifier.as_ref().map(|_| "<configured>"),
            )
            .field("progress_byte_trigger", &self.progress_byte_trigger)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeBasis {
    Physical,
    Logical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardlinkPolicy {
    Dedupe,
    Count,
}

/// Summary result from a scan operation
#[derive(Debug)]
pub struct Summary {
    pub root: String,
    pub entries: Vec<DirectoryEntry>,
    pub errors: Vec<ErrorItem>,
    pub started_at: std::time::SystemTime,
    pub finished_at: std::time::SystemTime,
    pub strategy: StrategyKind,
    pub progress: Vec<ProgressSnapshot>,
    pub entry_count: u64,
}

/// Scan a directory and return a summary
///
/// # Arguments
/// * `root` - The root directory to scan
/// * `opts` - Scan options
///
/// # Returns
/// A Summary containing directory entries and any errors encountered
pub fn scan_summary<P: AsRef<Path>>(root: P, opts: &ScanOptions) -> Result<Summary> {
    let root_path = root.as_ref().to_string_lossy().to_string();

    if !root.as_ref().exists() {
        return Err(Error::InvalidInput(format!(
            "Path does not exist: {root_path}"
        )));
    }

    if !root.as_ref().is_dir() {
        return Err(Error::InvalidInput(format!(
            "Path is not a directory: {root_path}"
        )));
    }

    let started_at = std::time::SystemTime::now();

    // Create traversal context
    let mut context = services::traverse::TraversalContext::new(opts.clone(), opts.max_depth);
    let dispatcher = services::traverse::TraversalDispatcher::for_platform(opts);

    // Traverse the directory tree
    let _ = dispatcher.traverse(&root, &mut context)?;
    context.finalize_progress();

    // Extract entries and errors
    let (sink_finish, progress, strategy) = context.into_parts()?;
    let SinkFinish {
        entries,
        errors,
        entry_count,
    } = sink_finish;

    let finished_at = std::time::SystemTime::now();

    Ok(Summary {
        root: root_path,
        entries,
        errors,
        started_at,
        finished_at,
        strategy,
        progress,
        entry_count,
    })
}

/// Scan a directory and stream results directly into a Parquet snapshot.
pub fn scan_to_snapshot<P: AsRef<Path>>(
    root: P,
    opts: &ScanOptions,
    snapshot_path: &str,
) -> Result<Summary> {
    let root_path = root.as_ref().to_string_lossy().to_string();

    if !root.as_ref().exists() {
        return Err(Error::InvalidInput(format!(
            "Path does not exist: {root_path}"
        )));
    }

    if !root.as_ref().is_dir() {
        return Err(Error::InvalidInput(format!(
            "Path is not a directory: {root_path}"
        )));
    }

    let started_at = std::time::SystemTime::now();

    let sink = ParquetStreamSink::try_new(snapshot_path, None)?;
    let mut context = services::traverse::TraversalContext::with_sink(
        opts.clone(),
        opts.max_depth,
        Box::new(sink),
    );
    let dispatcher = services::traverse::TraversalDispatcher::for_platform(opts);

    let _ = dispatcher.traverse(&root, &mut context)?;
    context.finalize_progress();

    let finished_at = std::time::SystemTime::now();
    let strategy_active = context.strategy();

    let meta = SnapshotMeta {
        scan_root: root_path.clone(),
        started_at: format!("{started_at:?}"),
        finished_at: format!("{finished_at:?}"),
        size_basis: match opts.basis {
            SizeBasis::Physical => "physical".to_string(),
            SizeBasis::Logical => "logical".to_string(),
        },
        hardlink_policy: match opts.hardlink_policy {
            HardlinkPolicy::Dedupe => "dedupe".to_string(),
            HardlinkPolicy::Count => "count".to_string(),
        },
        excludes: Vec::new(),
        strategy: strategy_active.to_string(),
    };

    context.set_sink_metadata(&meta)?;

    let (sink_finish, progress, strategy) = context.into_parts()?;
    let SinkFinish {
        entries,
        errors,
        entry_count,
    } = sink_finish;

    Ok(Summary {
        root: root_path,
        entries,
        errors,
        started_at,
        finished_at,
        strategy,
        progress,
        entry_count,
    })
}
