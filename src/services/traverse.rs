//! Filesystem traversal with filtering and boundary detection

use crate::models::{DirectoryEntry, ErrorItem};
use crate::{HardlinkPolicy, ScanOptions, SizeBasis};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Normalize path for cross-platform storage
/// On Windows: Convert backslashes to forward slashes for consistency
/// On Unix: Use path as-is (backslash is a valid filename character)
#[cfg(windows)]
fn normalize_path(path: &Path) -> String {
    use std::borrow::Cow;
    let path_str = path.to_string_lossy();

    // Only allocate if we actually need to replace backslashes
    if path_str.contains('\\') {
        path_str.replace('\\', "/")
    } else {
        // Avoid allocation if there are no backslashes
        match path_str {
            Cow::Borrowed(s) => s.to_string(),
            Cow::Owned(s) => s,
        }
    }
}

#[cfg(not(windows))]
fn normalize_path(path: &Path) -> String {
    use std::borrow::Cow;
    let path_str = path.to_string_lossy();

    // On Unix, avoid allocation when possible
    match path_str {
        Cow::Borrowed(s) => s.to_string(),
        Cow::Owned(s) => s,
    }
}

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(windows)]
use std::fs::OpenOptions;
#[cfg(windows)]
use std::mem::MaybeUninit;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use windows_sys::Win32::Foundation::HANDLE;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    GetFileInformationByHandle,
};

/// File identifier for hardlink tracking (device, inode)
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub struct FileId {
    dev: u64,
    ino: u64,
}

#[cfg(unix)]
#[allow(clippy::unnecessary_wraps)]
fn file_id_from_metadata(_path: &Path, metadata: &fs::Metadata) -> Option<FileId> {
    Some(FileId {
        dev: metadata.dev(),
        ino: metadata.ino(),
    })
}

#[cfg(windows)]
fn file_id_from_metadata(path: &Path, _metadata: &fs::Metadata) -> Option<FileId> {
    use std::io;

    let file = match OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .open(path)
    {
        Ok(f) => f,
        Err(err) => {
            log::warn!(
                "Failed to open handle for {} to determine file id: {err}",
                path.display()
            );
            return None;
        }
    };

    let handle = file.as_raw_handle() as HANDLE;
    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();

    let status = unsafe { GetFileInformationByHandle(handle, info.as_mut_ptr()) };
    if status == 0 {
        let err = io::Error::last_os_error();
        log::warn!(
            "GetFileInformationByHandle failed for {}: {err}",
            path.display()
        );
        return None;
    }

    let info = unsafe { info.assume_init() };
    let ino = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
    let dev = info.dwVolumeSerialNumber as u64;

    Some(FileId { dev, ino })
}

#[cfg(not(any(unix, windows)))]
fn file_id_from_metadata(_path: &Path, _metadata: &fs::Metadata) -> Option<FileId> {
    None
}

/// Get filesystem device ID for boundary detection
#[cfg(unix)]
fn get_device_id(metadata: &fs::Metadata) -> u64 {
    metadata.dev()
}

#[cfg(not(unix))]
fn get_device_id(_metadata: &fs::Metadata) -> u64 {
    // On Windows and other platforms, we'd need platform-specific code
    // For now, return a constant to effectively disable boundary checking
    0
}

/// Traversal context to track state during directory walk
pub struct TraversalContext {
    pub root_device: Option<u64>,
    pub seen_inodes: HashSet<FileId>,
    pub entries: HashMap<PathBuf, DirectoryEntry>,
    pub errors: Vec<ErrorItem>,
    pub options: ScanOptions,
    pub max_depth: Option<u16>,
}

impl TraversalContext {
    #[must_use]
    pub fn new(options: ScanOptions, max_depth: Option<u16>) -> Self {
        Self {
            root_device: None,
            seen_inodes: HashSet::new(),
            entries: HashMap::new(),
            errors: Vec::new(),
            options,
            max_depth,
        }
    }

    /// Check if we should count this file (based on hardlink policy)
    fn should_count_file(&mut self, path: &Path, metadata: &fs::Metadata) -> bool {
        match self.options.hardlink_policy {
            HardlinkPolicy::Count => true,
            HardlinkPolicy::Dedupe => {
                if let Some(file_id) = file_id_from_metadata(path, metadata) {
                    // Only count if we haven't seen this inode before
                    self.seen_inodes.insert(file_id)
                } else {
                    // If we cannot determine a stable file id, fall back to counting
                    true
                }
            }
        }
    }

    /// Get size based on the configured basis
    #[allow(unused_variables)]
    fn get_size(&self, path: &Path, metadata: &fs::Metadata) -> u64 {
        use crate::services::size;

        match self.options.basis {
            SizeBasis::Logical => {
                let s = size::logical_size(metadata);
                log::trace!("Logical size for {}: {s}", path.display());
                s
            }
            SizeBasis::Physical => {
                #[cfg(unix)]
                {
                    let s = size::physical_size_from_metadata(metadata);
                    log::trace!("Physical size (Unix) for {}: {s}", path.display());
                    s
                }
                #[cfg(windows)]
                {
                    let s = size::physical_size_from_path(path).unwrap_or_else(|e| {
                        log::warn!(
                            "Failed to get physical size for {}: {e}, using logical size",
                            path.display()
                        );
                        metadata.len()
                    });
                    log::trace!("Physical size (Windows) for {}: {s}", path.display());
                    s
                }
                #[cfg(not(any(unix, windows)))]
                {
                    let s = size::physical_size_from_metadata(metadata);
                    log::trace!("Physical size (other) for {}: {s}", path.display());
                    s
                }
            }
        }
    }

