//! Path normalization for cross-platform trail storage.
//!
//! D1 always stores Unix-style paths. This module normalizes Windows paths
//! before pushing to the global trail.

/// Normalize a file path for cross-platform storage.
/// D1 always stores Unix-style paths.
///
/// - Replaces backslashes with forward slashes
/// - Removes Windows drive letter prefix (C:, D:, etc.)
///
/// Examples:
/// - `C:\Users\dietl\project` → `/Users/dietl/project`
/// - `/home/jacob/repos/foo` → `/home/jacob/repos/foo` (unchanged)
/// - `C:\Users\dietl\VSCode Projects\foo` → `/Users/dietl/VSCode Projects/foo`
pub fn normalize_path(raw: &str) -> String {
    let mut path = raw.replace('\\', "/");

    // Remove Windows drive letter (C:, D:, etc.)
    if path.len() >= 2 && path.as_bytes().get(1) == Some(&b':') {
        path = path[2..].to_string();
    }

    path
}

/// Normalize all paths in a JSON array string (e.g. files_read, files_written).
/// Returns the normalized JSON array string, or the original if parsing fails.
pub fn normalize_json_paths(json_str: &str) -> String {
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(json_str) {
        let normalized: Vec<String> = arr.iter().map(|p| normalize_path(p)).collect();
        serde_json::to_string(&normalized).unwrap_or_else(|_| json_str.to_string())
    } else {
        json_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_windows_path() {
        assert_eq!(
            normalize_path(r"C:\Users\dietl\VSCode Projects\foo"),
            "/Users/dietl/VSCode Projects/foo"
        );
    }

    #[test]
    fn test_normalize_unix_path_unchanged() {
        assert_eq!(
            normalize_path("/home/jacob/repos/foo"),
            "/home/jacob/repos/foo"
        );
    }

    #[test]
    fn test_normalize_drive_letter_d() {
        assert_eq!(normalize_path(r"D:\data\file.txt"), "/data/file.txt");
    }

    #[test]
    fn test_normalize_empty_string() {
        assert_eq!(normalize_path(""), "");
    }

    #[test]
    fn test_normalize_short_path() {
        assert_eq!(normalize_path("a"), "a");
    }

    #[test]
    fn test_normalize_json_paths_array() {
        let input = r#"["C:\\Users\\dietl\\foo.rs","C:\\Users\\dietl\\bar.rs"]"#;
        let result = normalize_json_paths(input);
        assert_eq!(result, r#"["/Users/dietl/foo.rs","/Users/dietl/bar.rs"]"#);
    }

    #[test]
    fn test_normalize_json_paths_invalid_json() {
        let input = "not json";
        assert_eq!(normalize_json_paths(input), "not json");
    }

    #[test]
    fn test_normalize_json_paths_empty_array() {
        assert_eq!(normalize_json_paths("[]"), "[]");
    }
}
