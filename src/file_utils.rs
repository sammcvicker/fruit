//! Shared file reading utilities for extraction modules
//!
//! This module provides common file I/O patterns used across the codebase
//! for reading source files with size limits and extension detection.

use std::path::Path;

/// Maximum file size for extraction operations (1MB).
/// Files larger than this are skipped to prevent excessive memory usage.
pub const MAX_FILE_SIZE: u64 = 1_000_000;

/// Read a source file if it meets size requirements.
///
/// Returns `None` if:
/// - File is larger than MAX_FILE_SIZE
/// - File has no extension
/// - Extension is not valid UTF-8
/// - File cannot be read
///
/// Returns `Some((content, extension))` on success.
pub fn read_source_file(path: &Path) -> Option<(String, &str)> {
    // Check file size first
    if let Ok(metadata) = path.metadata() {
        if metadata.len() > MAX_FILE_SIZE {
            return None;
        }
    }

    // Get extension
    let extension = path.extension()?.to_str()?;

    // Read content
    let content = std::fs::read_to_string(path).ok()?;

    Some((content, extension))
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
}
