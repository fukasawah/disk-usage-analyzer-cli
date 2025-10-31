//! Performance smoke test harness

#[cfg(test)]
mod tests {
    use crate::fixtures::write_file_sync;
    use dua::{ScanOptions, SizeBasis};
    use std::fs;
    use std::time::Instant;
    use tempfile::TempDir;

    #[test]
    fn test_performance_smoke() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a moderate-sized directory structure
        // 100 directories with 10 files each = 1000 files total
        for i in 0..100 {
            let dir = root.join(format!("dir{i:03}"));
            fs::create_dir_all(&dir).unwrap();

            for j in 0..10 {
                let file_path = dir.join(format!("file{j}.txt"));
                write_file_sync(file_path, format!("Content {i}-{j}")).unwrap();
            }
        }

        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            ..Default::default()
        };

        let start = Instant::now();
        let result = dua::scan_summary(root, &opts);
        let duration = start.elapsed();

        assert!(result.is_ok(), "Scan failed: {:?}", result.err());

        let summary = result.unwrap();
        assert!(!summary.entries.is_empty());

        // Performance requirement: should complete in reasonable time
        // For 1000 files, expect < 5 seconds (very conservative)
        assert!(duration.as_secs() < 5, "Scan took too long: {duration:?}");

        println!(
            "Performance: scanned {} entries in {:?}",
            summary.entries.len(),
            duration
        );
    }

    #[test]
    fn test_deep_nesting() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create deep nesting (20 levels)
        let mut current = root.to_path_buf();
        for i in 0..20 {
            current = current.join(format!("level{i}"));
            fs::create_dir_all(&current).unwrap();
            write_file_sync(current.join("file.txt"), format!("Level {i}")).unwrap();
        }

        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            max_depth: None, // No limit
            ..Default::default()
        };

        let start = Instant::now();
        let result = dua::scan_summary(root, &opts);
        let duration = start.elapsed();

        assert!(result.is_ok());
        let summary = result.unwrap();

        // Should handle deep nesting
        let max_depth = summary.entries.iter().map(|e| e.depth).max().unwrap_or(0);
        assert!(max_depth >= 10, "Expected deeper traversal");

        println!("Deep nesting: max depth {max_depth} in {duration:?}");
    }
}
