//! Integration test for drill command

#[cfg(test)]
mod tests {
    use rs_disk_usage::{ScanOptions, SizeBasis};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_drill_equivalence() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test structure
        fs::create_dir_all(root.join("subdir/nested")).unwrap();
        fs::write(root.join("file1.txt"), b"root file").unwrap();
        fs::write(root.join("subdir/file2.txt"), b"subdir file").unwrap();
        fs::write(root.join("subdir/nested/file3.txt"), b"nested file").unwrap();

        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            ..Default::default()
        };

        // Scan the root
        let root_summary = rs_disk_usage::scan_summary(root, &opts).unwrap();
        
        // Scan the subdirectory directly
        let subdir_path = root.join("subdir");
        let subdir_summary = rs_disk_usage::scan_summary(&subdir_path, &opts).unwrap();

        // Both should succeed
        assert!(!root_summary.entries.is_empty());
        assert!(!subdir_summary.entries.is_empty());
        
        // The subdirectory scan should have the subdir as root
        assert_eq!(subdir_summary.root, subdir_path.to_string_lossy());
    }

    #[test]
    fn test_drill_with_depth() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create deeper structure
        fs::create_dir_all(root.join("a/b/c")).unwrap();
        fs::write(root.join("a/file.txt"), b"a").unwrap();
        fs::write(root.join("a/b/file.txt"), b"b").unwrap();
        fs::write(root.join("a/b/c/file.txt"), b"c").unwrap();

        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            max_depth: Some(2),
            ..Default::default()
        };

        let subdir = root.join("a");
        let result = rs_disk_usage::scan_summary(&subdir, &opts);
        
        assert!(result.is_ok());
        let summary = result.unwrap();
        
        // Check depth constraint
        for entry in &summary.entries {
            assert!(entry.depth <= 2);
        }
    }
}
