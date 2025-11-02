//! Windows-optimized traversal strategy leveraging large directory fetch APIs.
//!
//! Implements a breadth-first traversal using Win32 directory enumeration while
//! offloading subdirectory processing to Rayon for improved throughput. The
//! optimized path preserves legacy invariants around error collection,
//! hardlink deduplication, and progress reporting.

use super::legacy;
use super::strategy::TraversalStrategy;
use super::{StrategyKind, TraversalContext};
use crate::ScanOptions;
use std::io;
use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;

#[cfg(windows)]
use crate::models::DirectoryEntry;
#[cfg(windows)]
use rayon::prelude::*;
#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::fs;
#[cfg(windows)]
use std::mem::MaybeUninit;
#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt, OsStringExt};
#[cfg(windows)]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(windows)]
use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE,
};
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    FIND_FIRST_EX_LARGE_FETCH, FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW,
    FindNextFileW, WIN32_FIND_DATAW,
};
#[cfg(windows)]
use windows::core::PCWSTR;

/// Windows traversal backend.
#[derive(Debug, Default)]
pub struct WindowsTraversal;

impl WindowsTraversal {
    /// Determine whether the optimized Windows strategy can run on this build.
    #[must_use]
    pub fn is_supported() -> bool {
        cfg!(windows)
    }
}

impl TraversalStrategy for WindowsTraversal {
    fn kind(&self) -> StrategyKind {
        StrategyKind::WindowsOptimized
    }

    fn is_eligible(&self, _opts: &ScanOptions) -> bool {
        Self::is_supported()
    }

    fn traverse(&self, root: &Path, context: &mut TraversalContext) -> io::Result<u64> {
        #[cfg(windows)]
        {
            traverse_windows(root, context)
        }

        #[cfg(not(windows))]
        {
            log::debug!(
                "windows::WindowsTraversal invoked on non-Windows platform; falling back to legacy"
            );
            legacy::traverse_directory(root, context)
        }
    }
}

#[cfg(windows)]
fn traverse_windows(root: &Path, context: &TraversalContext) -> io::Result<u64> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(meta) => meta,
        Err(err) => {
            context.record_error(root, &err);
            return Ok(0);
        }
    };

    if !context.options.cross_filesystem {
        context.set_root_device_if_absent(legacy::get_device_id(&metadata));
    }

    traverse_directory(root, 0, context)
}

