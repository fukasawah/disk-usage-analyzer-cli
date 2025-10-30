//! JSON output from snapshot view matches schema

#[cfg(test)]
mod tests {
    use rs_disk_usage::io::snapshot::{write_snapshot, read_snapshot};
    use rs_disk_usage::models::{DirectoryEntry, SnapshotMeta};
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
        };

        let entries = vec![
            DirectoryEntry {
                path: "/test/dir".to_string(),
                parent_path: Some("/test".to_string()),
                depth: 1,
                size_bytes: 5000,
                file_count: 3,
                dir_count: 1,
            },
        ];

        write_snapshot(snapshot_path, &meta, &entries, &[]).unwrap();
        let (_meta, entries, _errors) = read_snapshot(snapshot_path).unwrap();

        // Serialize to JSON
        let json = serde_json::to_string(&entries).unwrap();
        
        // Verify JSON contains expected fields
        assert!(json.contains("path"));
        assert!(json.contains("size_bytes"));
        assert!(json.contains("file_count"));
        assert!(json.contains("dir_count"));
        assert!(json.contains("depth"));
        assert!(json.contains("parent_path"));
    }
}
