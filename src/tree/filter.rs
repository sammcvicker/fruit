//! File filtering for tree walking

use std::path::Path;

use crate::git::GitignoreFilter;

/// File filter based on .gitignore patterns.
/// This is a newtype wrapper around GitignoreFilter for future extensibility.
pub struct FileFilter(GitignoreFilter);

impl FileFilter {
    /// Create a new file filter from a GitignoreFilter.
    pub fn new(filter: GitignoreFilter) -> Self {
        Self(filter)
    }

    /// Check if a path should be included.
    pub fn is_included(&self, path: &Path) -> bool {
        self.0.is_included(path)
    }
}
