//! Unit tests for depth limiting behavior

#[cfg(test)]
mod tests {
    use dua::{HardlinkPolicy, ScanOptions, SizeBasis};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_depth_limiting() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create nested structure
        fs::create_dir_all(root.join("level1/level2/level3")).unwrap();
        fs::write(root.join("level1/file1.txt"), b"test").unwrap();
        fs::write(root.join("level1/level2/file2.txt"), b"test").unwrap();
        fs::write(root.join("level1/level2/level3/file3.txt"), b"test").unwrap();

        // Scan with depth limit of 1
        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            max_depth: Some(1),
            hardlink_policy: HardlinkPolicy::Dedupe,
            follow_symlinks: false,
            cross_filesystem: false,
        };

        let result = dua::scan_summary(root, &opts);
        assert!(result.is_ok());

        let summary = result.unwrap();

        // Should only contain entries up to depth 1
        for entry in &summary.entries {
            assert!(
                entry.depth <= 1,
                "Entry depth {} exceeds limit",
                entry.depth
            );
        }
    }

    #[test]
    fn test_no_depth_limit() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create nested structure
        fs::create_dir_all(root.join("a/b/c/d")).unwrap();
        fs::write(root.join("a/b/c/d/file.txt"), b"deep").unwrap();

        // Scan without depth limit
        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            max_depth: None,
            hardlink_policy: HardlinkPolicy::Dedupe,
            follow_symlinks: false,
            cross_filesystem: false,
        };

        let result = dua::scan_summary(root, &opts);
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert!(!summary.entries.is_empty());

        // Should have entries at various depths
        let max_depth = summary.entries.iter().map(|e| e.depth).max().unwrap_or(0);
        assert!(max_depth > 1, "Expected deeper traversal");
    }
}
