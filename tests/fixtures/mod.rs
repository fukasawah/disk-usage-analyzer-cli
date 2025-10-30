//! Test fixtures for deterministic testing

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Create a simple test directory structure
pub fn create_simple_fixture(base: &Path) -> std::io::Result<PathBuf> {
    let fixture_dir = base.join("simple_test");
    
    // Create directory structure
    fs::create_dir_all(&fixture_dir)?;
    fs::create_dir_all(fixture_dir.join("subdir1"))?;
    fs::create_dir_all(fixture_dir.join("subdir2"))?;
    fs::create_dir_all(fixture_dir.join("subdir1/nested"))?;
    
    // Create some files with known sizes
    let mut file1 = fs::File::create(fixture_dir.join("file1.txt"))?;
    file1.write_all(b"Hello, World!")?; // 13 bytes
    
    let mut file2 = fs::File::create(fixture_dir.join("subdir1/file2.txt"))?;
    file2.write_all(&[b'A'; 1024])?; // 1KB
    
    let mut file3 = fs::File::create(fixture_dir.join("subdir2/file3.txt"))?;
    file3.write_all(&[b'B'; 2048])?; // 2KB
    
    let mut file4 = fs::File::create(fixture_dir.join("subdir1/nested/file4.txt"))?;
    file4.write_all(&[b'C'; 512])?; // 512 bytes
    
    Ok(fixture_dir)
}

/// Clean up a test fixture directory
pub fn cleanup_fixture(fixture_dir: &Path) -> std::io::Result<()> {
    if fixture_dir.exists() {
        fs::remove_dir_all(fixture_dir)?;
    }
    Ok(())
}
