//! Error handling test for invalid/corrupt snapshot files

#[cfg(test)]
mod tests {
    use dua::io::snapshot::read_snapshot;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_invalid_snapshot_file() {
        let result = read_snapshot("/nonexistent/path/to/snapshot.parquet");
        assert!(result.is_err());
    }

    #[test]
    fn test_corrupt_snapshot_file() {
        let mut temp_file = NamedTempFile::new().unwrap();

        // Write invalid data
        temp_file
            .write_all(b"This is not a valid Parquet file!")
            .unwrap();
        temp_file.flush().unwrap();

        let snapshot_path = temp_file.path().to_str().unwrap();
        let result = read_snapshot(snapshot_path);

        assert!(result.is_err());
    }
}
