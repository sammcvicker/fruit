//! Shared file reading utilities for extraction modules
//!
//! This module provides common file I/O patterns used across the codebase
//! for reading source files with size limits and extension detection.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

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

/// Read a source file if it meets size requirements.
///
/// Returns `None` if:
/// - File is larger than the configured MAX_FILE_SIZE
/// - File has no extension
/// - Extension is not valid UTF-8
/// - File cannot be read
///
/// Returns `Some((content, extension))` on success.
pub fn read_source_file(path: &Path) -> Option<(String, &str)> {
    // Check file size first
    if let Ok(metadata) = path.metadata() {
        if metadata.len() > get_max_file_size() {
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
