//! Shared file reading utilities for extraction modules
//!
//! This module provides common file I/O patterns used across the codebase
//! for reading source files with size limits and extension detection.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::language::Language;

/// Default maximum file size for extraction operations (1MB).
/// Files larger than this are skipped to prevent excessive memory usage.
pub const DEFAULT_MAX_FILE_SIZE: u64 = 1_000_000;

/// Global configurable max file size. Set via `set_max_file_size()`.
static MAX_FILE_SIZE: AtomicU64 = AtomicU64::new(DEFAULT_MAX_FILE_SIZE);

/// Set the maximum file size for extraction operations.
/// This affects all subsequent calls to `read_source_file`.
pub fn set_max_file_size(size: u64) {
    MAX_FILE_SIZE.store(size, Ordering::SeqCst);
}

/// Get the current maximum file size setting.
pub fn get_max_file_size() -> u64 {
    MAX_FILE_SIZE.load(Ordering::SeqCst)
}

/// Normalize a file extension to lowercase for case-insensitive matching.
///
/// Returns a static string reference for recognized extensions,
/// normalizing variants to their canonical form (e.g., ".RS" -> "rs").
/// Returns `None` for unrecognized extensions.
///
/// Note: This function now uses the Language enum internally to ensure
/// consistency across all extension mappings.
pub fn normalize_extension(ext: &str) -> Option<&'static str> {
    Language::from_extension(ext).map(|lang| lang.canonical_extension())
}

/// Read a source file if it meets size requirements.
///
/// Returns `None` if:
/// - File is larger than the configured MAX_FILE_SIZE
/// - File has no extension
/// - Extension is not valid UTF-8
/// - Extension is not a recognized source file type
/// - File cannot be read
///
/// Returns `Some((content, extension))` on success.
/// The extension is normalized to lowercase for case-insensitive matching.
pub fn read_source_file(path: &Path) -> Option<(String, &'static str)> {
    // Check file size first
    if let Ok(metadata) = path.metadata() {
        if metadata.len() > get_max_file_size() {
            return None;
        }
    }

    // Get extension and normalize to lowercase
    let extension = path.extension()?.to_str()?;
    let ext_static = normalize_extension(extension)?;

    // Read content
    let content = std::fs::read_to_string(path).ok()?;

    Some((content, ext_static))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_read_source_file_success() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let result = read_source_file(&file_path);
        assert!(result.is_some());
        let (content, ext) = result.unwrap();
        assert_eq!(content, "fn main() {}");
        assert_eq!(ext, "rs");
    }

    #[test]
    fn test_read_source_file_no_extension() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("Makefile");
        fs::write(&file_path, "all: build").unwrap();

        let result = read_source_file(&file_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_read_source_file_nonexistent() {
        let result = read_source_file(Path::new("/nonexistent/file.rs"));
        assert!(result.is_none());
    }

    #[test]
    fn test_read_source_file_case_insensitive() {
        let dir = TempDir::new().unwrap();

        // Test uppercase extension
        let file_path = dir.path().join("test.RS");
        fs::write(&file_path, "fn main() {}").unwrap();
        let result = read_source_file(&file_path);
        assert!(result.is_some(), "should recognize .RS as .rs");
        let (_, ext) = result.unwrap();
        assert_eq!(ext, "rs", "extension should be normalized to lowercase");

        // Test mixed case extension
        let file_path = dir.path().join("test.Py");
        fs::write(&file_path, "print('hello')").unwrap();
        let result = read_source_file(&file_path);
        assert!(result.is_some(), "should recognize .Py as .py");
        let (_, ext) = result.unwrap();
        assert_eq!(ext, "py", "extension should be normalized to lowercase");
    }

    #[test]
    fn test_normalize_extension() {
        // Basic lowercase
        assert_eq!(normalize_extension("rs"), Some("rs"));
        assert_eq!(normalize_extension("py"), Some("py"));

        // Uppercase
        assert_eq!(normalize_extension("RS"), Some("rs"));
        assert_eq!(normalize_extension("PY"), Some("py"));
        assert_eq!(normalize_extension("JS"), Some("js"));

        // Mixed case
        assert_eq!(normalize_extension("Py"), Some("py"));
        assert_eq!(normalize_extension("rS"), Some("rs"));

        // Variants normalized to canonical form
        assert_eq!(normalize_extension("jsx"), Some("js"));
        assert_eq!(normalize_extension("JSX"), Some("js"));
        assert_eq!(normalize_extension("tsx"), Some("ts"));
        assert_eq!(normalize_extension("TSX"), Some("ts"));
        assert_eq!(normalize_extension("hpp"), Some("cpp"));
        assert_eq!(normalize_extension("HPP"), Some("cpp"));

        // Unknown extension
        assert_eq!(normalize_extension("xyz"), None);
        assert_eq!(normalize_extension("XYZ"), None);
    }

    #[test]
    fn test_read_source_file_unrecognized_extension() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("data.xyz");
        fs::write(&file_path, "some data").unwrap();

        let result = read_source_file(&file_path);
        assert!(
            result.is_none(),
            "unrecognized extension should return None"
        );
    }
}
