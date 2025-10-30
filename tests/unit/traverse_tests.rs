//! Unit tests for traversal filtering (symlinks, filesystem boundaries)

#[cfg(test)]
mod tests {
    use rs_disk_usage::{ScanOptions, SizeBasis, HardlinkPolicy};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_basic_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test structure
        fs::create_dir_all(root.join("subdir")).unwrap();
        fs::write(root.join("file1.txt"), b"hello").unwrap();
        fs::write(root.join("subdir/file2.txt"), b"world").unwrap();

        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            max_depth: None,
            hardlink_policy: HardlinkPolicy::Dedupe,
            follow_symlinks: false,
            cross_filesystem: false,
        };

        let result = rs_disk_usage::scan_summary(root, &opts);
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert!(!summary.entries.is_empty());
        assert_eq!(summary.errors.len(), 0);
    }

    #[test]
    fn test_invalid_path() {
        let opts = ScanOptions::default();
        let result = rs_disk_usage::scan_summary("/nonexistent/path/12345", &opts);
        assert!(result.is_err());
    }
}
