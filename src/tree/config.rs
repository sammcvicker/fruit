//! Configuration types for tree walkers

use std::time::SystemTime;

/// Configuration for tree walking behavior.
#[derive(Debug, Clone)]
pub struct WalkerConfig {
    pub show_all: bool,
    pub max_depth: Option<usize>,
    pub dirs_only: bool,
    pub extract_comments: bool,
    pub extract_types: bool,
    pub extract_todos: bool,
    /// Only show files that contain TODO/FIXME markers (requires extract_todos = true)
    pub todos_only: bool,
    pub extract_imports: bool,
    pub show_size: bool,
    pub ignore_patterns: Vec<String>,
    /// Number of parallel workers for metadata extraction.
    /// 0 = auto-detect (use all available cores)
    /// 1 = sequential (no parallelism)
    /// N = use N worker threads
    pub parallel_workers: usize,
    /// Only include files modified after this time
    pub newer_than: Option<SystemTime>,
    /// Only include files modified before this time
    pub older_than: Option<SystemTime>,
}

impl Default for WalkerConfig {
    fn default() -> Self {
        Self {
            show_all: false,
            max_depth: None,
            dirs_only: false,
            extract_comments: true, // default behavior: show comments
            extract_types: false,
            extract_todos: false,
            todos_only: false,
            extract_imports: false,
            show_size: false,
            ignore_patterns: Vec::new(),
            parallel_workers: 0,
            newer_than: None,
            older_than: None,
        }
    }
}
