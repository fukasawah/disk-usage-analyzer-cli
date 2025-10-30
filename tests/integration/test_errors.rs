//! Integration test for error handling

use dua::ScanOptions;

#[test]
fn test_invalid_path_error() {
    let opts = ScanOptions::default();
    let result = dua::scan_summary("/definitely/does/not/exist/xyz123", &opts);
    
    assert!(result.is_err());
    
    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(error_msg.contains("does not exist") || error_msg.contains("Invalid input"));
    }
}

#[test]
fn test_file_instead_of_directory() {
    use tempfile::NamedTempFile;
    
    let temp_file = NamedTempFile::new().unwrap();
    let opts = ScanOptions::default();
    let result = dua::scan_summary(temp_file.path(), &opts);
    
    assert!(result.is_err());
    
    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(error_msg.contains("not a directory") || error_msg.contains("Invalid input"));
    }
}
