//! Streaming aggregation for computing directory totals

use crate::models::DirectoryEntry;

/// Sort entries by a specified field
#[derive(Debug, Clone, Copy)]
pub enum SortBy {
    Size,
    Files,
    Dirs,
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
