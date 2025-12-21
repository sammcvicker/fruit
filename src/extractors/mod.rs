//! Unified extractor module architecture
//!
//! This module provides a common framework for extracting various types of metadata
//! from source files: comments, type signatures, TODO markers, and imports.
//!
//! # Architecture
//!
//! - **Extractor trait**: Common interface for all extractors
//! - **Language-specific implementations**: Organized in the `languages/` subdirectory
//! - **Shared utilities**: Helper functions and types used across extractors
//!
//! # Design Philosophy
//!
//! Each extractor focuses on a specific type of metadata (comments, types, todos, imports)
//! while sharing common language detection and file reading logic. Language-specific
//! extraction logic is organized by language to make it easy to add new languages
//! or extend existing support.

pub mod comments;
pub mod imports;
pub mod languages;
pub mod todos;
pub mod types;

use std::path::Path;

use crate::language::Language;

/// Configuration for extraction features.
///
/// Controls which types of metadata should be extracted from source files.
#[derive(Debug, Clone, Default)]
pub struct ExtractionConfig {
    /// Extract and display comments
    pub show_comments: bool,
    /// Extract and display type signatures
    pub show_types: bool,
    /// Extract and display TODO markers
    pub show_todos: bool,
    /// Extract and display imports
    pub show_imports: bool,
    /// Enable full/verbose mode for extractions
    pub full_mode: bool,
}

impl ExtractionConfig {
    /// Create a new config with all features enabled.
    pub fn all() -> Self {
        Self {
            show_comments: true,
            show_types: true,
            show_todos: true,
            show_imports: true,
            full_mode: false,
        }
    }

    /// Create a new config with only comments enabled.
    pub fn comments_only() -> Self {
        Self {
            show_comments: true,
            ..Default::default()
        }
    }

    /// Create a new config with only types enabled.
    pub fn types_only() -> Self {
        Self {
            show_types: true,
            ..Default::default()
        }
    }

    /// Create a new config with only todos enabled.
    pub fn todos_only() -> Self {
        Self {
            show_todos: true,
            ..Default::default()
        }
    }

    /// Create a new config with only imports enabled.
    pub fn imports_only() -> Self {
        Self {
            show_imports: true,
            ..Default::default()
        }
    }

    /// Check if any extraction feature is enabled.
    pub fn any_enabled(&self) -> bool {
        self.show_comments || self.show_types || self.show_todos || self.show_imports
    }
}

/// Base trait for all metadata extractors.
///
/// Extractors implement this trait to provide a consistent interface
/// for extracting different types of metadata from source files.
pub trait Extractor {
    /// The output type produced by this extractor.
    type Output;

    /// Extract metadata from a file at the given path.
    ///
    /// Returns `None` if:
    /// - The file cannot be read
    /// - The file type is not supported
    /// - No metadata was found
    fn extract(&self, path: &Path) -> Option<Self::Output>;

    /// Check if this extractor supports the given language.
    fn supports_language(&self, language: Language) -> bool;

    /// Get a descriptive name for this extractor (e.g., "comments", "types").
    fn name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_config_all() {
        let config = ExtractionConfig::all();
        assert!(config.show_comments);
        assert!(config.show_types);
        assert!(config.show_todos);
        assert!(config.show_imports);
        assert!(!config.full_mode);
    }

    #[test]
    fn test_extraction_config_comments_only() {
        let config = ExtractionConfig::comments_only();
        assert!(config.show_comments);
        assert!(!config.show_types);
        assert!(!config.show_todos);
        assert!(!config.show_imports);
    }

    #[test]
    fn test_extraction_config_any_enabled() {
        let config = ExtractionConfig::default();
        assert!(!config.any_enabled());

        let config = ExtractionConfig::comments_only();
        assert!(config.any_enabled());
    }
}