    /// Record an error encountered during traversal
    fn record_error(&mut self, path: &Path, error: &std::io::Error) {
        let code = match error.kind() {
            std::io::ErrorKind::NotFound => "ENOENT",
            std::io::ErrorKind::PermissionDenied => "EACCES",
            _ => "IO",
        };

        self.errors.push(ErrorItem {
            path: path.to_string_lossy().to_string(),
            code: code.to_string(),
            message: error.to_string(),
        });
    }
}

/// Traverse a directory tree and collect entries
///
/// # Arguments
/// * `root` - The root path to start traversal
/// * `context` - Traversal context with options and state
///
/// # Returns
/// The total size of the root directory
pub fn traverse_directory<P: AsRef<Path>>(
    root: P,
    context: &mut TraversalContext,
) -> std::io::Result<u64> {
    let root = root.as_ref();

    // Get root metadata
    let root_metadata = match fs::symlink_metadata(root) {
        Ok(m) => m,
        Err(e) => {
            context.record_error(root, &e);
            return Ok(0);
        }
    };

    // Initialize root device for boundary detection
    if context.root_device.is_none() && !context.options.cross_filesystem {
        context.root_device = Some(get_device_id(&root_metadata));
    }

    // Start recursive traversal
    traverse_recursive(root, 0, context)
}

/// Recursive traversal implementation
#[allow(clippy::too_many_lines)]
fn traverse_recursive(
    current: &Path,
    depth: u16,
    context: &mut TraversalContext,
) -> std::io::Result<u64> {
    // Check depth limit
    if let Some(max_depth) = context.max_depth
        && depth > max_depth
    {
        return Ok(0);
    }

    // Get metadata (without following symlinks)
    let metadata = match fs::symlink_metadata(current) {
        Ok(m) => m,
        Err(e) => {
            context.record_error(current, &e);
            return Ok(0);
        }
    };

    // Check if symlink (and whether to follow)
    if metadata.is_symlink() && !context.options.follow_symlinks {
        return Ok(0);
    }

    // Check filesystem boundary
    if !context.options.cross_filesystem
        && let Some(root_dev) = context.root_device
    {
        let current_dev = get_device_id(&metadata);
        if current_dev != root_dev {
            return Ok(0);
        }
    }

    if metadata.is_file() {
        // Process file
        let size = if context.should_count_file(current, &metadata) {
            context.get_size(current, &metadata)
        } else {
            0
        };
        Ok(size)
    } else if metadata.is_dir() {
        // Process directory
        let mut total_size = 0u64;
        let mut file_count = 0u32;
        let mut dir_count = 0u32;

        // Read directory entries
        let entries = match fs::read_dir(current) {
            Ok(e) => e,
            Err(e) => {
                context.record_error(current, &e);
                return Ok(0);
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    context.record_error(current, &e);
                    continue;
                }
            };

            let entry_path = entry.path();
            let entry_metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    context.record_error(&entry_path, &e);
                    continue;
                }
            };

            if entry_metadata.is_file() {
                let file_size = if context.should_count_file(&entry_path, &entry_metadata) {
                    context.get_size(&entry_path, &entry_metadata)
                } else {
                    0
                };
                total_size += file_size;
                file_count += 1;

                // Record file as an entry too (if within depth limit)
                let file_depth = depth + 1;
                let within_depth_limit = context.max_depth.is_none_or(|max| file_depth <= max);

                if within_depth_limit {
                    let parent_path_str = normalize_path(current);
                    let file_entry = DirectoryEntry {
                        path: normalize_path(&entry_path),
                        parent_path: Some(parent_path_str),
                        depth: file_depth,
                        size_bytes: file_size,
                        file_count: 0,
                        dir_count: 0,
                    };
                    log::debug!("File entry: {} (size: {})", file_entry.path, file_size);
                    context.entries.insert(entry_path, file_entry);
                }
            } else if entry_metadata.is_dir() {
                let subdir_size = traverse_recursive(&entry_path, depth + 1, context)?;
                total_size += subdir_size;
                dir_count += 1;
            }
        }

        // Record this directory entry
        let parent_path = current.parent().map(normalize_path);
        let normalized_path = normalize_path(current);

        let entry = DirectoryEntry {
            path: normalized_path.clone(),
            parent_path,
            depth,
            size_bytes: total_size,
            file_count,
            dir_count,
        };

        log::debug!(
            "Directory entry: {normalized_path} (size: {total_size}, files: {file_count}, dirs: {dir_count}, depth: {depth})"
        );

        context.entries.insert(current.to_path_buf(), entry);

        Ok(total_size)
    } else {
        // Other file types (symlinks, devices, etc.)
        Ok(0)
    }
}
