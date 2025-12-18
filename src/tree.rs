//! Directory tree walking logic

use std::path::{Path, PathBuf};

use glob::Pattern;
use rayon::prelude::*;
use serde::Serialize;

use crate::comments::extract_first_comment;
use crate::git::{GitFilter, GitignoreFilter};
use crate::metadata::{LineStyle, MetadataBlock, MetadataLine};
use crate::todos::extract_todos;
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

// ============================================================================
// Shared filtering functions used by both TreeWalker and StreamingWalker
// ============================================================================

/// Check if a directory has any included files (used for pruning empty directories).
fn has_included_files(path: &Path, filter: &Option<FileFilter>) -> bool {
    if let Some(f) = filter {
        f.is_included(path)
    } else {
        // Without filter, assume directory has content
        true
    }
}

/// Check if a path should be included based on filter and show_all flag.
fn should_include_path(path: &Path, config: &WalkerConfig, filter: &Option<FileFilter>) -> bool {
    if config.show_all {
        return true;
    }
    if let Some(f) = filter {
        return f.is_included(path);
    }
    true
}

/// Check if a path should be ignored based on name and ignore patterns.
fn should_ignore_path(path: &Path, ignore_patterns: &[String]) -> bool {
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

/// Serializable TODO item for JSON output.
#[derive(Debug, Clone, Serialize)]
pub struct JsonTodoItem {
    #[serde(rename = "type")]
    pub marker_type: String,
    pub text: String,
    pub line: usize,
}

impl From<&crate::todos::TodoItem> for JsonTodoItem {
    fn from(item: &crate::todos::TodoItem) -> Self {
        Self {
            marker_type: item.marker_type.clone(),
            text: item.text.clone(),
            line: item.line,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        types: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        todos: Option<Vec<JsonTodoItem>>,
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
    pub extract_todos: bool,
    pub ignore_patterns: Vec<String>,
    /// Number of parallel workers for metadata extraction.
    /// 0 = auto-detect (use all available cores)
    /// 1 = sequential (no parallelism)
    /// N = use N worker threads
    pub parallel_workers: usize,
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

        let at_max_depth = self.config.max_depth.is_some_and(|max| depth >= max);

        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        if path.is_file() {
            if self.config.dirs_only {
                return None;
            }
            if !should_include_path(path, &self.config, &self.filter) {
                return None;
            }
            let comment = if self.config.extract_comments {
                extract_first_comment(path)
            } else {
                None
            };
            let types = if self.config.extract_types {
                extract_type_signatures(path)
                    .map(|sigs| sigs.into_iter().map(|(sig, _sym, _indent)| sig).collect())
            } else {
                None
            };
            let todos = if self.config.extract_todos {
                extract_todos(path).map(|items| items.iter().map(JsonTodoItem::from).collect())
            } else {
                None
            };
            return Some(TreeNode::File {
                name,
                path: path.to_path_buf(),
                comment,
                types,
                todos,
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
        entries.sort_by_key(|a| a.file_name());

        for entry in entries {
            let entry_path = entry.path();

            if should_ignore_path(&entry_path, &self.config.ignore_patterns) {
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
                        && !has_included_files(&entry_path, &self.filter)
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
}

fn glob_match(pattern: &str, name: &str) -> bool {
    Pattern::new(pattern)
        .map(|p| p.matches(name))
        .unwrap_or(false)
}

/// Entry collected during tree traversal for parallel metadata extraction.
#[derive(Debug)]
struct CollectedEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    is_last: bool,
    prefix: String,
    is_root: bool,
}

/// Streaming tree walker that outputs directly without building tree in memory.
/// Uses O(depth) memory instead of O(files) for the tree structure.
/// Supports parallel metadata extraction when parallel_workers != 1.
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
        // Use parallel extraction if workers != 1
        let use_parallel = self.config.parallel_workers != 1
            && (self.config.extract_comments || self.config.extract_types);

        if use_parallel {
            self.walk_streaming_parallel(root, output)
        } else {
            self.walk_streaming_sequential(root, output)
        }
    }

    /// Sequential streaming walk - original implementation for -j1 or no metadata extraction.
    fn walk_streaming_sequential<O: StreamingOutput>(
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

    /// Parallel streaming walk - collects files first, extracts metadata in parallel.
    fn walk_streaming_parallel<O: StreamingOutput>(
        &self,
        root: &Path,
        output: &mut O,
    ) -> std::io::Result<Option<(usize, usize)>> {
        // Phase 1: Collect all entries in tree order
        let mut entries = Vec::new();
        if self
            .collect_entries(root, 0, "", true, &mut entries)
            .is_none()
        {
            return Ok(None);
        }

        // Phase 2: Extract metadata in parallel for all files
        // Configure rayon thread pool if specific worker count requested
        let file_indices: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| if !e.is_dir { Some(i) } else { None })
            .collect();

        // Extract metadata in parallel
        // Note: We use a standalone function to avoid capturing &self (which contains
        // non-Sync FileFilter/GitFilter) in the parallel closure.
        let extract_comments = self.config.extract_comments;
        let extract_types = self.config.extract_types;
        let extract_todo_markers = self.config.extract_todos;

        let metadata_results: Vec<(usize, Option<MetadataBlock>)> =
            if self.config.parallel_workers == 0 {
                // Auto-detect: use rayon's default thread pool
                file_indices
                    .par_iter()
                    .map(|&i| {
                        let path = &entries[i].path;
                        let metadata = extract_metadata_from_path(
                            path,
                            extract_comments,
                            extract_types,
                            extract_todo_markers,
                        );
                        (i, metadata)
                    })
                    .collect()
            } else {
                // Use custom thread pool with specified worker count
                match rayon::ThreadPoolBuilder::new()
                    .num_threads(self.config.parallel_workers)
                    .build()
                {
                    Ok(pool) => pool.install(|| {
                        file_indices
                            .par_iter()
                            .map(|&i| {
                                let path = &entries[i].path;
                                let metadata = extract_metadata_from_path(
                                    path,
                                    extract_comments,
                                    extract_types,
                                    extract_todo_markers,
                                );
                                (i, metadata)
                            })
                            .collect()
                    }),
                    Err(_) => {
                        // Fall back to rayon's global pool if custom pool creation fails
                        file_indices
                            .par_iter()
                            .map(|&i| {
                                let path = &entries[i].path;
                                let metadata = extract_metadata_from_path(
                                    path,
                                    extract_comments,
                                    extract_types,
                                    extract_todo_markers,
                                );
                                (i, metadata)
                            })
                            .collect()
                    }
                }
            };

        // Build a map of index -> metadata for quick lookup
        let mut metadata_map: std::collections::HashMap<usize, Option<MetadataBlock>> =
            metadata_results.into_iter().collect();

        // Phase 3: Output entries in tree order
        let mut dir_count = 0usize;
        let mut file_count = 0usize;

        for (i, entry) in entries.iter().enumerate() {
            let metadata = if entry.is_dir {
                None
            } else {
                metadata_map.remove(&i).flatten()
            };

            output.output_node(
                &entry.name,
                metadata,
                entry.is_dir,
                entry.is_last,
                &entry.prefix,
                entry.is_root,
            )?;

            if entry.is_dir && !entry.is_root {
                dir_count += 1;
            } else if !entry.is_dir {
                file_count += 1;
            }
        }

        output.finish(dir_count, file_count)?;
        Ok(Some((dir_count, file_count)))
    }

    /// Collected entry for parallel processing.
    fn collect_entries(
        &self,
        path: &Path,
        depth: usize,
        prefix: &str,
        is_root: bool,
        entries: &mut Vec<CollectedEntry>,
    ) -> Option<()> {
        // Skip symlinks to prevent infinite loops
        if path.is_symlink() {
            return None;
        }

        let at_max_depth = self.config.max_depth.is_some_and(|max| depth >= max);

        // Files are handled by their parent directory iteration
        if path.is_file() || !path.is_dir() {
            return None;
        }

        // Collect and sort directory entries
        let dir_entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return None,
        };

        let mut dir_entries: Vec<_> = dir_entries.filter_map(|e| e.ok()).collect();
        dir_entries.sort_by_key(|a| a.file_name());

        // Filter entries
        let filtered_entries: Vec<_> = dir_entries
            .into_iter()
            .filter(|entry| {
                let entry_path = entry.path();
                !should_ignore_path(&entry_path, &self.config.ignore_patterns)
            })
            .collect();

        // Get directory name
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        // Handle max depth
        if at_max_depth && !is_root {
            return Some(());
        }

        // Add root directory entry
        if is_root {
            entries.push(CollectedEntry {
                name,
                path: path.to_path_buf(),
                is_dir: true,
                is_last: true,
                prefix: prefix.to_string(),
                is_root: true,
            });
        }

        // Build list of valid entries (files and non-empty directories)
        let mut valid_entries: Vec<(std::fs::DirEntry, bool)> = Vec::new();

        for entry in filtered_entries {
            let entry_path = entry.path();

            if entry_path.is_file() {
                if self.config.dirs_only {
                    continue;
                }
                if !should_include_path(&entry_path, &self.config, &self.filter) {
                    continue;
                }
                valid_entries.push((entry, false)); // false = is file
            } else if entry_path.is_dir()
                && !entry_path.is_symlink()
                && (self.config.dirs_only || has_included_files(&entry_path, &self.filter))
            {
                valid_entries.push((entry, true)); // true = is directory
            }
        }

        let total = valid_entries.len();

        for (i, (entry, is_dir)) in valid_entries.into_iter().enumerate() {
            let entry_path = entry.path();
            let entry_name = entry.file_name().to_string_lossy().to_string();
            let is_last = i == total - 1;

            let new_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            if is_dir {
                // Add directory entry
                entries.push(CollectedEntry {
                    name: entry_name,
                    path: entry_path.clone(),
                    is_dir: true,
                    is_last,
                    prefix: prefix.to_string(),
                    is_root: false,
                });

                // Recurse into directory
                self.collect_entries(&entry_path, depth + 1, &new_prefix, false, entries);
            } else {
                // Add file entry
                entries.push(CollectedEntry {
                    name: entry_name,
                    path: entry_path,
                    is_dir: false,
                    is_last,
                    prefix: prefix.to_string(),
                    is_root: false,
                });
            }
        }

        Some(())
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

        let at_max_depth = self.config.max_depth.is_some_and(|max| depth >= max);

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
        entries.sort_by_key(|a| a.file_name());

        // Filter entries first to know which ones will be included
        let filtered_entries: Vec<_> = entries
            .into_iter()
            .filter(|entry| {
                let entry_path = entry.path();
                !should_ignore_path(&entry_path, &self.config.ignore_patterns)
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
                if !should_include_path(&entry_path, &self.config, &self.filter) {
                    continue;
                }
                let metadata = self.extract_metadata(&entry_path);
                valid_entries.push((entry, false, metadata));
            } else if entry_path.is_dir() && !entry_path.is_symlink() {
                // Check if this directory has any content (or if we're in dirs_only mode)
                if self.config.dirs_only || has_included_files(&entry_path, &self.filter) {
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

    /// Extract metadata (comments and/or type signatures and/or TODOs) from a file.
    fn extract_metadata(&self, path: &Path) -> Option<MetadataBlock> {
        extract_metadata_from_path(
            path,
            self.config.extract_comments,
            self.config.extract_types,
            self.config.extract_todos,
        )
    }
}

/// Extract metadata from a file path - standalone function for parallel execution.
/// This is a free function to avoid capturing &StreamingWalker (which contains
/// non-thread-safe FileFilter) in parallel closures.
fn extract_metadata_from_path(
    path: &Path,
    extract_comments: bool,
    extract_types: bool,
    extract_todo_markers: bool,
) -> Option<MetadataBlock> {
    let mut block = MetadataBlock::new();

    // Extract comments
    if extract_comments {
        if let Some(comment) = extract_first_comment(path) {
            block.comment_lines = comment
                .lines()
                .map(|line| MetadataLine::new(line.to_string()))
                .collect();
        }
    }

    // Extract type signatures
    if extract_types {
        if let Some(signatures) = extract_type_signatures(path) {
            block.type_lines = signatures
                .into_iter()
                .map(|(sig, sym, indent)| {
                    MetadataLine::with_symbol(sig, LineStyle::TypeSignature, sym, indent)
                })
                .collect();
        }
    }

    // Extract TODO/FIXME markers
    if extract_todo_markers {
        if let Some(todos) = extract_todos(path) {
            block.todo_lines = todos
                .iter()
                .map(|todo| {
                    let content =
                        format!("{}: {} (line {})", todo.marker_type, todo.text, todo.line);
                    MetadataLine::with_style(content, LineStyle::Todo)
                })
                .collect();
        }
    }

    if block.is_empty() { None } else { Some(block) }
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
