//! Generic metadata block abstraction for extensible file info display
//!
//! This module provides a unified pattern for displaying various types of information
//! beneath file paths in the tree output. It enables composable metadata display from
//! multiple sources (comments, type signatures, code structure, etc.).

use std::path::Path;
use termcolor::Color;

/// Style for how a metadata line should be displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineStyle {
    /// Standard comment display (dim gray)
    #[default]
    Comment,
    /// Type signature display (for future use)
    TypeSignature,
    /// Class/struct name display (for future use)
    ClassName,
    /// Method/function name display (for future use)
    MethodName,
    /// Docstring display (for future use)
    Docstring,
}

impl LineStyle {
    /// Get the color for this line style.
    pub fn color(&self) -> Color {
        match self {
            LineStyle::Comment => Color::Black,
            LineStyle::TypeSignature => Color::Cyan,
            LineStyle::ClassName => Color::Yellow,
            LineStyle::MethodName => Color::Green,
            LineStyle::Docstring => Color::Black,
        }
    }

    /// Whether this style should use intense/bright colors.
    pub fn is_intense(&self) -> bool {
        matches!(self, LineStyle::Comment | LineStyle::Docstring)
    }
}

/// A single line of metadata to display.
#[derive(Debug, Clone)]
pub struct MetadataLine {
    /// The content of this line
    pub content: String,
    /// Style for coloring
    pub style: LineStyle,
}

impl MetadataLine {
    /// Create a new metadata line with the given content and default comment style.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: LineStyle::Comment,
        }
    }

    /// Create a new metadata line with a specific style.
    pub fn with_style(content: impl Into<String>, style: LineStyle) -> Self {
        Self {
            content: content.into(),
            style,
        }
    }
}

/// A block of metadata lines to display beneath a file.
#[derive(Debug, Clone)]
pub struct MetadataBlock {
    /// The lines of metadata
    pub lines: Vec<MetadataLine>,
    /// Name of the extractor that produced this block (for debugging/display)
    pub source: &'static str,
}

impl MetadataBlock {
    /// Create a new metadata block with the given lines.
    pub fn new(source: &'static str, lines: Vec<MetadataLine>) -> Self {
        Self { lines, source }
    }

    /// Create a metadata block from plain text (one line per newline).
    pub fn from_text(source: &'static str, text: &str) -> Self {
        let lines = text
            .lines()
            .map(|line| MetadataLine::new(line.to_string()))
            .collect();
        Self { lines, source }
    }

    /// Create a metadata block from plain text with a specific style.
    pub fn from_text_styled(source: &'static str, text: &str, style: LineStyle) -> Self {
        let lines = text
            .lines()
            .map(|line| MetadataLine::with_style(line.to_string(), style))
            .collect();
        Self { lines, source }
    }

    /// Check if this block has any content.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// Trait for extracting metadata from files.
///
/// Implementors provide a way to extract structured metadata from file content
/// that can be displayed in the tree output.
pub trait MetadataExtractor: Send + Sync {
    /// Extract metadata from a file at the given path.
    ///
    /// Returns `None` if no metadata could be extracted (unsupported file type,
    /// no relevant content, etc.).
    fn extract(&self, path: &Path) -> Option<MetadataBlock>;

    /// The name of this extractor (e.g., "comments", "types", "structure").
    fn name(&self) -> &'static str;
}

/// Built-in comment extractor that wraps the existing comment extraction logic.
pub struct CommentExtractor;

impl MetadataExtractor for CommentExtractor {
    fn extract(&self, path: &Path) -> Option<MetadataBlock> {
        crate::comments::extract_first_comment(path)
            .map(|text| MetadataBlock::from_text("comments", &text))
    }

    fn name(&self) -> &'static str {
        "comments"
    }
}

/// Configuration for which metadata extractors to use.
#[derive(Debug, Clone, Default)]
pub struct MetadataConfig {
    /// Show comments (default: true unless --no-comments)
    pub comments: bool,
    /// Show type signatures (--types / -t)
    pub types: bool,
    /// Show full metadata blocks (multi-line) vs first line only
    pub full: bool,
    /// Optional prefix to add before each metadata line (e.g., "# ")
    pub prefix: Option<String>,
}

impl MetadataConfig {
    /// Create a config that shows comments only (default behavior).
    pub fn comments_only(full: bool) -> Self {
        Self {
            comments: true,
            types: false,
            full,
            prefix: None,
        }
    }

    /// Create a config that shows type signatures only.
    pub fn types_only(full: bool) -> Self {
        Self {
            comments: false,
            types: true,
            full,
            prefix: None,
        }
    }

    /// Create a config that shows both comments and types.
    pub fn all(full: bool) -> Self {
        Self {
            comments: true,
            types: true,
            full,
            prefix: None,
        }
    }

    /// Create a config that disables all metadata.
    pub fn none() -> Self {
        Self {
            comments: false,
            types: false,
            full: false,
            prefix: None,
        }
    }

    /// Set a prefix for metadata lines (e.g., "# " or "// ").
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Get the prefix string, or empty string if none set.
    pub fn prefix_str(&self) -> &str {
        self.prefix.as_deref().unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_line_creation() {
        let line = MetadataLine::new("test content");
        assert_eq!(line.content, "test content");
        assert_eq!(line.style, LineStyle::Comment);
    }

    #[test]
    fn test_metadata_line_with_style() {
        let line = MetadataLine::with_style("fn foo()", LineStyle::TypeSignature);
        assert_eq!(line.content, "fn foo()");
        assert_eq!(line.style, LineStyle::TypeSignature);
    }

    #[test]
    fn test_metadata_block_from_text() {
        let block = MetadataBlock::from_text("test", "line 1\nline 2\nline 3");
        assert_eq!(block.lines.len(), 3);
        assert_eq!(block.lines[0].content, "line 1");
        assert_eq!(block.lines[1].content, "line 2");
        assert_eq!(block.lines[2].content, "line 3");
        assert_eq!(block.source, "test");
    }

    #[test]
    fn test_metadata_block_is_empty() {
        let empty = MetadataBlock::new("test", vec![]);
        assert!(empty.is_empty());

        let non_empty = MetadataBlock::from_text("test", "content");
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_line_style_colors() {
        assert_eq!(LineStyle::Comment.color(), Color::Black);
        assert!(LineStyle::Comment.is_intense());
        assert_eq!(LineStyle::TypeSignature.color(), Color::Cyan);
        assert!(!LineStyle::TypeSignature.is_intense());
    }

    #[test]
    fn test_metadata_config_defaults() {
        let config = MetadataConfig::default();
        assert!(!config.comments);
        assert!(!config.types);
        assert!(!config.full);
        assert!(config.prefix.is_none());

        let comments = MetadataConfig::comments_only(true);
        assert!(comments.comments);
        assert!(!comments.types);
        assert!(comments.full);
        assert!(comments.prefix.is_none());

        let types = MetadataConfig::types_only(true);
        assert!(!types.comments);
        assert!(types.types);
        assert!(types.full);

        let all = MetadataConfig::all(false);
        assert!(all.comments);
        assert!(all.types);
        assert!(!all.full);

        let none = MetadataConfig::none();
        assert!(!none.comments);
        assert!(!none.types);
        assert!(!none.full);
    }

    #[test]
    fn test_metadata_config_with_prefix() {
        let config = MetadataConfig::comments_only(false).with_prefix("# ");
        assert_eq!(config.prefix_str(), "# ");

        let no_prefix = MetadataConfig::comments_only(false);
        assert_eq!(no_prefix.prefix_str(), "");
    }
}
