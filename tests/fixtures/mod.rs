//! Test fixtures for deterministic testing

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Write data to a file and ensure it's flushed to disk (Windows-safe)
///
/// On Windows, file metadata (including size) may not be immediately available
/// after writing. This function ensures the data is synced to disk before returning.
pub fn write_file_sync<P: AsRef<Path>, C: AsRef<[u8]>>(
    path: P,
    contents: C,
) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(contents.as_ref())?;
    file.sync_all()?;
    Ok(())
}

/// Create a simple test directory structure
#[allow(dead_code)]
pub fn create_simple_fixture(base: &Path) -> std::io::Result<PathBuf> {
    let fixture_dir = base.join("simple_test");

    // Create directory structure
    fs::create_dir_all(&fixture_dir)?;
    fs::create_dir_all(fixture_dir.join("subdir1"))?;
    fs::create_dir_all(fixture_dir.join("subdir2"))?;
    fs::create_dir_all(fixture_dir.join("subdir1/nested"))?;

    // Create some files with known sizes (using sync to ensure Windows compatibility)
    write_file_sync(fixture_dir.join("file1.txt"), b"Hello, World!")?; // 13 bytes
    write_file_sync(fixture_dir.join("subdir1/file2.txt"), [b'A'; 1024])?; // 1KB
    write_file_sync(fixture_dir.join("subdir2/file3.txt"), [b'B'; 2048])?; // 2KB
    write_file_sync(fixture_dir.join("subdir1/nested/file4.txt"), [b'C'; 512])?; // 512 bytes

    Ok(fixture_dir)
}

/// Clean up a test fixture directory
#[allow(dead_code)]
pub fn cleanup_fixture(fixture_dir: &Path) -> std::io::Result<()> {
    if fixture_dir.exists() {
        fs::remove_dir_all(fixture_dir)?;
    }
    Ok(())
}
