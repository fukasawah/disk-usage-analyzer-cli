//! Unit tests for aggregation and size basis conversions

#[cfg(test)]
mod tests {
    use dua::models::DirectoryEntry;
    use dua::services::aggregate::{
        DirectoryShard, EntryKind, SortBy, consolidate_shards, sort_and_limit,
    };
    use rayon::prelude::*;
    use std::convert::TryFrom;

    #[test]
    fn test_sort_by_size() {
        let mut entries = vec![
            DirectoryEntry {
                path: "a".to_string(),
                parent_path: None,
                depth: 0,
                size_bytes: 100,
                file_count: 1,
                dir_count: 0,
            },
            DirectoryEntry {
                path: "b".to_string(),
                parent_path: None,
                depth: 0,
                size_bytes: 500,
                file_count: 2,
                dir_count: 0,
            },
            DirectoryEntry {
                path: "c".to_string(),
                parent_path: None,
                depth: 0,
                size_bytes: 200,
                file_count: 3,
                dir_count: 0,
            },
        ];

        entries = sort_and_limit(entries, SortBy::Size, None);

        assert_eq!(entries[0].path, "b");
        assert_eq!(entries[1].path, "c");
        assert_eq!(entries[2].path, "a");
    }

    #[test]
    fn test_top_k_limiting() {
        let entries = vec![
            DirectoryEntry {
                path: "a".to_string(),
                parent_path: None,
                depth: 0,
                size_bytes: 100,
                file_count: 1,
                dir_count: 0,
            },
            DirectoryEntry {
                path: "b".to_string(),
                parent_path: None,
                depth: 0,
                size_bytes: 500,
                file_count: 2,
                dir_count: 0,
            },
            DirectoryEntry {
                path: "c".to_string(),
                parent_path: None,
                depth: 0,
                size_bytes: 200,
                file_count: 3,
                dir_count: 0,
            },
        ];

        let limited = sort_and_limit(entries, SortBy::Size, Some(2));

        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].path, "b");
        assert_eq!(limited[1].path, "c");
    }

    #[test]
    fn test_sort_by_files() {
        let mut entries = vec![
            DirectoryEntry {
                path: "a".to_string(),
                parent_path: None,
                depth: 0,
                size_bytes: 100,
                file_count: 5,
                dir_count: 0,
            },
            DirectoryEntry {
                path: "b".to_string(),
                parent_path: None,
                depth: 0,
                size_bytes: 500,
                file_count: 2,
                dir_count: 0,
            },
        ];

        entries = sort_and_limit(entries, SortBy::Files, None);

        assert_eq!(entries[0].path, "a");
        assert_eq!(entries[1].path, "b");
    }

    #[test]
    fn test_directory_shard_absorb_and_merge() {
        let mut shard_a = DirectoryShard::with_capacity(2);
        shard_a.absorb_entry(
            DirectoryEntry {
                path: "root/file_a".to_string(),
                parent_path: Some("root".to_string()),
                depth: 1,
                size_bytes: 1024,
                file_count: 0,
                dir_count: 0,
            },
            EntryKind::File,
        );

        let mut shard_b = DirectoryShard::default();
        shard_b.absorb_entry(
            DirectoryEntry {
                path: "root/dir_b".to_string(),
                parent_path: Some("root".to_string()),
                depth: 1,
                size_bytes: 4096,
                file_count: 3,
                dir_count: 1,
            },
            EntryKind::Directory,
        );

        shard_a.merge_in_place(shard_b);
        let totals = *shard_a.totals();

        assert_eq!(totals.size_bytes, 1024 + 4096);
        assert_eq!(totals.files, 1);
        assert_eq!(totals.directories, 1);
    }

    #[test]
    fn test_consolidate_shards() {
        let mut shard_one = DirectoryShard::default();
        shard_one.absorb_entry(
            DirectoryEntry {
                path: "root/a".to_string(),
                parent_path: Some("root".to_string()),
                depth: 1,
                size_bytes: 512,
                file_count: 0,
                dir_count: 0,
            },
            EntryKind::File,
        );

        let mut shard_two = DirectoryShard::default();
        shard_two.absorb_entry(
            DirectoryEntry {
                path: "root/b".to_string(),
                parent_path: Some("root".to_string()),
                depth: 1,
                size_bytes: 2048,
                file_count: 0,
                dir_count: 0,
            },
            EntryKind::File,
        );

        shard_two.absorb_entry(
            DirectoryEntry {
                path: "root/dir".to_string(),
                parent_path: Some("root".to_string()),
                depth: 1,
                size_bytes: 8192,
                file_count: 5,
                dir_count: 1,
            },
            EntryKind::Directory,
        );

        let (entries, totals) = consolidate_shards(vec![shard_one, shard_two]);

        assert_eq!(entries.len(), 3);
        assert_eq!(totals.size_bytes, 512 + 2048 + 8192);
        assert_eq!(totals.files, 2);
        assert_eq!(totals.directories, 1);
    }

    #[test]
    fn test_consolidate_parallel_shards() {
        const SHARDS: usize = 4;
        const FILES_PER_SHARD: usize = 32;
        const SIZE_PER_FILE: u64 = 1024;

        let files_per_shard_u32 =
            u32::try_from(FILES_PER_SHARD).expect("files per shard fits in u32");
        let files_per_shard_u64 =
            u64::try_from(FILES_PER_SHARD).expect("files per shard fits in u64");
        let shards_u64 = u64::try_from(SHARDS).expect("shard count fits in u64");

        let shards: Vec<DirectoryShard> = (0..SHARDS)
            .into_par_iter()
            .map(|shard_idx| {
                let mut shard = DirectoryShard::default();

                for file_idx in 0..FILES_PER_SHARD {
                    let path = format!("root/dir{shard_idx}/file{file_idx:03}.bin");
                    let parent = format!("root/dir{shard_idx}");
                    shard.absorb_entry(
                        DirectoryEntry {
                            path,
                            parent_path: Some(parent.clone()),
                            depth: 2,
                            size_bytes: SIZE_PER_FILE,
                            file_count: 0,
                            dir_count: 0,
                        },
                        EntryKind::File,
                    );
                }

                // Each shard emits the directory aggregate it is responsible for.
                shard.absorb_entry(
                    DirectoryEntry {
                        path: format!("root/dir{shard_idx}"),
                        parent_path: Some("root".to_string()),
                        depth: 1,
                        size_bytes: SIZE_PER_FILE * files_per_shard_u64,
                        file_count: files_per_shard_u32,
                        dir_count: 0,
                    },
                    EntryKind::Directory,
                );

                shard
            })
            .collect();

        let (entries, totals) = consolidate_shards(shards);

        let expected_files = SHARDS * FILES_PER_SHARD;
        let expected_entries = expected_files + SHARDS; // per-file + per-directory entries
        let total_files = files_per_shard_u64 * shards_u64;

        assert_eq!(entries.len(), expected_entries);
        assert_eq!(totals.files, total_files);
        assert_eq!(totals.directories, shards_u64);

        // Totals should reflect both file and directory contributions folded across shards.
        let files_size = SIZE_PER_FILE * total_files;
        let dir_size = SIZE_PER_FILE * total_files;
        let expected_size = files_size + dir_size;
        assert_eq!(totals.size_bytes, expected_size);

        // Directory aggregates should remain consistent regardless of merge order.
        let mut dir_entries = entries
            .iter()
            .filter(|entry| entry.depth == 1)
            .collect::<Vec<_>>();
        assert_eq!(dir_entries.len(), SHARDS);
        dir_entries.sort_by(|a, b| a.path.cmp(&b.path));

        for (idx, entry) in dir_entries.into_iter().enumerate() {
            assert_eq!(entry.path, format!("root/dir{idx}"));
            assert_eq!(
                usize::try_from(entry.file_count).expect("file count fits in usize"),
                FILES_PER_SHARD
            );
            assert_eq!(entry.dir_count, 0);
            assert_eq!(entry.size_bytes, SIZE_PER_FILE * files_per_shard_u64);
        }
    }
}
