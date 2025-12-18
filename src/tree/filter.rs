//! File filtering for tree walking

use std::path::Path;

use crate::git::{GitFilter, GitignoreFilter};

/// File filter that can use either gitignore patterns or git tracking status.
pub enum FileFilter {
    /// Filter based on .gitignore patterns (default)
    Gitignore(GitignoreFilter),
    /// Filter based on git tracking status (--tracked mode)
    GitTracked(GitFilter),
}

impl FileFilter {
    /// Check if a path should be included.
    pub fn is_included(&self, path: &Path) -> bool {
        match self {
            FileFilter::Gitignore(f) => f.is_included(path),
            FileFilter::GitTracked(f) => f.is_tracked(path),
        }
    }
}
