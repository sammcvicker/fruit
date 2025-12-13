//! Directory tree walking logic

use std::path::{Path, PathBuf};

use crate::comments::extract_first_comment;
use crate::git::GitFilter;

#[derive(Debug, Clone)]
pub enum TreeNode {
    File {
        name: String,
        path: PathBuf,
        comment: Option<String>,
    },
    Dir {
        name: String,
        path: PathBuf,
        children: Vec<TreeNode>,
    },
}

impl TreeNode {
    pub fn name(&self) -> &str {
        match self {
            TreeNode::File { name, .. } => name,
            TreeNode::Dir { name, .. } => name,
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, TreeNode::Dir { .. })
    }
}

#[derive(Debug, Clone, Default)]
pub struct WalkerConfig {
    pub show_all: bool,
    pub max_depth: Option<usize>,
    pub dirs_only: bool,
    pub extract_comments: bool,
    pub ignore_patterns: Vec<String>,
}

pub struct TreeWalker {
    config: WalkerConfig,
    git_filter: Option<GitFilter>,
}

impl TreeWalker {
    pub fn new(config: WalkerConfig) -> Self {
        Self {
            config,
            git_filter: None,
        }
    }

    pub fn with_git_filter(mut self, filter: GitFilter) -> Self {
        self.git_filter = Some(filter);
        self
    }

    pub fn walk(&self, root: &Path) -> Option<TreeNode> {
        self.walk_dir(root, 0)
    }

    fn walk_dir(&self, path: &Path, depth: usize) -> Option<TreeNode> {
        // Skip symlinks to prevent infinite loops and directory traversal issues
        if path.is_symlink() {
            return None;
        }

        let at_max_depth = self.config.max_depth.map_or(false, |max| depth >= max);

        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        if path.is_file() {
            if self.config.dirs_only {
                return None;
            }
            if !self.should_include(path) {
                return None;
            }
            let comment = if self.config.extract_comments {
                extract_first_comment(path)
            } else {
                None
            };
            return Some(TreeNode::File {
                name,
                path: path.to_path_buf(),
                comment,
            });
        }

        if !path.is_dir() {
            return None;
        }

        // If at max depth, return the directory but don't descend
        if at_max_depth {
            return Some(TreeNode::Dir {
                name,
                path: path.to_path_buf(),
                children: Vec::new(),
            });
        }

        let mut children = Vec::new();
        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return None,
        };

        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        for entry in entries {
            let entry_path = entry.path();

            if self.should_ignore(&entry_path) {
                continue;
            }

            if let Some(node) = self.walk_dir(&entry_path, depth + 1) {
                // Skip empty directories (but only if not in dirs_only mode
                // and not showing a depth-limited directory)
                if let TreeNode::Dir {
                    children: ref c, ..
                } = node
                {
                    // In dirs_only mode, always show directories
                    // Otherwise, skip truly empty directories (those with no tracked files)
                    if c.is_empty() && !self.config.dirs_only && !self.has_tracked_files(&entry_path) {
                        continue;
                    }
                }
                children.push(node);
            }
        }

        Some(TreeNode::Dir {
            name,
            path: path.to_path_buf(),
            children,
        })
    }

    fn has_tracked_files(&self, path: &Path) -> bool {
        if let Some(ref filter) = self.git_filter {
            filter.is_tracked(path)
        } else {
            // Without git filter, assume directory has content
            true
        }
    }

    fn should_include(&self, path: &Path) -> bool {
        if self.config.show_all {
            return true;
        }
        if let Some(ref filter) = self.git_filter {
            return filter.is_tracked(path);
        }
        true
    }

    fn should_ignore(&self, path: &Path) -> bool {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        // Always ignore .git directory
        if name == ".git" {
            return true;
        }

        // Check custom ignore patterns
        for pattern in &self.config.ignore_patterns {
            if name == *pattern || glob_match(pattern, &name) {
                return true;
            }
        }

        false
    }
}

fn glob_match(pattern: &str, name: &str) -> bool {
    // Handle *substring* pattern (contains)
    if pattern.starts_with('*') && pattern.ends_with('*') && pattern.len() > 2 {
        let middle = &pattern[1..pattern.len() - 1];
        return name.contains(middle);
    }
    // Handle *suffix pattern
    if pattern.starts_with('*') && pattern.len() > 1 {
        let suffix = &pattern[1..];
        return name.ends_with(suffix);
    }
    // Handle prefix* pattern
    if pattern.ends_with('*') && pattern.len() > 1 {
        let prefix = &pattern[..pattern.len() - 1];
        return name.starts_with(prefix);
    }
    pattern == name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("*.rs", "lib.rs"));
        assert!(!glob_match("*.rs", "main.py"));
        assert!(glob_match("test*", "test_foo"));
        assert!(!glob_match("test*", "foo_test"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "notexact"));
    }
}
