//! Streaming aggregation for computing directory totals

use crate::models::DirectoryEntry;
use std::collections::HashMap;

/// Sort entries by a specified field
#[derive(Debug, Clone, Copy)]
pub enum SortBy {
    Size,
    Files,
    Dirs,
}

/// Entry classification used when folding traversal shards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
}

/// Aggregated totals accumulated across traversal threads.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AggregateTotals {
    pub size_bytes: u64,
    pub files: u64,
    pub directories: u64,
}

impl AggregateTotals {
    /// Record a file contribution.
    pub fn record_file(&mut self, size_bytes: u64) {
        self.size_bytes += size_bytes;
        self.files += 1;
    }

    /// Record a directory contribution.
    pub fn record_directory(&mut self, size_bytes: u64) {
        self.size_bytes += size_bytes;
        self.directories += 1;
    }

    /// Merge another totals snapshot into this one.
    pub fn merge(&mut self, other: &AggregateTotals) {
        self.size_bytes += other.size_bytes;
        self.files += other.files;
        self.directories += other.directories;
    }
}

/// Thread-local shard used by parallel traversal to accumulate entries and totals.
#[derive(Debug, Default)]
pub struct DirectoryShard {
    entries: HashMap<String, DirectoryEntry>,
    totals: AggregateTotals,
}

impl DirectoryShard {
    /// Create an empty shard with optional capacity hint.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            totals: AggregateTotals::default(),
        }
    }

    /// Insert or replace an entry while accounting for its classification.
    pub fn absorb_entry(&mut self, entry: DirectoryEntry, kind: EntryKind) {
        match kind {
            EntryKind::File => self.totals.record_file(entry.size_bytes),
            EntryKind::Directory => self.totals.record_directory(entry.size_bytes),
        }

        self.entries.insert(entry.path.clone(), entry);
    }

    /// Extend the shard with a batch of entries.
    pub fn extend<I>(&mut self, entries: I)
    where
        I: IntoIterator<Item = (DirectoryEntry, EntryKind)>,
    {
        for (entry, kind) in entries {
            self.absorb_entry(entry, kind);
        }
    }

    /// Combine another shard into this one, consuming the source.
    pub fn merge_in_place(&mut self, other: DirectoryShard) {
        self.totals.merge(&other.totals);

        for (path, entry) in other.entries {
            self.entries.insert(path, entry);
        }
    }

    /// Consume the shard and return its entries and totals.
    #[must_use]
    pub fn into_parts(self) -> (Vec<DirectoryEntry>, AggregateTotals) {
        (self.entries.into_values().collect(), self.totals)
    }

    /// Borrow the current totals without consuming the shard.
    #[must_use]
    pub fn totals(&self) -> &AggregateTotals {
        &self.totals
    }
}

impl From<DirectoryShard> for AggregateTotals {
    fn from(shard: DirectoryShard) -> Self {
        shard.totals
    }
}

/// Consolidate multiple shards produced by parallel traversal into a single set of entries and totals.
#[must_use]
pub fn consolidate_shards<I>(shards: I) -> (Vec<DirectoryEntry>, AggregateTotals)
where
    I: IntoIterator<Item = DirectoryShard>,
{
    let mut accumulator = DirectoryShard::default();

    for shard in shards {
        accumulator.merge_in_place(shard);
    }

    accumulator.into_parts()
}

/// Sort and limit entries to top K
#[must_use]
pub fn sort_and_limit(
    mut entries: Vec<DirectoryEntry>,
    sort_by: SortBy,
    top_k: Option<usize>,
) -> Vec<DirectoryEntry> {
    // Sort entries
    match sort_by {
        SortBy::Size => {
            entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        }
        SortBy::Files => {
            entries.sort_by(|a, b| b.file_count.cmp(&a.file_count));
        }
        SortBy::Dirs => {
            entries.sort_by(|a, b| b.dir_count.cmp(&a.dir_count));
        }
    }

    // Truncate to top K if specified
    if let Some(k) = top_k {
        entries.truncate(k);
    }

    entries
}

/// Get immediate children of a directory (depth = `parent_depth` + 1)
#[must_use]
pub fn get_immediate_children(
    all_entries: &[DirectoryEntry],
    parent_path: &str,
    parent_depth: u16,
) -> Vec<DirectoryEntry> {
    let target_depth = parent_depth + 1;

    all_entries
        .iter()
        .filter(|e| {
            e.depth == target_depth && e.parent_path.as_ref().is_some_and(|p| p == parent_path)
        })
        .cloned()
        .collect()
}
