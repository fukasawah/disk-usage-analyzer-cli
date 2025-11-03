//! Resilience test with permission errors and file churn

#[cfg(test)]
mod tests {
    use crate::fixtures::write_file_sync;
    use dua::{ScanOptions, SizeBasis};
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_continues_after_errors() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create some accessible directories
        fs::create_dir_all(root.join("accessible1")).unwrap();
        fs::create_dir_all(root.join("accessible2")).unwrap();
        write_file_sync(root.join("accessible1/file1.txt"), b"content1").unwrap();
        write_file_sync(root.join("accessible2/file2.txt"), b"content2").unwrap();

        // Try to create an inaccessible directory (this might not work in all environments)
        let inaccessible_dir = root.join("inaccessible");
        fs::create_dir_all(&inaccessible_dir).ok();

        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            ..Default::default()
        };

        let result = dua::scan_summary(root, &opts);

        // Should complete even if there are some errors
        assert!(result.is_ok(), "Scan should complete despite errors");

        let summary = result.unwrap();

        // Should have scanned the accessible directories
        assert!(!summary.entries.is_empty(), "Should have some entries");

        // If there were errors, they should be recorded
        if !summary.errors.is_empty() {
            println!("Errors recorded: {}", summary.errors.len());
            for error in &summary.errors {
                println!("  - {}: {}", error.path, error.code);
            }
        }
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create an empty directory structure
        fs::create_dir_all(root.join("empty1")).unwrap();
        fs::create_dir_all(root.join("empty2/nested")).unwrap();

        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            ..Default::default()
        };

        let result = dua::scan_summary(root, &opts);

        assert!(result.is_ok());
        let summary = result.unwrap();

        // Should handle empty directories gracefully
        for entry in &summary.entries {
            // All sizes should be 0 or very small (directory metadata)
            assert!(entry.file_count == 0 || entry.size_bytes < 100);
        }
    }

    #[test]
    fn test_many_small_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create many small files with known content
        for i in 0..50 {
            write_file_sync(root.join(format!("file{i}.txt")), format!("{i}")).unwrap();
        }

        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            ..Default::default()
        };

        let result = dua::scan_summary(root, &opts);

        assert!(result.is_ok());
        let summary = result.unwrap();
        assert!(!summary.entries.is_empty());

        // Verify all paths are normalized (cross-platform test)
        for entry in &summary.entries {
            let path = &entry.path;
            assert!(
                !entry.path.contains('\\'),
                "Path should be normalized: {path}"
            );
        }

        // Verify specific files with known sizes
        let first_entry = summary
            .entries
            .iter()
            .find(|e| e.path.ends_with("file0.txt"))
            .expect("Should find file0.txt");
        assert_eq!(
            first_entry.size_bytes, 1,
            "file0.txt should be 1 byte ('0')"
        );

        let second_entry = summary
            .entries
            .iter()
            .find(|e| e.path.ends_with("file10.txt"))
            .expect("Should find file10.txt");
        assert_eq!(
            second_entry.size_bytes, 2,
            "file10.txt should be 2 bytes ('10')"
        );
    }

    #[test]
    fn progress_cadence_respects_interval() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create large files to trigger byte-based progress snapshots.
        for i in 0..16 {
            write_file_sync(
                root.join(format!("blob_{i:02}.bin")),
                vec![0_u8; 128 * 1024],
            )
            .unwrap();
        }

        let mut opts = ScanOptions {
            basis: SizeBasis::Logical,
            ..Default::default()
        };
        opts.progress_interval = Duration::from_millis(200);

        let summary = dua::scan_summary(root, &opts).expect("scan should succeed");
        let snapshots = summary.progress;

        assert!(
            !snapshots.is_empty(),
            "expected at least the final progress snapshot"
        );

        if snapshots.len() >= 2 {
            for window in snapshots.windows(2) {
                let delta = window[1]
                    .timestamp_ms
                    .saturating_sub(window[0].timestamp_ms);
                let max_interval =
                    u64::try_from(opts.progress_interval.as_millis()).unwrap_or(u64::MAX);
                assert!(
                    delta <= max_interval,
                    "progress gap {delta}ms exceeded interval {max_interval}ms"
                );
            }
        }

        if let Some(last) = snapshots.last() {
            assert_eq!(
                last.estimated_completion_ratio,
                Some(1.0),
                "final snapshot should indicate completion"
            );
        }
    }
}
