//! Unit tests for aggregation and size basis conversions

#[cfg(test)]
mod tests {
    use dua::models::DirectoryEntry;
    use dua::services::aggregate::{sort_and_limit, SortBy};

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
}
