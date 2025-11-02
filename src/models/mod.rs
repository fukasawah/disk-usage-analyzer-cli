//! Data models for directory entries, snapshot metadata, and errors

use serde::{Deserialize, Serialize};

/// Snapshot of traversal progress emitted during a scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressSnapshot {
    /// Milliseconds elapsed since the scan started.
    pub timestamp_ms: u64,
    /// Cumulative count of files and directories processed.
    pub processed_entries: u64,
    /// Cumulative logical bytes attributed to processed files.
    pub processed_bytes: u64,
    /// Optional estimated completion ratio between 0.0 and 1.0.
    pub estimated_completion_ratio: Option<f32>,
    /// Optional rolling throughput estimate in bytes per second.
    pub recent_throughput_bytes_per_sec: Option<u64>,
}

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
    pub strategy: String,
}

/// Represents an error encountered during scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorItem {
    pub path: String,
    pub code: String,
    pub message: String,
}
