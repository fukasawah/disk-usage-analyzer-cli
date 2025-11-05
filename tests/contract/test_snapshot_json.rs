//! JSON output from snapshot view matches schema

#[cfg(test)]
mod tests {
    use dua::cli::output::format_json;
    use dua::io::snapshot::{read_snapshot, write_snapshot};
    use dua::models::{DirectoryEntry, ProgressSnapshot, SnapshotMeta};
    use dua::{StrategyKind, Summary};
    use std::time::SystemTime;
    use tempfile::NamedTempFile;

    #[test]
    fn test_snapshot_json_serialization() {
        let temp_file = NamedTempFile::new().unwrap();
        let snapshot_path = temp_file.path().to_str().unwrap();

        let meta = SnapshotMeta {
            scan_root: "/test".to_string(),
            started_at: "2025-10-30T00:00:00Z".to_string(),
            finished_at: "2025-10-30T00:01:00Z".to_string(),
            size_basis: "physical".to_string(),
            hardlink_policy: "dedupe".to_string(),
            excludes: vec![],
            strategy: "posix".to_string(),
        };

        let entries = vec![DirectoryEntry {
            path: "/test/dir".to_string(),
            parent_path: Some("/test".to_string()),
            depth: 1,
            size_bytes: 5000,
            file_count: 3,
            dir_count: 1,
        }];

        write_snapshot(snapshot_path, &meta, &entries, &[]).unwrap();
        let (meta_out, entries, _errors) = read_snapshot(snapshot_path).unwrap();
        assert_eq!(meta_out.strategy, "posix");

        let meta_json = serde_json::to_value(&meta_out).unwrap();
        assert_eq!(meta_json["strategy"], "posix");

        // Serialize to JSON
        let json = serde_json::to_string(&entries).unwrap();

        // Verify JSON contains expected fields
        assert!(json.contains("path"));
        assert!(json.contains("size_bytes"));
        assert!(json.contains("file_count"));
        assert!(json.contains("dir_count"));
        assert!(json.contains("depth"));
        assert!(json.contains("parent_path"));

        let summary = Summary {
            root: meta_out.scan_root.clone(),
            entries: entries.clone(),
            errors: vec![],
            started_at: SystemTime::UNIX_EPOCH,
            finished_at: SystemTime::UNIX_EPOCH,
            strategy: StrategyKind::PosixOptimized,
            progress: vec![ProgressSnapshot {
                timestamp_ms: 0,
                processed_entries: 5,
                processed_bytes: 1024,
                estimated_completion_ratio: Some(0.5),
                recent_throughput_bytes_per_sec: Some(512),
            }],
            entry_count: entries.len() as u64,
        };

        let summary_json = format_json(&summary, &summary.entries);
        let summary_value: serde_json::Value = serde_json::from_str(&summary_json).unwrap();
        assert!(summary_value["progress"].is_array());
        let progress = summary_value["progress"].as_array().unwrap();
        assert_eq!(progress.len(), 1);
        let snapshot = &progress[0];
        assert!(snapshot.get("timestamp_ms").is_some());
        assert!(snapshot.get("processed_entries").is_some());
        assert!(snapshot.get("processed_bytes").is_some());
        assert!(snapshot.get("estimated_completion_ratio").is_some());
        assert!(snapshot.get("recent_throughput_bytes_per_sec").is_some());
    }
}
