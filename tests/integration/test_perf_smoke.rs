//! Performance smoke test harness

#[cfg(test)]
mod tests {
    use crate::fixtures::write_file_sync;
    use dua::{ScanOptions, SizeBasis, StrategyKind, TraversalDispatcher};
    use std::fs;
    use std::time::Instant;
    use tempfile::TempDir;

    const NTFS_BENCH_RUNS: usize = 5;

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

    #[cfg_attr(
        not(windows),
        ignore = "NTFS traversal benchmark uses Windows-specific APIs"
    )]
    #[test]
    fn test_ntfs_benchmark_within_slo() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Generate a moderately sized tree to approximate NTFS workloads.
        for i in 0..200 {
            let dir = root.join(format!("bench_dir_{i:03}"));
            fs::create_dir_all(&dir).unwrap();

            // 50 files per directory keeps runtime manageable while stressing metadata fetches.
            for j in 0..50 {
                let file_path = dir.join(format!("file_{j:03}.bin"));
                write_file_sync(&file_path, vec![0u8; 4 * 1024]).unwrap();
            }
        }

        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            ..Default::default()
        };

        let dispatcher = TraversalDispatcher::for_platform(&opts);
        assert_eq!(dispatcher.active_strategy(), StrategyKind::WindowsOptimized);

        let mut durations = Vec::with_capacity(NTFS_BENCH_RUNS);
        let mut last_summary = None;

        for iteration in 0..NTFS_BENCH_RUNS {
            let start = Instant::now();
            let summary = dua::scan_summary(root, &opts).expect("optimized scan should succeed");
            let duration = start.elapsed();

            log::debug!("NTFS iteration {iteration} completed in {duration:?}");
            durations.push(duration);
            last_summary = Some(summary);
        }

        durations.sort();
        let percentile_position = (NTFS_BENCH_RUNS * 95).div_ceil(100);
        let percentile_index = percentile_position
            .saturating_sub(1)
            .min(NTFS_BENCH_RUNS.saturating_sub(1));
        let p95 = durations[percentile_index];

        assert!(
            p95.as_secs_f32() < 3.0,
            "Optimized traversal p95 exceeded 3s SLO: {p95:?}"
        );

        if let Some(summary) = last_summary {
            // ensure traversal enumerated expected entry count (directories + files + root)
            let expected_entries = 200_u64 * 50 + 200 + 1;
            assert!(
                summary.entry_count >= expected_entries,
                "Expected at least {expected_entries} entries, found {}",
                summary.entry_count
            );
        } else {
            panic!("benchmark loop did not produce a summary");
        }
    }
}
