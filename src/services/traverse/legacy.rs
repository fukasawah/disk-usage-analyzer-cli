//! Legacy traversal implementation using standard library primitives.
//! This module preserves the pre-optimization traversal logic and acts as a
//! fallback strategy when optimized backends are unavailable or explicitly
//! disabled.

use super::StrategyKind;
use super::progress::ProgressThrottler;
use crate::models::{DirectoryEntry, ErrorItem, ProgressSnapshot};
use crate::{HardlinkPolicy, ScanOptions, SizeBasis};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::time::{Duration, Instant};

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

/// Traversal context to track state during directory walk
pub struct TraversalContext {
    root_device: Mutex<Option<u64>>,
    seen_inodes: Mutex<HashSet<FileId>>,
    entries: Mutex<HashMap<PathBuf, DirectoryEntry>>,
    errors: Mutex<Vec<ErrorItem>>,
    pub options: ScanOptions,
    pub max_depth: Option<u16>,
    strategy: AtomicU8,
    processed_entries: AtomicU64,
    processed_bytes: AtomicU64,
    progress_events: Mutex<Vec<ProgressSnapshot>>,
    progress_throttler: Mutex<ProgressThrottler>,
    start_instant: Instant,
    pub progress_interval: Duration,
}

fn encode_strategy(kind: StrategyKind) -> u8 {
    match kind {
        StrategyKind::Legacy => 0,
        StrategyKind::WindowsOptimized => 1,
        StrategyKind::PosixOptimized => 2,
    }
}

fn decode_strategy(value: u8) -> StrategyKind {
    match value {
        1 => StrategyKind::WindowsOptimized,
        2 => StrategyKind::PosixOptimized,
        _ => StrategyKind::Legacy,
    }
}

impl TraversalContext {
    #[must_use]
    pub fn new(options: ScanOptions, max_depth: Option<u16>) -> Self {
        let interval = options.progress_interval;
        let trigger = options.progress_byte_trigger;
        Self {
            root_device: Mutex::new(None),
            seen_inodes: Mutex::new(HashSet::new()),
            entries: Mutex::new(HashMap::new()),
            errors: Mutex::new(Vec::new()),
            options,
            max_depth,
            strategy: AtomicU8::new(encode_strategy(StrategyKind::Legacy)),
            processed_entries: AtomicU64::new(0),
            processed_bytes: AtomicU64::new(0),
            progress_events: Mutex::new(Vec::new()),
            progress_throttler: Mutex::new(ProgressThrottler::with_interval_and_trigger(
                interval, trigger,
            )),
            start_instant: Instant::now(),
            progress_interval: interval,
        }
    }

    #[must_use]
    pub fn strategy(&self) -> StrategyKind {
        decode_strategy(self.strategy.load(Ordering::Relaxed))
    }

    pub fn set_strategy(&self, strategy: StrategyKind) {
        self.strategy
            .store(encode_strategy(strategy), Ordering::Relaxed);
    }

    pub fn update_progress_interval(&mut self, interval: Duration) {
        self.progress_interval = interval;
        let trigger = self.options.progress_byte_trigger;
        match self.progress_throttler.get_mut() {
            Ok(throttler) => throttler.set_interval(interval, trigger),
            Err(err) => err.into_inner().set_interval(interval, trigger),
        }
    }

    #[must_use]
    pub fn root_device(&self) -> Option<u64> {
        *self.root_device.lock().unwrap()
    }

    pub fn set_root_device_if_absent(&self, device: u64) {
        if self.options.cross_filesystem {
            return;
        }

        let mut guard = self.root_device.lock().unwrap();
        if guard.is_none() {
            *guard = Some(device);
        }
    }

    /// Check if we should count this file (based on hardlink policy)
    pub(crate) fn should_count_file(&self, path: &Path, metadata: &fs::Metadata) -> bool {
        match self.options.hardlink_policy {
            HardlinkPolicy::Count => true,
            HardlinkPolicy::Dedupe => {
                if let Some(file_id) = file_id_from_metadata(path, metadata) {
                    let mut seen = self.seen_inodes.lock().unwrap();
                    seen.insert(file_id)
                } else {
                    true
                }
            }
        }
    }

