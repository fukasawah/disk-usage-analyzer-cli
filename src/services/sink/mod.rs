//! Scan sinks for handling traversal output without retaining everything in-memory.

use crate::{DirectoryEntry, ErrorItem, SnapshotMeta};
use std::io;

/// Aggregated result returned by a sink after traversal completes.
#[derive(Default)]
pub struct SinkFinish {
    pub entries: Vec<DirectoryEntry>,
    pub errors: Vec<ErrorItem>,
    pub entry_count: u64,
}

impl SinkFinish {
    #[must_use]
    pub fn new(entries: Vec<DirectoryEntry>, errors: Vec<ErrorItem>, entry_count: u64) -> Self {
        Self {
            entries,
            errors,
            entry_count,
        }
    }
}

/// Trait implemented by traversal sinks that receive entries and errors.
pub trait ScanSink: Send {
    /// Record a directory entry produced during traversal.
    fn record_entry(&mut self, entry: DirectoryEntry) -> io::Result<()>;

    /// Record an error encountered during traversal.
    fn record_error(&mut self, error: ErrorItem) -> io::Result<()>;

    /// Provide snapshot metadata prior to finalization.
    fn set_metadata(&mut self, _meta: &SnapshotMeta) -> io::Result<()> {
        Ok(())
    }

    /// Finalize the sink once traversal completes.
    fn finish(self: Box<Self>) -> io::Result<SinkFinish>;
}

pub mod memory;
pub mod parquet;
