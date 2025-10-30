//! Size computation (logical and physical) with platform-specific implementations

use std::fs::Metadata;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(windows)]
use std::path::Path;

/// Compute logical size from metadata
#[must_use]
pub fn logical_size(metadata: &Metadata) -> u64 {
    metadata.len()
}

/// Compute physical size from metadata (Unix platform)
/// Uses the number of 512-byte blocks allocated to the file
#[cfg(unix)]
#[must_use]
pub fn physical_size_from_metadata(metadata: &Metadata) -> u64 {
    // On Unix, use block count * 512 (standard block size)
    // This accounts for filesystem block allocation
    metadata.blocks() * 512
}

/// Compute physical size from metadata (Windows platform)
/// Uses GetCompressedFileSizeW to get actual disk usage
#[cfg(windows)]
pub fn physical_size_from_path(path: &Path) -> std::io::Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetCompressedFileSizeW;

    const INVALID_FILE_SIZE: u32 = 0xFFFFFFFF;

    // Convert path to wide string (UTF-16)
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut high: u32 = 0;
    let low = unsafe { GetCompressedFileSizeW(wide.as_ptr(), &mut high) };

    if low == INVALID_FILE_SIZE {
        // Fall back to logical size on error
        let metadata = std::fs::metadata(path)?;
        Ok(metadata.len())
    } else {
        // Combine high and low parts into 64-bit size
        Ok(u64::from(high) << 32 | u64::from(low))
    }
}

/// Compute physical size (non-Unix, non-Windows fallback)
#[cfg(not(any(unix, windows)))]
#[must_use]
pub fn physical_size_from_metadata(metadata: &Metadata) -> u64 {
    // Fallback to logical size on unsupported platforms
    logical_size(metadata)
}
