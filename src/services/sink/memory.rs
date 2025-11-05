//! In-memory sink retaining traversal results for callers that need full materialization.

use super::{ScanSink, SinkFinish};
use crate::{DirectoryEntry, ErrorItem, SnapshotMeta};
use std::collections::HashMap;
use std::io;

#[derive(Default)]
pub struct MemorySink {
    entries: HashMap<String, DirectoryEntry>,
    errors: Vec<ErrorItem>,
    entry_count: u64,
    metadata: Option<SnapshotMeta>,
}

impl MemorySink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ScanSink for MemorySink {
    fn record_entry(&mut self, entry: DirectoryEntry) -> io::Result<()> {
        self.entry_count = self.entry_count.saturating_add(1);
        self.entries.insert(entry.path.clone(), entry);
        Ok(())
    }

    fn record_error(&mut self, error: ErrorItem) -> io::Result<()> {
        self.errors.push(error);
        Ok(())
    }

    fn set_metadata(&mut self, meta: &SnapshotMeta) -> io::Result<()> {
        self.metadata = Some(meta.clone());
        Ok(())
    }

    fn finish(self: Box<Self>) -> io::Result<SinkFinish> {
        let mut entries: Vec<DirectoryEntry> = self.entries.into_values().collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(SinkFinish::new(entries, self.errors, self.entry_count))
    }
}
