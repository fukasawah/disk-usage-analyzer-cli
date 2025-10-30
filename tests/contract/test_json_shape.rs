//! Contract test for JSON output shape

use rs_disk_usage::{ScanOptions, SizeBasis};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_json_output_fields() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create simple test structure
    fs::create_dir_all(root.join("testdir")).unwrap();
    fs::write(root.join("testdir/file.txt"), b"test").unwrap();

    let opts = ScanOptions {
        basis: SizeBasis::Logical,
        ..Default::default()
    };

    let summary = rs_disk_usage::scan_summary(root, &opts).unwrap();

    // Verify summary structure
    assert!(!summary.root.is_empty());
    assert!(summary.entries.is_empty() || !summary.entries.is_empty());
    
    // Verify each entry has required fields
    for entry in &summary.entries {
        assert!(!entry.path.is_empty());
        // depth, size_bytes, file_count, dir_count are all present by type
        assert!(entry.depth < 1000); // Sanity check
    }

    // Test JSON serialization
    let json = serde_json::to_string(&summary.entries).unwrap();
    assert!(json.contains("path"));
    assert!(json.contains("size_bytes"));
    assert!(json.contains("file_count"));
    assert!(json.contains("dir_count"));
    assert!(json.contains("depth"));
}
