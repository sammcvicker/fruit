//! StreamingWalker - streams output without building full tree in memory

use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::comments::extract_first_comment;
use crate::git::{GitFilter, GitignoreFilter};
use crate::imports::extract_imports;
use crate::metadata::{LineStyle, MetadataBlock, MetadataLine};
use crate::todos::extract_todos;
use crate::types::extract_type_signatures;

use super::config::WalkerConfig;
use super::filter::FileFilter;
use super::utils::{has_included_files, should_ignore_path, should_include_path};

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

/// Callback for streaming output - receives node information for display.
pub trait StreamingOutput {
    #[allow(clippy::too_many_arguments)]
    fn output_node(
        &mut self,
        name: &str,
        metadata: Option<MetadataBlock>,
        is_dir: bool,
        is_last: bool,
        prefix: &str,
        is_root: bool,
        size: Option<u64>,
    ) -> std::io::Result<()>;

    fn finish(&mut self, dir_count: usize, file_count: usize) -> std::io::Result<()>;
}

/// Streaming tree walker that outputs directly without building tree in memory.
/// Uses O(depth) memory instead of O(files) for the tree structure.
/// Supports parallel metadata extraction when parallel_workers != 1.
pub struct StreamingWalker {
    config: WalkerConfig,
    filter: Option<FileFilter>,
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
        let extract_import_statements = self.config.extract_imports;

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
                            extract_import_statements,
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
                                    extract_import_statements,
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
                                    extract_import_statements,
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

        // If todos_only is enabled, we need to filter files without TODOs
        // and track which indices to skip
        let skip_indices: std::collections::HashSet<usize> = if self.config.todos_only {
            entries
                .iter()
                .enumerate()
                .filter_map(|(i, entry)| {
                    if entry.is_dir {
                        None // Don't skip directories
                    } else {
                        // Check if this file has TODOs
                        let has_todos = metadata_map
                            .get(&i)
                            .and_then(|opt| opt.as_ref())
                            .is_some_and(|meta| !meta.todo_lines.is_empty());
                        if has_todos {
                            None // Don't skip
                        } else {
                            Some(i) // Skip this file
                        }
                    }
                })
                .collect()
        } else {
            std::collections::HashSet::new()
        };

        // Phase 3: Output entries in tree order
        let mut dir_count = 0usize;
        let mut file_count = 0usize;

        // We need to track is_last correctly after filtering
        // Group entries by parent prefix and recalculate is_last
        let filtered_entries: Vec<_> = entries
            .iter()
            .enumerate()
            .filter(|(i, _)| !skip_indices.contains(i))
            .collect();

        // Calculate which filtered entries are last among their siblings
        let is_last_map: std::collections::HashMap<usize, bool> = {
            let mut map = std::collections::HashMap::new();
            let mut prefix_counts: std::collections::HashMap<&str, Vec<usize>> =
                std::collections::HashMap::new();

            for &(i, entry) in &filtered_entries {
                prefix_counts.entry(&entry.prefix).or_default().push(i);
            }

            for indices in prefix_counts.values() {
                for (pos, &idx) in indices.iter().enumerate() {
                    map.insert(idx, pos == indices.len() - 1);
                }
            }

            map
        };

        for (i, entry) in filtered_entries {
            let metadata = if entry.is_dir {
                None
            } else {
                metadata_map.remove(&i).flatten()
            };

            // Get file size if enabled and this is a file
            let size = if !entry.is_dir && self.config.show_size {
                entry.path.metadata().ok().map(|m| m.len())
            } else {
                None
            };

            // Use recalculated is_last, or original if not in map (shouldn't happen)
            let is_last = is_last_map.get(&i).copied().unwrap_or(entry.is_last);

            output.output_node(
                &entry.name,
                metadata,
                entry.is_dir,
                is_last,
                &entry.prefix,
                entry.is_root,
                size,
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
            output.output_node(&name, None, true, true, prefix, true, None)?;
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
                // If todos_only is enabled, skip files without TODOs
                if self.config.todos_only {
                    if let Some(ref meta) = metadata {
                        if meta.todo_lines.is_empty() {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
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
                output.output_node(&entry_name, None, true, is_last, prefix, false, None)?;
                dir_count += 1;

                // Recurse
                if let Ok(Some((d, f))) =
                    self.walk_dir_streaming(&entry_path, depth + 1, &new_prefix, false, output)
                {
                    dir_count += d;
                    file_count += f;
                }
            } else {
                // Get file size if enabled
                let size = if self.config.show_size {
                    entry_path.metadata().ok().map(|m| m.len())
                } else {
                    None
                };
                output.output_node(&entry_name, metadata, false, is_last, prefix, false, size)?;
                file_count += 1;
            }
        }

        Ok(Some((dir_count, file_count)))
    }

    /// Extract metadata (comments and/or type signatures and/or TODOs and/or imports) from a file.
    fn extract_metadata(&self, path: &Path) -> Option<MetadataBlock> {
        extract_metadata_from_path(
            path,
            self.config.extract_comments,
            self.config.extract_types,
            self.config.extract_todos,
            self.config.extract_imports,
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
    extract_import_statements: bool,
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

    // Extract imports
    if extract_import_statements {
        if let Some(imports) = extract_imports(path) {
            // Format imports as a summary line
            let summary = imports.summary();
            if !summary.is_empty() {
                block.import_lines = vec![MetadataLine::with_style(
                    format!("imports: {}", summary),
                    LineStyle::Import,
                )];
            }
        }
    }

    if block.is_empty() { None } else { Some(block) }
}
