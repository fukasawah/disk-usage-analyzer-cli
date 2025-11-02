//! Integration test for scan command

use crate::fixtures::write_file_sync;
use dua::services::traverse::{
    StrategyKind, detect, posix::PosixTraversal, windows::WindowsTraversal,
};
use dua::{ScanOptions, SizeBasis};
use std::collections::HashMap;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn root_total(summary: &dua::Summary) -> u64 {
    summary
        .entries
        .iter()
        .find(|entry| entry.parent_path.is_none())
        .map_or(0, |entry| entry.size_bytes)
}

#[test]
fn test_scan_command_help() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "dua", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Disk Usage CLI"));
    assert!(stdout.contains("scan"));
}

#[test]
fn test_scan_via_api() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create test structure
    fs::create_dir_all(root.join("dir1")).unwrap();
    fs::create_dir_all(root.join("dir2")).unwrap();
    write_file_sync(root.join("file.txt"), b"test content").unwrap();
    write_file_sync(root.join("dir1/file1.txt"), b"hello world").unwrap();

    let opts = ScanOptions {
        basis: SizeBasis::Logical,
        ..Default::default()
    };

    let result = dua::scan_summary(root, &opts);
    assert!(result.is_ok());

    let summary = result.unwrap();
    assert!(!summary.entries.is_empty());
    assert_eq!(summary.root, root.to_string_lossy());

    // Verify paths are normalized (no backslashes on any platform)
    for entry in &summary.entries {
        let path = &entry.path;
        assert!(
            !entry.path.contains('\\'),
            "Path should not contain backslashes: {path}"
        );
        if let Some(parent) = &entry.parent_path {
            assert!(
                !parent.contains('\\'),
                "Parent path should not contain backslashes: {parent}"
            );
        }
    }

    // Verify expected files exist with correct sizes
    let test_file_entry = summary
        .entries
        .iter()
        .find(|e| e.path.ends_with("file.txt"))
        .expect("Should find file.txt");
    assert_eq!(
        test_file_entry.size_bytes, 12,
        "file.txt should be 12 bytes ('test content')"
    );

    let nested_file_entry = summary
        .entries
        .iter()
        .find(|e| e.path.ends_with("file1.txt"))
        .expect("Should find file1.txt");
    assert_eq!(
        nested_file_entry.size_bytes, 11,
        "file1.txt should be 11 bytes ('hello world')"
    );
}

#[test]
fn test_optimized_vs_legacy_parity() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Prepare deterministic workload with mixed file sizes.
    fs::create_dir_all(root.join("nested")).unwrap();
    write_file_sync(root.join("a.bin"), vec![1u8; 8 * 1024]).unwrap();
    write_file_sync(root.join("b.bin"), vec![2u8; 12 * 1024]).unwrap();
    write_file_sync(root.join("nested/c.bin"), vec![3u8; 4 * 1024]).unwrap();

    let optimized_opts = ScanOptions {
        basis: SizeBasis::Logical,
        ..Default::default()
    };

    let mut legacy_opts = optimized_opts.clone();
    legacy_opts.strategy_override = Some(StrategyKind::Legacy);

    let optimized = dua::scan_summary(root, &optimized_opts).expect("optimized scan");
    let legacy = dua::scan_summary(root, &legacy_opts).expect("legacy scan");

    // Confirm strategy auto-selection aligns with filesystem detection when supported.
    let detected_fs = detect::filesystem_kind_for_path(root);
    let preferred_strategy = detect::strategy_for_filesystem(detected_fs);
    let supported_strategy = match preferred_strategy {
        StrategyKind::WindowsOptimized if !WindowsTraversal::is_supported() => StrategyKind::Legacy,
        StrategyKind::PosixOptimized if !PosixTraversal::is_supported() => StrategyKind::Legacy,
        other => other,
    };
    assert_eq!(optimized.strategy, supported_strategy);

    let optimized_bytes = root_total(&optimized);
    let legacy_bytes = root_total(&legacy);
    let delta = optimized_bytes.abs_diff(legacy_bytes);
    let tolerance = std::cmp::max(legacy_bytes / 100, 10 * 1024 * 1024);

    assert!(
        delta <= tolerance,
        "optimized ({optimized_bytes}) vs legacy ({legacy_bytes}) delta {delta} exceeds tolerance {tolerance}"
    );

    let optimized_map: HashMap<_, _> = optimized
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    let legacy_map: HashMap<_, _> = legacy
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();

    assert_eq!(
        optimized_map.len(),
        legacy_map.len(),
        "entry count mismatch"
    );

    for (path, legacy_entry) in &legacy_map {
        let optimized_entry = optimized_map
            .get(path)
            .unwrap_or_else(|| panic!("Missing path {path} in optimized traversal"));

        let entry_delta = optimized_entry.size_bytes.abs_diff(legacy_entry.size_bytes);
        assert!(
            entry_delta <= tolerance,
            "Entry {path} delta {entry_delta} exceeds tolerance {tolerance}"
        );
        assert_eq!(
            optimized_entry.file_count, legacy_entry.file_count,
            "file count mismatch for {path}"
        );
        assert_eq!(
            optimized_entry.dir_count, legacy_entry.dir_count,
            "dir count mismatch for {path}"
        );
    }

    assert!(
        optimized.errors.is_empty(),
        "optimized traversal recorded unexpected errors"
    );
    assert!(
        legacy.errors.is_empty(),
        "legacy traversal recorded unexpected errors"
    );
}
