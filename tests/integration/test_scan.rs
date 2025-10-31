//! Integration test for scan command

use dua::{ScanOptions, SizeBasis};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

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
    fs::write(root.join("file.txt"), b"test content").unwrap();
    fs::write(root.join("dir1/file1.txt"), b"hello world").unwrap();

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
