//! Data models for directory entries, snapshot metadata, and errors

use serde::{Deserialize, Serialize};

/// Represents a directory entry in the scan results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub path: String,
    pub parent_path: Option<String>,
    pub depth: u16,
    pub size_bytes: u64,
    pub file_count: u32,
    pub dir_count: u32,
}

/// Metadata for a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub scan_root: String,
    pub started_at: String,  // RFC3339 format
    pub finished_at: String, // RFC3339 format
    pub size_basis: String,
    pub hardlink_policy: String,
    pub excludes: Vec<String>,
}

/// Represents an error encountered during scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorItem {
    pub path: String,
    pub code: String,
    pub message: String,
}