#[cfg(windows)]
#[allow(clippy::too_many_lines)]
fn traverse_directory(current: &Path, depth: u16, context: &TraversalContext) -> io::Result<u64> {
    if let Some(max_depth) = context.max_depth
        && depth > max_depth
    {
        return Ok(0);
    }

    let metadata = match fs::symlink_metadata(current) {
        Ok(meta) => meta,
        Err(err) => {
            context.record_error(current, &err);
            return Ok(0);
        }
    };

    if metadata.is_symlink() && !context.options.follow_symlinks {
        return Ok(0);
    }

    if !context.options.cross_filesystem {
        if let Some(root_dev) = context.root_device() {
            let current_dev = legacy::get_device_id(&metadata);
            if current_dev != root_dev {
                return Ok(0);
            }
        }
    }

    if metadata.is_file() {
        let size = if context.should_count_file(current, &metadata) {
            context.get_size(current, &metadata)
        } else {
            0
        };
        return Ok(size);
    }

    if !metadata.is_dir() {
        return Ok(0);
    }

    let mut total_size = 0u64;
    let mut file_count = 0u32;
    let mut dir_count = 0u32;
    let mut child_dirs: Vec<PathBuf> = Vec::new();

    let search_spec = current.join("*");
    let search_wide = to_wide_null(&search_spec);
    let mut find_data = MaybeUninit::<WIN32_FIND_DATAW>::uninit();

    let handle = match unsafe {
        FindFirstFileExW(
            PCWSTR(search_wide.as_ptr()),
            FindExInfoBasic,
            find_data.as_mut_ptr() as *mut _,
            FindExSearchNameMatch,
            None,
            FIND_FIRST_EX_LARGE_FETCH,
        )
    } {
        Ok(handle) => SearchHandle::new(handle),
        Err(err) => {
            let io_err: io::Error = err.into();
            if !matches!(io_err.raw_os_error(), Some(code) if code == ERROR_FILE_NOT_FOUND.0 as i32)
            {
                context.record_error(current, &io_err);
            }
            return Ok(0);
        }
    };

    {
        let mut data = unsafe { find_data.assume_init() };

        loop {
            handle_entry(
                &data,
                current,
                depth,
                context,
                &mut total_size,
                &mut file_count,
                &mut dir_count,
                &mut child_dirs,
            )?;

            let mut next = MaybeUninit::<WIN32_FIND_DATAW>::uninit();
            match unsafe { FindNextFileW(handle.raw(), next.as_mut_ptr()) } {
                Ok(()) => {
                    data = unsafe { next.assume_init() };
                }
                Err(err) => {
                    let io_err: io::Error = err.into();
                    if !matches!(
                        io_err.raw_os_error(),
                        Some(code) if code == ERROR_NO_MORE_FILES.0 as i32
                    ) {
                        context.record_error(current, &io_err);
                    }
                    break;
                }
            }
        }
    }

    let subdir_total = AtomicU64::new(0);
    child_dirs.into_par_iter().try_for_each(|child_path| {
        let size = traverse_directory(&child_path, depth + 1, context)?;
        subdir_total.fetch_add(size, Ordering::Relaxed);
        Ok::<(), io::Error>(())
    })?;

    total_size = total_size.saturating_add(subdir_total.load(Ordering::Relaxed));

    let parent_path = current.parent().map(legacy::normalize_path);
    let normalized_path = legacy::normalize_path(current);

    let entry = DirectoryEntry {
        path: normalized_path.clone(),
        parent_path,
        depth,
        size_bytes: total_size,
        file_count,
        dir_count,
    };

    context.insert_entry(current.to_path_buf(), entry);
    context.register_directory_progress();

    Ok(total_size)
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn handle_entry(
    data: &WIN32_FIND_DATAW,
    parent: &Path,
    depth: u16,
    context: &TraversalContext,
    total_size: &mut u64,
    file_count: &mut u32,
    dir_count: &mut u32,
    child_dirs: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let name = filename_from_data(data);
    if name == "." || name == ".." {
        return Ok(());
    }

    let child_path = parent.join(&name);
    let entry_metadata = match fs::symlink_metadata(&child_path) {
        Ok(meta) => meta,
        Err(err) => {
            context.record_error(&child_path, &err);
            return Ok(());
        }
    };

    if entry_metadata.is_symlink() && !context.options.follow_symlinks {
        return Ok(());
    }

    if !context.options.cross_filesystem {
        if let Some(root_dev) = context.root_device() {
            let current_dev = legacy::get_device_id(&entry_metadata);
            if current_dev != root_dev {
                return Ok(());
            }
        }
    }

    if entry_metadata.is_file() {
        let file_size = if context.should_count_file(&child_path, &entry_metadata) {
            context.get_size(&child_path, &entry_metadata)
        } else {
            0
        };

        *total_size = total_size.saturating_add(file_size);
        *file_count = file_count.saturating_add(1);
        context.register_file_progress(file_size);

        let file_depth = depth + 1;
        if context.max_depth.is_none_or(|max| file_depth <= max) {
            let parent_path_str = legacy::normalize_path(parent);
            let entry = DirectoryEntry {
                path: legacy::normalize_path(&child_path),
                parent_path: Some(parent_path_str),
                depth: file_depth,
                size_bytes: file_size,
                file_count: 0,
                dir_count: 0,
            };
            context.insert_entry(child_path, entry);
        }
    } else if entry_metadata.is_dir() {
        *dir_count = dir_count.saturating_add(1);
        let next_depth = depth + 1;

        if context.max_depth.is_some_and(|max| next_depth > max) {
            return Ok(());
        }

        child_dirs.push(child_path);
    }

    Ok(())
}

#[cfg(windows)]
fn to_wide_null(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if !wide.ends_with(&[0]) {
        wide.push(0);
    }
    wide
}

#[cfg(windows)]
fn filename_from_data(data: &WIN32_FIND_DATAW) -> OsString {
    let buffer = &data.cFileName;
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    OsString::from_wide(&buffer[..len])
}

#[cfg(windows)]
struct SearchHandle(HANDLE);

#[cfg(windows)]
impl SearchHandle {
    fn new(raw: HANDLE) -> Self {
        Self(raw)
    }

    fn raw(&self) -> HANDLE {
        self.0
    }
}

#[cfg(windows)]
impl Drop for SearchHandle {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE {
            if let Err(err) = unsafe { FindClose(self.0) } {
                let io_err: io::Error = err.into();
                log::warn!("FindClose failed for traversal handle: {io_err}");
            }
            self.0 = INVALID_HANDLE_VALUE;
        }
    }
}
