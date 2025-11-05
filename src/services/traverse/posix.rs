//! POSIX-optimized traversal strategy leveraging `rustix` primitives.
//!
//! The optimized implementation will land in later phases; this placeholder
//! routes through the legacy traversal engine while we wire coordination and
//! eligibility checks.

use super::legacy;
use super::strategy::TraversalStrategy;
use super::{StrategyKind, TraversalContext};
use crate::ScanOptions;
use std::io;
use std::path::Path;

#[cfg(unix)]
use crate::models::DirectoryEntry;
#[cfg(unix)]
use std::path::PathBuf;

#[cfg(unix)]
use rayon::prelude::*;
#[cfg(unix)]
use rustix::fd::OwnedFd;
#[cfg(unix)]
use rustix::fs::{self as rfs, Dir, Mode, OFlags};
#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};

/// POSIX traversal backend placeholder.
#[derive(Debug, Default)]
pub struct PosixTraversal;

impl PosixTraversal {
    #[must_use]
    pub fn is_supported() -> bool {
        cfg!(unix)
    }
}

impl TraversalStrategy for PosixTraversal {
    fn kind(&self) -> StrategyKind {
        StrategyKind::PosixOptimized
    }

    fn is_eligible(&self, _opts: &ScanOptions) -> bool {
        Self::is_supported()
    }

    fn traverse(&self, root: &Path, context: &mut TraversalContext) -> io::Result<u64> {
        #[cfg(unix)]
        {
            posix_traverse(root, context)
        }

        #[cfg(not(unix))]
        {
            log::debug!(
                "posix::PosixTraversal invoked on non-Unix platform; falling back to legacy"
            );
            legacy::traverse_directory(root, context)
        }
    }
}

#[cfg(unix)]
fn posix_traverse(root: &Path, context: &TraversalContext) -> io::Result<u64> {
    let root_metadata = match std::fs::symlink_metadata(root) {
        Ok(meta) => meta,
        Err(err) => {
            context.record_error(root, &err)?;
            return Ok(0);
        }
    };

    if !root_metadata.is_dir() {
        return legacy::traverse_directory(root, context);
    }

    if !context.options.cross_filesystem {
        context.set_root_device_if_absent(legacy::get_device_id(&root_metadata));
    }

    let dir_fd = rfs::openat(
        rfs::CWD,
        root,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(std::io::Error::from)?;

    traverse_directory_fd(root, dir_fd, 0, context)
}

#[cfg(unix)]
#[allow(clippy::too_many_lines)]
fn traverse_directory_fd(
    current: &Path,
    dir_fd: OwnedFd,
    depth: u16,
    context: &TraversalContext,
) -> io::Result<u64> {
    if let Some(max_depth) = context.max_depth
        && depth > max_depth
    {
        return Ok(0);
    }

    let mut total_size = 0u64;
    let mut file_count = 0u32;
    let mut dir_count = 0u32;
    let mut child_dirs: Vec<(PathBuf, OwnedFd)> = Vec::new();

    let dir_iter = Dir::read_from(&dir_fd).map_err(std::io::Error::from)?;

    for entry_result in dir_iter {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(err) => {
                let io_err: std::io::Error = err.into();
                context.record_error(current, &io_err)?;
                continue;
            }
        };

        let name_bytes = entry.file_name().to_bytes();
        if name_bytes == b"." || name_bytes == b".." {
            continue;
        }

        let child_name = OsString::from_vec(name_bytes.to_vec());
        let child_component = PathBuf::from(&child_name);
        let child_path: PathBuf = current.join(&child_component);

        let metadata = match std::fs::symlink_metadata(&child_path) {
            Ok(meta) => meta,
            Err(err) => {
                context.record_error(&child_path, &err)?;
                continue;
            }
        };

        if metadata.is_symlink() && !context.options.follow_symlinks {
            continue;
        }

        if !context.options.cross_filesystem
            && let Some(root_dev) = context.root_device()
        {
            let current_dev = legacy::get_device_id(&metadata);
            if current_dev != root_dev {
                continue;
            }
        }

        if metadata.is_file() {
            let file_size = if context.should_count_file(&child_path, &metadata) {
                context.get_size(&child_path, &metadata)
            } else {
                0
            };

            total_size = total_size.saturating_add(file_size);
            file_count = file_count.saturating_add(1);
            context.register_file_progress(file_size);

            let file_depth = depth + 1;
            if context.max_depth.is_none_or(|max| file_depth <= max) {
                let parent_path_str = legacy::normalize_path(current);
                let file_entry = DirectoryEntry {
                    path: legacy::normalize_path(&child_path),
                    parent_path: Some(parent_path_str),
                    depth: file_depth,
                    size_bytes: file_size,
                    file_count: 0,
                    dir_count: 0,
                };
                context.insert_entry(file_entry)?;
            }
        } else if metadata.is_dir() {
            dir_count = dir_count.saturating_add(1);
            let next_depth = depth + 1;

            if context.max_depth.is_some_and(|max| next_depth > max) {
                continue;
            }

            let child_fd = match rfs::openat(
                &dir_fd,
                child_component.as_path(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
            ) {
                Ok(fd) => fd,
                Err(err) => {
                    let io_err: std::io::Error = err.into();
                    context.record_error(&child_path, &io_err)?;
                    continue;
                }
            };

            child_dirs.push((child_path.clone(), child_fd));
        }
    }

    drop(dir_fd);

    let subdir_total = AtomicU64::new(0);
    child_dirs
        .into_par_iter()
        .try_for_each(|(child_path, child_fd)| {
            let size = traverse_directory_fd(&child_path, child_fd, depth + 1, context)?;
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

    context.insert_entry(entry)?;
    context.register_directory_progress();

    Ok(total_size)
}
