//! Directory tree walking logic

use std::path::{Path, PathBuf};

use glob::Pattern;
use serde::Serialize;

use crate::comments::extract_first_comment;
use crate::git::{GitFilter, GitignoreFilter};
use crate::metadata::{LineStyle, MetadataBlock, MetadataLine};
use crate::types::extract_type_signatures;

// Re-export for convenience
pub use crate::metadata::MetadataOrder;

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

/// TreeNode for JSON output - still builds full tree in memory.
/// For large repos, use StreamingWalker instead for console output.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TreeNode {
    File {
        name: String,
        path: PathBuf,
        #[serde(skip_serializing_if = "Option::is_none")]
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
    pub extract_types: bool,
    pub ignore_patterns: Vec<String>,
}

pub struct TreeWalker {
    config: WalkerConfig,
    filter: Option<FileFilter>,
}

impl TreeWalker {
    pub fn new(config: WalkerConfig) -> Self {
        Self {
            config,
            filter: None,
        }
    }

    pub fn with_filter(mut self, filter: FileFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Legacy method for backwards compatibility - use with_filter instead.
    pub fn with_git_filter(self, filter: GitFilter) -> Self {
        self.with_filter(FileFilter::GitTracked(filter))
    }

    /// Set gitignore-based filtering (default behavior).
    pub fn with_gitignore_filter(self, filter: GitignoreFilter) -> Self {
        self.with_filter(FileFilter::Gitignore(filter))
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
                    if c.is_empty()
                        && !self.config.dirs_only
                        && !self.has_included_files(&entry_path)
                    {
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

    fn has_included_files(&self, path: &Path) -> bool {
        if let Some(ref filter) = self.filter {
            filter.is_included(path)
        } else {
            // Without filter, assume directory has content
            true
        }
    }

    fn should_include(&self, path: &Path) -> bool {
        if self.config.show_all {
            return true;
        }
        if let Some(ref filter) = self.filter {
            return filter.is_included(path);
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
    Pattern::new(pattern)
        .map(|p| p.matches(name))
        .unwrap_or(false)
}

/// Streaming tree walker that outputs directly without building tree in memory.
/// Uses O(depth) memory instead of O(files) for the tree structure.
pub struct StreamingWalker {
    config: WalkerConfig,
    filter: Option<FileFilter>,
}

/// Callback for streaming output - receives name, metadata, is_dir, is_last, prefix
pub trait StreamingOutput {
    fn output_node(
        &mut self,
        name: &str,
        metadata: Option<MetadataBlock>,
        is_dir: bool,
        is_last: bool,
        prefix: &str,
        is_root: bool,
    ) -> std::io::Result<()>;

    fn finish(&mut self, dir_count: usize, file_count: usize) -> std::io::Result<()>;
}

impl StreamingWalker {
    pub fn new(config: WalkerConfig) -> Self {
        Self {
            config,
            filter: None,
        }
    }

    pub fn with_filter(mut self, filter: FileFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Legacy method for backwards compatibility - use with_filter instead.
    pub fn with_git_filter(self, filter: GitFilter) -> Self {
        self.with_filter(FileFilter::GitTracked(filter))
    }

    /// Set gitignore-based filtering (default behavior).
    pub fn with_gitignore_filter(self, filter: GitignoreFilter) -> Self {
        self.with_filter(FileFilter::Gitignore(filter))
    }

    /// Walk and stream output - returns (dir_count, file_count)
    pub fn walk_streaming<O: StreamingOutput>(
        &self,
        root: &Path,
        output: &mut O,
    ) -> std::io::Result<Option<(usize, usize)>> {
        match self.walk_dir_streaming(root, 0, "", true, output) {
            Ok(Some((d, f))) => {
                output.finish(d, f)?;
                Ok(Some((d, f)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn walk_dir_streaming<O: StreamingOutput>(
        &self,
        path: &Path,
        depth: usize,
        prefix: &str,
        is_root: bool,
        output: &mut O,
    ) -> std::io::Result<Option<(usize, usize)>> {
        // Skip symlinks to prevent infinite loops and directory traversal issues
        if path.is_symlink() {
            return Ok(None);
        }

        let at_max_depth = self.config.max_depth.map_or(false, |max| depth >= max);

        // Files are handled by their parent directory iteration
        if path.is_file() || !path.is_dir() {
            return Ok(None);
        }

        // Collect and sort entries
        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        // Filter entries first to know which ones will be included
        let filtered_entries: Vec<_> = entries
            .into_iter()
            .filter(|entry| {
                let entry_path = entry.path();
                !self.should_ignore(&entry_path)
            })
            .collect();

        // Get the directory name for output
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        // If at max depth, output directory but don't descend
        if at_max_depth && !is_root {
            return Ok(Some((0, 0)));
        }

        // Output this directory (root handled specially)
        if is_root {
            output.output_node(&name, None, true, true, prefix, true)?;
        }

        let mut dir_count = 0usize;
        let mut file_count = 0usize;

        // We need to peek ahead to know which entries will actually produce output
        // to determine is_last correctly
        let mut valid_entries: Vec<(std::fs::DirEntry, bool, Option<MetadataBlock>)> = Vec::new();

        for entry in filtered_entries {
            let entry_path = entry.path();

            if entry_path.is_file() {
                if self.config.dirs_only {
                    continue;
                }
                if !self.should_include(&entry_path) {
                    continue;
                }
                let metadata = self.extract_metadata(&entry_path);
                valid_entries.push((entry, false, metadata));
            } else if entry_path.is_dir() && !entry_path.is_symlink() {
                // Check if this directory has any content (or if we're in dirs_only mode)
                if self.config.dirs_only || self.has_included_files(&entry_path) {
                    valid_entries.push((entry, true, None));
                }
            }
        }

        let total = valid_entries.len();

        for (i, (entry, is_dir, metadata)) in valid_entries.into_iter().enumerate() {
            let entry_path = entry.path();
            let entry_name = entry.file_name().to_string_lossy().to_string();
            let is_last = i == total - 1;

            // Calculate the prefix for this entry's children
            // (based on whether this entry is last among its siblings)
            let new_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            if is_dir {
                output.output_node(&entry_name, None, true, is_last, prefix, false)?;
                dir_count += 1;

                // Recurse
                if let Ok(Some((d, f))) =
                    self.walk_dir_streaming(&entry_path, depth + 1, &new_prefix, false, output)
                {
                    dir_count += d;
                    file_count += f;
                }
            } else {
                output.output_node(&entry_name, metadata, false, is_last, prefix, false)?;
                file_count += 1;
            }
        }

        Ok(Some((dir_count, file_count)))
    }

    fn has_included_files(&self, path: &Path) -> bool {
        if let Some(ref filter) = self.filter {
            filter.is_included(path)
        } else {
            // Without filter, assume directory has content
            true
        }
    }

    fn should_include(&self, path: &Path) -> bool {
        if self.config.show_all {
            return true;
        }
        if let Some(ref filter) = self.filter {
            return filter.is_included(path);
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

    /// Extract metadata (comments and/or type signatures) from a file.
    fn extract_metadata(&self, path: &Path) -> Option<MetadataBlock> {
        let mut block = MetadataBlock::new();

        // Extract comments
        if self.config.extract_comments {
            if let Some(comment) = extract_first_comment(path) {
                block.comment_lines = comment
                    .lines()
                    .map(|line| MetadataLine::new(line.to_string()))
                    .collect();
            }
        }

        // Extract type signatures
        if self.config.extract_types {
            if let Some(signatures) = extract_type_signatures(path) {
                block.type_lines = signatures
                    .into_iter()
                    .map(|(sig, sym)| MetadataLine::with_symbol(sig, LineStyle::TypeSignature, sym))
                    .collect();
            }
        }

        if block.is_empty() { None } else { Some(block) }
    }
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
