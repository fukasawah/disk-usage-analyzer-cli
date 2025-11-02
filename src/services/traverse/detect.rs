//! Filesystem detection helpers for strategy selection.

use super::StrategyKind;
use std::path::Path;

/// Normalized filesystem classifications used for strategy selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemKind {
    Ntfs,
    Apfs,
    Ext,
    Other,
}

/// Determine the filesystem kind for the provided path.
#[must_use]
pub fn filesystem_kind_for_path(path: &Path) -> FilesystemKind {
    #[cfg(windows)]
    {
        detect_windows_filesystem(path)
    }

    #[cfg(target_os = "macos")]
    {
        detect_macos_filesystem(path)
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        detect_unix_filesystem(path)
    }

    #[cfg(not(any(windows, target_os = "macos", unix)))]
    {
        let _ = path;
        FilesystemKind::Other
    }
}

/// Return the default traversal strategy for the current host.
#[must_use]
pub fn default_strategy() -> StrategyKind {
    let cwd = std::env::current_dir().ok();
    let fs_kind = cwd
        .as_deref()
        .map_or(FilesystemKind::Other, filesystem_kind_for_path);
    strategy_for_filesystem(fs_kind)
}

/// Map a filesystem kind to the preferred traversal strategy.
#[must_use]
pub fn strategy_for_filesystem(kind: FilesystemKind) -> StrategyKind {
    match kind {
        FilesystemKind::Ntfs => StrategyKind::WindowsOptimized,
        FilesystemKind::Apfs | FilesystemKind::Ext => StrategyKind::PosixOptimized,
        FilesystemKind::Other => StrategyKind::Legacy,
    }
}

#[cfg(windows)]
fn detect_windows_filesystem(_path: &Path) -> FilesystemKind {
    // Placeholder implementation until full Win32 detection is added.
    FilesystemKind::Ntfs
}

#[cfg(target_os = "macos")]
fn detect_macos_filesystem(path: &Path) -> FilesystemKind {
    use std::ffi::CStr;
    use std::os::unix::ffi::OsStrExt;

    let Ok(stat) = rustix::fs::statfs(path) else {
        return FilesystemKind::Other;
    };

    let fstype = unsafe { CStr::from_ptr(stat.f_fstypename.as_ptr()) }
        .to_string_lossy()
        .to_ascii_uppercase();

    if fstype.contains("APFS") {
        FilesystemKind::Apfs
    } else {
        FilesystemKind::Other
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn detect_unix_filesystem(_path: &Path) -> FilesystemKind {
    #[cfg(target_os = "linux")]
    {
        FilesystemKind::Ext
    }

    #[cfg(not(target_os = "linux"))]
    {
        FilesystemKind::Other
    }
}
