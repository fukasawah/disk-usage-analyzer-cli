//! Tests for path normalization logic

use std::path::Path;

// Re-implement normalize_path logic for testing
#[cfg(windows)]
fn normalize_path(path: &Path) -> String {
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
fn normalize_path(path: &Path) -> String {
    use std::borrow::Cow;
    let path_str = path.to_string_lossy();

    match path_str {
        Cow::Borrowed(s) => s.to_string(),
        Cow::Owned(s) => s,
    }
}

#[test]
#[cfg(windows)]
fn test_windows_backslash_conversion() {
    let path = Path::new(r"C:\Users\test\file.txt");
    let normalized = normalize_path(path);
    assert_eq!(normalized, "C:/Users/test/file.txt");
}

#[test]
#[cfg(windows)]
fn test_windows_mixed_slashes() {
    let path = Path::new(r"C:\Users/test\file.txt");
    let normalized = normalize_path(path);
    assert_eq!(normalized, "C:/Users/test/file.txt");
}

#[test]
#[cfg(windows)]
fn test_windows_forward_slashes_only() {
    // If path already uses forward slashes, no replacement needed
    let path = Path::new("C:/Users/test/file.txt");
    let normalized = normalize_path(path);
    assert_eq!(normalized, "C:/Users/test/file.txt");
}

#[test]
#[cfg(windows)]
fn test_windows_unc_path() {
    let path = Path::new(r"\\server\share\file.txt");
    let normalized = normalize_path(path);
    assert_eq!(normalized, "//server/share/file.txt");
}

#[test]
#[cfg(unix)]
fn test_unix_backslash_in_filename() {
    // On Unix, backslash is a valid character in filenames
    let path = Path::new("/home/user/file\\with\\backslash.txt");
    let normalized = normalize_path(path);
    assert_eq!(normalized, "/home/user/file\\with\\backslash.txt");
}

#[test]
#[cfg(unix)]
fn test_unix_normal_path() {
    let path = Path::new("/home/user/documents/file.txt");
    let normalized = normalize_path(path);
    assert_eq!(normalized, "/home/user/documents/file.txt");
}

#[test]
#[cfg(unix)]
fn test_unix_relative_path() {
    let path = Path::new("./relative/path/file.txt");
    let normalized = normalize_path(path);
    assert_eq!(normalized, "./relative/path/file.txt");
}

#[test]
fn test_empty_path() {
    let path = Path::new("");
    let normalized = normalize_path(path);
    assert_eq!(normalized, "");
}

#[test]
fn test_single_dot() {
    let path = Path::new(".");
    let normalized = normalize_path(path);
    assert_eq!(normalized, ".");
}

#[test]
fn test_double_dot() {
    let path = Path::new("..");
    let normalized = normalize_path(path);
    assert_eq!(normalized, "..");
}

#[test]
#[cfg(windows)]
fn test_windows_root_only() {
    let path = Path::new(r"C:\");
    let normalized = normalize_path(path);
    assert_eq!(normalized, "C:/");
}

#[test]
#[cfg(unix)]
fn test_unix_root_only() {
    let path = Path::new("/");
    let normalized = normalize_path(path);
    assert_eq!(normalized, "/");
}

// Performance-related tests
#[test]
fn test_multiple_normalizations_same_path() {
    // Ensure multiple normalizations produce consistent results
    #[cfg(windows)]
    let path = Path::new(r"C:\test\path");
    #[cfg(not(windows))]
    let path = Path::new("/test/path");

    let first = normalize_path(path);
    let second = normalize_path(path);
    let third = normalize_path(path);

    assert_eq!(first, second);
    assert_eq!(second, third);
}

#[test]
#[cfg(windows)]
fn test_windows_deep_path() {
    let path = Path::new(r"C:\very\deep\directory\structure\with\many\levels\file.txt");
    let normalized = normalize_path(path);
    assert_eq!(
        normalized,
        "C:/very/deep/directory/structure/with/many/levels/file.txt"
    );
}

#[test]
#[cfg(unix)]
fn test_unix_deep_path() {
    let path = Path::new("/very/deep/directory/structure/with/many/levels/file.txt");
    let normalized = normalize_path(path);
    assert_eq!(
        normalized,
        "/very/deep/directory/structure/with/many/levels/file.txt"
    );
}