    /// Get size based on the configured basis
    #[allow(unused_variables)]
    pub(crate) fn get_size(&self, path: &Path, metadata: &fs::Metadata) -> u64 {
        use crate::services::size;

        match self.options.basis {
            SizeBasis::Logical => {
                let size = size::logical_size(metadata);
                log::trace!("Logical size for {}: {size}", path.display());
                size
            }
            SizeBasis::Physical => {
                #[cfg(unix)]
                {
                    let size = size::physical_size_from_metadata(metadata);
                    log::trace!("Physical size (Unix) for {}: {size}", path.display());
                    size
                }
                #[cfg(windows)]
                {
                    let size = size::physical_size_from_path(path).unwrap_or_else(|err| {
                        log::warn!(
                            "Failed to get physical size for {}: {err}, falling back to logical size",
                            path.display()
                        );
                        metadata.len()
                    });
                    log::trace!("Physical size (Windows) for {}: {size}", path.display());
                    size
                }
                #[cfg(not(any(unix, windows)))]
                {
                    let size = size::physical_size_from_metadata(metadata);
                    log::trace!("Physical size (other) for {}: {size}", path.display());
                    size
                }
            }
        }
    }

    /// Record an error encountered during traversal
    pub(crate) fn record_error(&self, path: &Path, error: &std::io::Error) {
        let code = match error.kind() {
            std::io::ErrorKind::NotFound => "ENOENT",
            std::io::ErrorKind::PermissionDenied => "EACCES",
            _ => "IO",
        };

        let mut errors = self.errors.lock().unwrap();
        errors.push(ErrorItem {
            path: path.to_string_lossy().to_string(),
            code: code.to_string(),
            message: error.to_string(),
        });
    }

    /// Register file progress metrics and consider emitting a snapshot.
    pub fn register_file_progress(&self, size_bytes: u64) {
        let entries = self.processed_entries.fetch_add(1, Ordering::Relaxed) + 1;
        let bytes = self
            .processed_bytes
            .fetch_add(size_bytes, Ordering::Relaxed)
            + size_bytes;
        self.maybe_emit_progress(entries, bytes);
    }

    /// Register directory progress metrics and consider emitting a snapshot.
    pub fn register_directory_progress(&self) {
        let entries = self.processed_entries.fetch_add(1, Ordering::Relaxed) + 1;
        let bytes = self.processed_bytes.load(Ordering::Relaxed);
        self.maybe_emit_progress(entries, bytes);
    }

    fn maybe_emit_progress(&self, processed_entries: u64, processed_bytes: u64) {
        let now = Instant::now();
        let elapsed_ms = self.elapsed_ms(now);

        let mut throttler = self.progress_throttler.lock().unwrap();
        if let Some(snapshot) =
            throttler.consider(now, processed_bytes, processed_entries, elapsed_ms)
        {
            drop(throttler);

            if let Some(notifier) = &self.options.progress_notifier {
                notifier(&snapshot);
            }

            let mut events = self.progress_events.lock().unwrap();
            events.push(snapshot);
        }
    }

    fn elapsed_ms(&self, now: Instant) -> u64 {
        let millis = now
            .checked_duration_since(self.start_instant)
            .unwrap_or_default()
            .as_millis();
        u64::try_from(millis).unwrap_or(u64::MAX)
    }

    /// Emit a final snapshot capturing completion metrics.
    pub fn finalize_progress(&self) {
        let now = Instant::now();
        let elapsed_ms = self.elapsed_ms(now);
        let bytes = self.processed_bytes.load(Ordering::Relaxed);
        let entries = self.processed_entries.load(Ordering::Relaxed);

        let mut throttler = self.progress_throttler.lock().unwrap();
        if let Some(snapshot) = throttler.force_emit(now, bytes, entries, elapsed_ms) {
            drop(throttler);

            let mut events = self.progress_events.lock().unwrap();
            let should_append = events.last().is_none_or(|last| {
                last.processed_entries != snapshot.processed_entries
                    || last.processed_bytes != snapshot.processed_bytes
            });

            let snapshot_for_notifier = if should_append {
                Some(snapshot.clone())
            } else {
                None
            };

            if should_append {
                events.push(snapshot);
            }
            drop(events);

            if let Some(snapshot) = snapshot_for_notifier
                && let Some(notifier) = &self.options.progress_notifier
            {
                notifier(&snapshot);
            }
        }
    }

