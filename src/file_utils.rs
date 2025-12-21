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
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Global mutex to serialize tests that modify MAX_FILE_SIZE
    // This prevents parallel tests from interfering with each other
    static MAX_FILE_SIZE_TEST_LOCK: Mutex<()> = Mutex::new(());

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

    #[test]
    fn test_file_at_max_size_boundary() {
        // Lock to prevent interference from other tests modifying MAX_FILE_SIZE
        let _lock = MAX_FILE_SIZE_TEST_LOCK.lock().unwrap();

        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("boundary.rs");

        // Save original max size and ensure we restore it even on panic
        let original_max = get_max_file_size();

        // Set test max size
        let test_max_size = 50_000u64;
        set_max_file_size(test_max_size);

        // File exactly at limit should be read (implementation uses > not >=)
        let content = "x".repeat(test_max_size as usize);
        fs::write(&file_path, &content).unwrap();
        let metadata = std::fs::metadata(&file_path).unwrap();
        assert_eq!(
            metadata.len(),
            test_max_size,
            "sanity check: file is exactly at max size"
        );

        let result = read_source_file(&file_path);
        assert!(
            result.is_some(),
            "file exactly at max size should be read (uses > not >=)"
        );

        // File one byte over should be skipped
        let content = "x".repeat((test_max_size + 1) as usize);
        fs::write(&file_path, &content).unwrap();
        let metadata = std::fs::metadata(&file_path).unwrap();
        assert_eq!(
            metadata.len(),
            test_max_size + 1,
            "sanity check: file is one byte over"
        );

        let result = read_source_file(&file_path);
        assert!(result.is_none(), "file over max size should be skipped");

        // Restore original max size
        set_max_file_size(original_max);
    }

    #[test]
    fn test_file_size_just_under_boundary() {
        // Lock to prevent interference from other tests modifying MAX_FILE_SIZE
        let _lock = MAX_FILE_SIZE_TEST_LOCK.lock().unwrap();

        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("under.rs");

        // Save original max size
        let original_max = get_max_file_size();

        // Set test max size
        let test_max_size = 50_000u64;
        set_max_file_size(test_max_size);

        // File one byte under limit should be read
        let content = "x".repeat((test_max_size - 1) as usize);
        fs::write(&file_path, &content).unwrap();

        let result = read_source_file(&file_path);

        // Restore original max size
        set_max_file_size(original_max);

        assert!(result.is_some(), "file under max size should be read");
    }

    #[test]
    fn test_invalid_utf8_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("invalid.rs");

        // Write invalid UTF-8 bytes
        fs::write(&file_path, &[0xFF, 0xFE, 0x00, 0x01]).unwrap();

        // Should return None when trying to read as UTF-8
        let result = read_source_file(&file_path);
        assert!(
            result.is_none(),
            "file with invalid UTF-8 should return None"
        );
    }

    #[test]
    fn test_utf8_with_bom() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("bom.rs");

        // UTF-8 BOM followed by valid content
        let mut content = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        content.extend_from_slice(b"fn main() {}");
        fs::write(&file_path, &content).unwrap();

        let result = read_source_file(&file_path);
        assert!(result.is_some(), "UTF-8 file with BOM should be readable");
        let (text, ext) = result.unwrap();
        assert_eq!(ext, "rs");
        // BOM should be present in the string (Rust's read_to_string preserves it)
        assert!(text.starts_with('\u{FEFF}') || text.starts_with("fn"));
    }

    #[test]
    fn test_set_max_file_size() {
        // Lock to prevent interference from other tests modifying MAX_FILE_SIZE
        let _lock = MAX_FILE_SIZE_TEST_LOCK.lock().unwrap();

        // Save original value
        let original = get_max_file_size();

        // Test setting a new value
        set_max_file_size(500_000);
        assert_eq!(get_max_file_size(), 500_000);

        set_max_file_size(2_000_000);
        assert_eq!(get_max_file_size(), 2_000_000);

        // Restore original value
        set_max_file_size(original);
    }

    #[test]
    fn test_max_file_size_thread_safety() {
        use std::thread;

        // Lock to prevent interference from other tests modifying MAX_FILE_SIZE
        let _lock = MAX_FILE_SIZE_TEST_LOCK.lock().unwrap();

        // This test verifies that set_max_file_size and get_max_file_size
        // can be called concurrently without panicking or data races.
        // We use AtomicU64 with SeqCst ordering, so concurrent access is safe.

        // Save original value
        let original = get_max_file_size();

        // Use a unique range for thread testing
        let base_size = 800_000u64;

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let size = base_size + (i * 1000);
                thread::spawn(move || {
                    set_max_file_size(size);
                    // Verify we can read back without panicking
                    let _ = get_max_file_size();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // The final value will be one of the values set by the threads
        // We can't predict which one due to race conditions, but it should be valid
        let final_value = get_max_file_size();
        // Just verify no corruption occurred - value should be reasonable
        assert!(
            final_value > 0 && final_value < 10_000_000,
            "value should be reasonable, got {}",
            final_value
        );

        // Restore original value
        set_max_file_size(original);
    }

    #[test]
    fn test_empty_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("empty.rs");
        fs::write(&file_path, "").unwrap();

        let result = read_source_file(&file_path);
        assert!(result.is_some(), "empty file should be readable");
        let (content, ext) = result.unwrap();
        assert_eq!(content, "");
        assert_eq!(ext, "rs");
    }

    #[test]
    fn test_file_with_special_characters() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("special.rs");

        // Unicode characters, emojis, special whitespace
        let content = "// Hello 世界 🦀\nfn main() {\n\tprintln!(\"Hello\");\n}";
        fs::write(&file_path, content).unwrap();

        let result = read_source_file(&file_path);
        assert!(
            result.is_some(),
            "file with special chars should be readable"
        );
        let (text, ext) = result.unwrap();
        assert_eq!(text, content);
        assert_eq!(ext, "rs");
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_handling() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().unwrap();
        let target_path = dir.path().join("target.rs");
        let symlink_path = dir.path().join("link.rs");

        // Create target file
        fs::write(&target_path, "fn main() {}").unwrap();

        // Create symlink to target
        symlink(&target_path, &symlink_path).unwrap();

        // Should be able to read through symlink
        let result = read_source_file(&symlink_path);
        assert!(result.is_some(), "should be able to read through symlink");
        let (content, ext) = result.unwrap();
        assert_eq!(content, "fn main() {}");
        assert_eq!(ext, "rs");
    }

    #[cfg(unix)]
    #[test]
    fn test_broken_symlink() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().unwrap();
        let target_path = dir.path().join("nonexistent.rs");
        let symlink_path = dir.path().join("broken_link.rs");

        // Create symlink to non-existent target
        symlink(&target_path, &symlink_path).unwrap();

        // Should return None for broken symlink
        let result = read_source_file(&symlink_path);
        assert!(result.is_none(), "broken symlink should return None");
    }

    #[test]
    fn test_multiple_dots_in_filename() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("my.test.file.rs");
        fs::write(&file_path, "fn test() {}").unwrap();

        let result = read_source_file(&file_path);
        assert!(result.is_some(), "file with multiple dots should work");
        let (_, ext) = result.unwrap();
        assert_eq!(ext, "rs", "should use the final extension");
    }

    #[test]
    fn test_default_max_file_size() {
        assert_eq!(DEFAULT_MAX_FILE_SIZE, 1_000_000);
    }
}
