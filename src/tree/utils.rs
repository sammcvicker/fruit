//! Shared utility functions for tree walking

use std::path::Path;

use glob::Pattern;

use super::config::WalkerConfig;
use super::filter::FileFilter;

/// Check if a directory has any included files (used for pruning empty directories).
pub fn has_included_files(path: &Path, filter: &Option<FileFilter>) -> bool {
    if let Some(f) = filter {
        f.is_included(path)
    } else {
        // Without filter, assume directory has content
        true
    }
}

/// Check if a path should be included based on filter, show_all flag, and time filters.
pub fn should_include_path(path: &Path, config: &WalkerConfig, filter: &Option<FileFilter>) -> bool {
    // Check gitignore filter
    if !config.show_all {
        if let Some(f) = filter {
            if !f.is_included(path) {
                return false;
            }
        }
    }

    // Check time filter (applies to files only)
    if path.is_file() && !passes_time_filter(path, config) {
        return false;
    }

    true
}

/// Check if a path should be ignored based on name and ignore patterns.
pub fn should_ignore_path(path: &Path, ignore_patterns: &[String]) -> bool {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // Always ignore .git directory
    if name == ".git" {
        return true;
    }

    // Check custom ignore patterns
    for pattern in ignore_patterns {
        if name == *pattern || glob_match(pattern, &name) {
            return true;
        }
    }

    false
}

/// Match a glob pattern against a name.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    Pattern::new(pattern)
        .map(|p| p.matches(name))
        .unwrap_or(false)
}

/// Get file size and return both bytes and human-readable format.
pub fn get_file_size(path: &Path) -> (Option<u64>, Option<String>) {
    match path.metadata() {
        Ok(meta) => {
            let size = meta.len();
            (Some(size), Some(format_size(size)))
        }
        Err(_) => (None, None),
    }
}

/// Format a size in bytes to human-readable format.
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// Check if a file passes the time filter based on its modification time.
pub fn passes_time_filter(path: &Path, config: &WalkerConfig) -> bool {
    // If no time filters, pass
    if config.newer_than.is_none() && config.older_than.is_none() {
        return true;
    }

    // Get modification time
    let mtime = match path.metadata().and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return true, // If we can't get mtime, include the file
    };

    // Check newer_than filter
    if let Some(newer) = config.newer_than {
        if mtime < newer {
            return false;
        }
    }

    // Check older_than filter
    if let Some(older) = config.older_than {
        if mtime > older {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        // Basic patterns
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("*.rs", "lib.rs"));
        assert!(!glob_match("*.rs", "main.py"));
        assert!(glob_match("test*", "test_foo"));
        assert!(!glob_match("test*", "foo_test"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "notexact"));

        // Single character wildcard
        assert!(glob_match("test?.rs", "test1.rs"));
        assert!(glob_match("test?.rs", "testa.rs"));
        assert!(!glob_match("test?.rs", "test12.rs"));

        // Character classes
        assert!(glob_match("[abc].txt", "a.txt"));
        assert!(glob_match("[abc].txt", "b.txt"));
        assert!(!glob_match("[abc].txt", "d.txt"));

        // Character ranges
        assert!(glob_match("[a-z].txt", "x.txt"));
        assert!(!glob_match("[a-z].txt", "X.txt"));
    }
}
