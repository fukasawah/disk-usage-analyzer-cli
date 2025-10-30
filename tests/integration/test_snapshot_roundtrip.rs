//! Snapshot write/read round-trip test

#[cfg(test)]
mod tests {
    use dua::io::snapshot::{read_snapshot, write_snapshot};
    use dua::models::{DirectoryEntry, ErrorItem, SnapshotMeta};
    use tempfile::NamedTempFile;

    #[test]
    fn test_snapshot_roundtrip() {
        let temp_file = NamedTempFile::new().unwrap();
        let snapshot_path = temp_file.path().to_str().unwrap();

        // Create test data
        let meta = SnapshotMeta {
            scan_root: "/test/root".to_string(),
            started_at: "2025-10-30T00:00:00Z".to_string(),
            finished_at: "2025-10-30T00:01:00Z".to_string(),
            size_basis: "physical".to_string(),
            hardlink_policy: "dedupe".to_string(),
            excludes: vec![],
        };

        let entries = vec![
            DirectoryEntry {
                path: "/test/root/dir1".to_string(),
                parent_path: Some("/test/root".to_string()),
                depth: 1,
                size_bytes: 1024,
                file_count: 5,
                dir_count: 2,
            },
            DirectoryEntry {
                path: "/test/root/dir2".to_string(),
                parent_path: Some("/test/root".to_string()),
                depth: 1,
                size_bytes: 2048,
                file_count: 10,
                dir_count: 3,
            },
        ];

        let errors = vec![ErrorItem {
            path: "/test/root/forbidden".to_string(),
            code: "EACCES".to_string(),
            message: "Permission denied".to_string(),
        }];

        // Write snapshot
        let write_result = write_snapshot(snapshot_path, &meta, &entries, &errors);
        assert!(
            write_result.is_ok(),
            "Failed to write snapshot: {:?}",
            write_result.err()
        );

        // Read snapshot
        let read_result = read_snapshot(snapshot_path);
        assert!(
            read_result.is_ok(),
            "Failed to read snapshot: {:?}",
            read_result.err()
        );

        let (read_meta, read_entries, read_errors) = read_result.unwrap();

        // Verify metadata
        assert_eq!(read_meta.scan_root, meta.scan_root);
        assert_eq!(read_meta.size_basis, meta.size_basis);
        assert_eq!(read_meta.hardlink_policy, meta.hardlink_policy);

        // Verify entries
        assert_eq!(read_entries.len(), entries.len());
        for (orig, read) in entries.iter().zip(read_entries.iter()) {
            assert_eq!(read.path, orig.path);
            assert_eq!(read.size_bytes, orig.size_bytes);
            assert_eq!(read.file_count, orig.file_count);
            assert_eq!(read.dir_count, orig.dir_count);
        }

        // Verify errors
        assert_eq!(read_errors.len(), errors.len());
        assert_eq!(read_errors[0].path, errors[0].path);
        assert_eq!(read_errors[0].code, errors[0].code);
    }

    #[test]
    fn test_empty_snapshot() {
        let temp_file = NamedTempFile::new().unwrap();
        let snapshot_path = temp_file.path().to_str().unwrap();

        let meta = SnapshotMeta {
            scan_root: "/empty".to_string(),
            started_at: "2025-10-30T00:00:00Z".to_string(),
            finished_at: "2025-10-30T00:00:01Z".to_string(),
            size_basis: "logical".to_string(),
            hardlink_policy: "count".to_string(),
            excludes: vec![],
        };

        let write_result = write_snapshot(snapshot_path, &meta, &[], &[]);
        assert!(write_result.is_ok());

        let read_result = read_snapshot(snapshot_path);
        assert!(read_result.is_ok());

        let (_, entries, errors) = read_result.unwrap();
        assert_eq!(entries.len(), 0);
        assert_eq!(errors.len(), 0);
    }
}
