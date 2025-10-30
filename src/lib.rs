//! Disk Usage Analysis Library
//!
//! This library provides functionality to analyze disk usage for directory trees,
//! with support for different size bases (logical vs physical), hardlink deduplication,
//! and snapshot persistence via Parquet format.

pub mod cli;
pub mod io;
pub mod models;
pub mod services;

pub use models::{DirectoryEntry, ErrorItem, SnapshotMeta};

use std::path::Path;
use std::result;

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
#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub basis: SizeBasis,
    pub max_depth: Option<u16>,
    pub hardlink_policy: HardlinkPolicy,
    pub follow_symlinks: bool,
    pub cross_filesystem: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            basis: SizeBasis::Physical,
            max_depth: None,
            hardlink_policy: HardlinkPolicy::Dedupe,
            follow_symlinks: false,
            cross_filesystem: false,
        }
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

    // Traverse the directory tree
    let _ = services::traverse::traverse_directory(&root, &mut context)?;

    // Extract entries and errors
    let entries: Vec<DirectoryEntry> = context.entries.into_values().collect();
    let errors = context.errors;

    let finished_at = std::time::SystemTime::now();

    Ok(Summary {
        root: root_path,
        entries,
        errors,
        started_at,
        finished_at,
    })
}