    pub fn insert_entry(&self, path: PathBuf, entry: DirectoryEntry) {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(path, entry);
    }

    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        Vec<DirectoryEntry>,
        Vec<ErrorItem>,
        Vec<ProgressSnapshot>,
        StrategyKind,
    ) {
        let strategy = decode_strategy(self.strategy.load(Ordering::Relaxed));

        let entries = self
            .entries
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .into_values()
            .collect();

        let errors = self
            .errors
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let progress = self
            .progress_events
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        (entries, errors, progress, strategy)
    }
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

/// Normalize path for cross-platform storage
#[cfg(windows)]
pub(crate) fn normalize_path(path: &Path) -> String {
    use std::borrow::Cow;
    let path_str = path.to_string_lossy();

    if path_str.contains('\\') {
        path_str.replace('\\', "/")
    } else {
        match path_str {
            Cow::Borrowed(s) => s.to_string(),
            Cow::Owned(s) => s,
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn normalize_path(path: &Path) -> String {
    use std::borrow::Cow;
    let path_str = path.to_string_lossy();

    match path_str {
        Cow::Borrowed(s) => s.to_string(),
        Cow::Owned(s) => s,
    }
}

/// Get filesystem device ID for boundary detection
#[cfg(unix)]
pub(crate) fn get_device_id(metadata: &fs::Metadata) -> u64 {
    metadata.dev()
}

#[cfg(not(unix))]
pub(crate) fn get_device_id(_metadata: &fs::Metadata) -> u64 {
    0
}

/// Traverse a directory tree and collect entries using the legacy algorithm.
pub fn traverse_directory<P: AsRef<Path>>(
    root: P,
    context: &TraversalContext,
) -> std::io::Result<u64> {
    let root = root.as_ref();

    let root_metadata = match fs::symlink_metadata(root) {
        Ok(m) => m,
        Err(e) => {
            context.record_error(root, &e);
            return Ok(0);
        }
    };

    if !context.options.cross_filesystem {
        context.set_root_device_if_absent(get_device_id(&root_metadata));
    }

    traverse_recursive(root, 0, context)
}

#[allow(clippy::too_many_lines)]
fn traverse_recursive(
    current: &Path,
    depth: u16,
    context: &TraversalContext,
) -> std::io::Result<u64> {
    if let Some(max_depth) = context.max_depth
        && depth > max_depth
    {
        return Ok(0);
    }

    let metadata = match fs::symlink_metadata(current) {
        Ok(m) => m,
        Err(e) => {
            context.record_error(current, &e);
            return Ok(0);
        }
    };

    if metadata.is_symlink() && !context.options.follow_symlinks {
        return Ok(0);
    }

    if !context.options.cross_filesystem
        && let Some(root_dev) = context.root_device()
    {
        let current_dev = get_device_id(&metadata);
        if current_dev != root_dev {
            return Ok(0);
        }
    }

    if metadata.is_file() {
        let size = if context.should_count_file(current, &metadata) {
            context.get_size(current, &metadata)
        } else {
            0
        };
        Ok(size)
    } else if metadata.is_dir() {
        let mut total_size = 0u64;
        let mut file_count = 0u32;
        let mut dir_count = 0u32;

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
                context.register_file_progress(file_size);

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
                    context.insert_entry(entry_path, file_entry);
                }
            } else if entry_metadata.is_dir() {
                let subdir_size = traverse_recursive(&entry_path, depth + 1, context)?;
                total_size += subdir_size;
                dir_count += 1;
            }
        }

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

        context.insert_entry(current.to_path_buf(), entry);
        context.register_directory_progress();

        Ok(total_size)
    } else {
        Ok(0)
    }
}
