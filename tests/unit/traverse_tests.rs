//! Unit tests for traversal filtering (symlinks, filesystem boundaries)

#[cfg(test)]
mod tests {
    use crate::fixtures::write_file_sync;
    use dua::services::traverse::detect::FilesystemKind;
    use dua::services::traverse::{StrategyKind, TraversalContext, TraversalDispatcher, detect};
    use dua::{ScanOptions, SizeBasis};
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_basic_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test structure
        fs::create_dir_all(root.join("subdir")).unwrap();
        write_file_sync(root.join("file1.txt"), b"hello").unwrap();
        write_file_sync(root.join("subdir/file2.txt"), b"world").unwrap();

        let opts = ScanOptions {
            basis: SizeBasis::Logical,
            ..ScanOptions::default()
        };

        let result = dua::scan_summary(root, &opts);
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert!(!summary.entries.is_empty());
        assert_eq!(summary.errors.len(), 0);
    }

    #[test]
    fn test_invalid_path() {
        let opts = ScanOptions::default();
        let result = dua::scan_summary("/nonexistent/path/12345", &opts);
        assert!(result.is_err());
    }

    #[test]
    fn dispatcher_respects_strategy_override() {
        let opts = ScanOptions {
            strategy_override: Some(StrategyKind::Legacy),
            ..ScanOptions::default()
        };
        let dispatcher = TraversalDispatcher::for_platform(&opts);
        assert_eq!(dispatcher.active_strategy(), StrategyKind::Legacy);
    }

    #[test]
    fn maps_filesystem_kinds_to_expected_strategies() {
        assert_eq!(
            detect::strategy_for_filesystem(FilesystemKind::Ntfs),
            StrategyKind::WindowsOptimized
        );
        assert_eq!(
            detect::strategy_for_filesystem(FilesystemKind::Apfs),
            StrategyKind::PosixOptimized
        );
        assert_eq!(
            detect::strategy_for_filesystem(FilesystemKind::Ext),
            StrategyKind::PosixOptimized
        );
        assert_eq!(
            detect::strategy_for_filesystem(FilesystemKind::Other),
            StrategyKind::Legacy
        );
    }

    #[cfg(windows)]
    #[test]
    fn dispatcher_defaults_to_windows_strategy() {
        let opts = ScanOptions::default();
        let dispatcher = TraversalDispatcher::for_platform(&opts);
        assert_eq!(dispatcher.active_strategy(), StrategyKind::WindowsOptimized);
    }

    #[cfg(not(windows))]
    #[test]
    fn dispatcher_defaults_to_non_windows_strategy() {
        let opts = ScanOptions::default();
        let dispatcher = TraversalDispatcher::for_platform(&opts);
        assert_ne!(dispatcher.active_strategy(), StrategyKind::WindowsOptimized);
    }

    #[test]
    fn dispatcher_propagates_progress_interval() {
        let opts = ScanOptions {
            progress_interval: Duration::from_millis(750),
            ..ScanOptions::default()
        };
        let dispatcher = TraversalDispatcher::for_platform(&opts);
        assert_eq!(dispatcher.progress_interval(), Duration::from_millis(750));
    }

    #[cfg(not(windows))]
    #[test]
    fn windows_override_falls_back_when_unsupported() {
        let temp_dir = TempDir::new().unwrap();
        let opts = ScanOptions::default();
        let mut context = TraversalContext::new(opts, None);
        let dispatcher = TraversalDispatcher::with_strategy(StrategyKind::WindowsOptimized, true);

        let size = dispatcher.traverse(temp_dir.path(), &mut context).unwrap();
        assert_eq!(size, 0);
        assert_eq!(context.strategy(), StrategyKind::Legacy);
    }

    #[test]
    fn detects_filesystem_kind_for_temp_dir() {
        let temp_dir = TempDir::new().unwrap();
        let fs_kind = detect::filesystem_kind_for_path(temp_dir.path());

        #[cfg(windows)]
        assert_eq!(fs_kind, detect::FilesystemKind::Ntfs);

        #[cfg(target_os = "macos")]
        assert!(matches!(
            fs_kind,
            detect::FilesystemKind::Apfs | detect::FilesystemKind::Other
        ));

        #[cfg(all(unix, not(target_os = "macos")))]
        assert!(matches!(
            fs_kind,
            detect::FilesystemKind::Ext | detect::FilesystemKind::Other
        ));
    }
}
