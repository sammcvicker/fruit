//! TreeWalker - builds full tree in memory for JSON output

use std::path::Path;

use crate::comments::extract_first_comment;
use crate::git::GitignoreFilter;
use crate::imports::extract_imports;
use crate::todos::extract_todos;
use crate::types::extract_type_signatures;

use super::config::WalkerConfig;
use super::filter::FileFilter;
use super::json_types::{JsonTodoItem, JsonTypeItem, TreeNode};
use super::utils::{get_file_size, has_included_files, should_ignore_path, should_include_path};

/// Tree walker that builds the full tree in memory.
/// Required for JSON output serialization.
/// For large repos with console output, use StreamingWalker instead.
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

    /// Set gitignore-based filtering (default behavior).
    pub fn with_gitignore_filter(self, filter: GitignoreFilter) -> Self {
        self.with_filter(FileFilter::new(filter))
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
                extract_type_signatures(path).map(|sigs| {
                    sigs.into_iter()
                        .map(|ts| JsonTypeItem::new(ts.signature, ts.symbol_name, ts.indent))
                        .collect()
                })
            } else {
                None
            };
            let todos = if self.config.extract_todos {
                extract_todos(path).map(|items| items.iter().map(JsonTodoItem::from).collect())
            } else {
                None
            };
            // If todos_only is enabled, skip files without TODOs
            if self.config.todos_only
                && todos
                    .as_ref()
                    .is_none_or(|t: &Vec<JsonTodoItem>| t.is_empty())
            {
                return None;
            }
            let imports = if self.config.extract_imports {
                extract_imports(path)
            } else {
                None
            };
            let (size_bytes, size_human) = if self.config.show_size {
                get_file_size(path)
            } else {
                (None, None)
            };
            return Some(TreeNode::File {
                name,
                path: path.to_path_buf(),
                comments: comment,
                types,
                todos,
                imports,
                size_bytes,
                size_human,
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
