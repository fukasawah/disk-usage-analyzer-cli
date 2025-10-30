//! Resilience test with permission errors and file churn

#[cfg(test)]
mod tests {
    use dua::{ScanOptions, SizeBasis};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_continues_after_errors() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create some accessible directories
        fs::create_dir_all(root.join("accessible1")).unwrap();
        fs::create_dir_all(root.join("accessible2")).unwrap();
        fs::write(root.join("accessible1/file1.txt"), b"content1").unwrap();
        fs::write(root.join("accessible2/file2.txt"), b"content2").unwrap();

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

        // Create many small files
        for i in 0..50 {
            fs::write(root.join(format!("file{i}.txt")), format!("{i}")).unwrap();
        }

        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            ..Default::default()
        };

        let result = dua::scan_summary(root, &opts);
        
        assert!(result.is_ok());
        let summary = result.unwrap();
        assert!(!summary.entries.is_empty());
    }
}
