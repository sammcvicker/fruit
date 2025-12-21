//! Common tree traversal logic shared by TreeWalker and StreamingWalker.
//!
//! This module provides a unified traversal algorithm that eliminates code
//! duplication between the buffered (TreeWalker) and streaming (StreamingWalker)
//! implementations.

use std::path::Path;

use super::config::WalkerConfig;
use super::filter::FileFilter;
use super::utils::{has_included_files, should_ignore_path, should_include_path};

/// Common base traversal functionality shared by both walker implementations.
pub struct BaseTraversal<'a> {
    pub config: &'a WalkerConfig,
    pub filter: &'a Option<FileFilter>,
}

impl<'a> BaseTraversal<'a> {
    /// Create a new base traversal
    pub fn new(config: &'a WalkerConfig, filter: &'a Option<FileFilter>) -> Self {
        Self { config, filter }
    }

    /// Check if we're at maximum depth
    pub fn at_max_depth(&self, depth: usize) -> bool {
        self.config.max_depth.is_some_and(|max| depth >= max)
    }

    /// Check if a path should be included
    pub fn should_include(&self, path: &Path) -> bool {
        should_include_path(path, self.config, self.filter)
    }

    /// Check if a path should be ignored
    pub fn should_ignore(&self, path: &Path) -> bool {
        should_ignore_path(path, &self.config.ignore_patterns)
    }

    /// Check if a directory has included files
    pub fn has_included_files(&self, path: &Path) -> bool {
        has_included_files(path, self.filter)
    }

    /// Get the name of a path, defaulting to "." for root
    pub fn get_name(&self, path: &Path) -> String {
        path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string())
    }

    /// Read, filter, and sort directory entries
    pub fn read_and_filter_entries(&self, path: &Path) -> Option<Vec<std::fs::DirEntry>> {
        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return None,
        };

        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|a| a.file_name());

        Some(
            entries
                .into_iter()
                .filter(|entry| !self.should_ignore(&entry.path()))
                .collect(),
        )
    }

    /// Determine which entries are valid (files that pass filter, or non-empty directories)
    pub fn filter_valid_entries(
        &self,
        entries: Vec<std::fs::DirEntry>,
    ) -> Vec<(std::fs::DirEntry, bool)> {
        let mut valid_entries = Vec::new();

        for entry in entries {
            let entry_path = entry.path();

            if entry_path.is_file() {
                if self.config.dirs_only {
                    continue;
                }
                if !self.should_include(&entry_path) {
                    continue;
                }
                valid_entries.push((entry, false)); // false = is file
            } else if entry_path.is_dir()
                && !entry_path.is_symlink()
                && (self.config.dirs_only || self.has_included_files(&entry_path))
            {
                valid_entries.push((entry, true)); // true = is directory
            }
        }

        valid_entries
    }

    /// Calculate the prefix for child entries
    pub fn calculate_child_prefix(&self, current_prefix: &str, is_last: bool) -> String {
        if is_last {
            format!("{}    ", current_prefix)
        } else {
            format!("{}│   ", current_prefix)
        }
    }
}
