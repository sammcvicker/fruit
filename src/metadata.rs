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
    /// Type signature display
    TypeSignature,
    /// TODO/FIXME marker display
    Todo,
    /// Import/dependency display
    Import,
}

impl LineStyle {
    /// Get the color for this line style.
    pub fn color(&self) -> Color {
        match self {
            LineStyle::Comment => Color::Black,
            LineStyle::TypeSignature => Color::Cyan,
            LineStyle::Todo => Color::Yellow,
            LineStyle::Import => Color::Magenta,
        }
    }

    /// Whether this style should use intense/bright colors.
    pub fn is_intense(&self) -> bool {
        matches!(self, LineStyle::Comment)
    }
}

/// A single line of metadata to display.
#[derive(Debug, Clone)]
pub struct MetadataLine {
    /// The content of this line
    pub content: String,
    /// Style for coloring
    pub style: LineStyle,
    /// Symbol name to highlight (for type signatures)
    pub symbol_name: Option<String>,
    /// Indentation level (number of spaces) for hierarchy display
    pub indent: usize,
}

impl MetadataLine {
    /// Create a new metadata line with the given content and default comment style.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: LineStyle::Comment,
            symbol_name: None,
            indent: 0,
        }
    }

    /// Create a new metadata line with a specific style.
    pub fn with_style(content: impl Into<String>, style: LineStyle) -> Self {
        Self {
            content: content.into(),
            style,
            symbol_name: None,
            indent: 0,
        }
    }

    /// Create a new metadata line with a style, highlighted symbol name, and indentation.
    pub fn with_symbol(
        content: impl Into<String>,
        style: LineStyle,
        symbol_name: impl Into<String>,
        indent: usize,
    ) -> Self {
        Self {
            content: content.into(),
            style,
            symbol_name: Some(symbol_name.into()),
            indent,
        }
    }
}

/// A block of metadata lines to display beneath a file.
#[derive(Debug, Clone, Default)]
pub struct MetadataBlock {
    /// Comment lines (from file header comments/docstrings)
    pub comment_lines: Vec<MetadataLine>,
    /// Type signature lines (from exported functions, classes, etc.)
    pub type_lines: Vec<MetadataLine>,
    /// TODO/FIXME marker lines
    pub todo_lines: Vec<MetadataLine>,
    /// Import/dependency lines
    pub import_lines: Vec<MetadataLine>,
}

impl MetadataBlock {
    /// Create a new empty metadata block.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a metadata block with only comment lines.
    pub fn from_comments(text: &str) -> Self {
        let comment_lines = text
            .lines()
            .map(|line| MetadataLine::new(line))
            .collect();
        Self {
            comment_lines,
            type_lines: Vec::new(),
            todo_lines: Vec::new(),
            import_lines: Vec::new(),
        }
    }

    /// Create a metadata block with only type lines from type signatures.
    pub fn from_types(signatures: Vec<crate::extractors::types::TypeSignature>) -> Self {
        let type_lines = signatures
            .into_iter()
            .map(|ts| {
                MetadataLine::with_symbol(
                    ts.signature,
                    LineStyle::TypeSignature,
                    ts.symbol_name,
                    ts.indent,
                )
            })
            .collect();
        Self {
            comment_lines: Vec::new(),
            type_lines,
            todo_lines: Vec::new(),
            import_lines: Vec::new(),
        }
    }

    /// Create a metadata block with only TODO lines.
    ///
    /// # Output Format Note
    ///
    /// Console/markdown output formats TODOs as a combined string for human readability:
    /// "TODO: Fix this bug (line 42)"
    ///
    /// JSON output keeps fields separate for machine parsing (see `JsonTodoItem`):
    /// `{"type": "TODO", "text": "Fix this bug", "line": 42}`
    pub fn from_todos(todos: &[crate::todos::TodoItem]) -> Self {
        let todo_lines = todos
            .iter()
            .map(|todo| {
                let content = format!("{}: {} (line {})", todo.marker_type, todo.text, todo.line);
                MetadataLine::with_style(content, LineStyle::Todo)
            })
            .collect();
        Self {
            comment_lines: Vec::new(),
            type_lines: Vec::new(),
            todo_lines,
            import_lines: Vec::new(),
        }
    }

    /// Check if this block has any content.
    pub fn is_empty(&self) -> bool {
        self.comment_lines.is_empty()
            && self.type_lines.is_empty()
            && self.todo_lines.is_empty()
            && self.import_lines.is_empty()
    }

    /// Check if only comments are present (no types, todos, or imports).
    pub fn has_only_comments(&self) -> bool {
        !self.comment_lines.is_empty()
            && self.type_lines.is_empty()
            && self.todo_lines.is_empty()
            && self.import_lines.is_empty()
    }

    /// Check if only types are present (no comments, todos, or imports).
    pub fn has_only_types(&self) -> bool {
        self.comment_lines.is_empty()
            && !self.type_lines.is_empty()
            && self.todo_lines.is_empty()
            && self.import_lines.is_empty()
    }

    /// Check if only imports are present (no comments, types, or todos).
    pub fn has_only_imports(&self) -> bool {
        self.comment_lines.is_empty()
            && self.type_lines.is_empty()
            && self.todo_lines.is_empty()
            && !self.import_lines.is_empty()
    }

    /// Check if only todos are present (no comments, types, or imports).
    pub fn has_only_todos(&self) -> bool {
        self.comment_lines.is_empty()
            && self.type_lines.is_empty()
            && !self.todo_lines.is_empty()
            && self.import_lines.is_empty()
    }

    /// Check if both comments and types are present.
    pub fn has_both(&self) -> bool {
        !self.comment_lines.is_empty() && !self.type_lines.is_empty()
    }

    /// Check if todos are present.
    pub fn has_todos(&self) -> bool {
        !self.todo_lines.is_empty()
    }

    /// Check if imports are present.
    pub fn has_imports(&self) -> bool {
        !self.import_lines.is_empty()
    }

    /// Get lines in the specified order, with an empty line between groups if both exist.
    /// Order: comments/types (per order), then imports, then TODOs.
    pub fn lines_in_order(&self, order: MetadataOrder) -> Vec<MetadataLine> {
        let mut result = Vec::new();

        let (first, second) = match order {
            MetadataOrder::CommentsFirst => (&self.comment_lines, &self.type_lines),
            MetadataOrder::TypesFirst => (&self.type_lines, &self.comment_lines),
        };

        result.extend(first.iter().cloned());

        // Add separator if both groups have content
        if !first.is_empty() && !second.is_empty() {
            result.push(MetadataLine::new(String::new())); // empty line separator
        }

        result.extend(second.iter().cloned());

        // Add imports with separator
        if !self.import_lines.is_empty() && !result.is_empty() {
            result.push(MetadataLine::new(String::new())); // empty line separator
        }
        result.extend(self.import_lines.iter().cloned());

        // Add TODOs at the end with separator
        if !self.todo_lines.is_empty() && !result.is_empty() {
            result.push(MetadataLine::new(String::new())); // empty line separator
        }
        result.extend(self.todo_lines.iter().cloned());

        result
    }

    /// Get the first line of metadata (for inline display).
    /// Returns the first line from the first non-empty group based on order.
    pub fn first_line(&self, order: MetadataOrder) -> Option<&MetadataLine> {
        let (first, second) = match order {
            MetadataOrder::CommentsFirst => (&self.comment_lines, &self.type_lines),
            MetadataOrder::TypesFirst => (&self.type_lines, &self.comment_lines),
        };

        first
            .first()
            .or_else(|| second.first())
            .or_else(|| self.todo_lines.first())
    }

    /// Check if the first metadata section (based on order) has only one line.
    /// This is used to determine if it should be displayed inline.
    pub fn first_section_is_single_line(&self, order: MetadataOrder) -> bool {
        let first = match order {
            MetadataOrder::CommentsFirst => &self.comment_lines,
            MetadataOrder::TypesFirst => &self.type_lines,
        };

        // If the first section is empty, check the second
        if first.is_empty() {
            let second = match order {
                MetadataOrder::CommentsFirst => &self.type_lines,
                MetadataOrder::TypesFirst => &self.comment_lines,
            };
            if second.is_empty() {
                // Only todos present
                return self.todo_lines.len() == 1;
            }
            return second.len() == 1;
        }

        first.len() == 1
    }

    /// Total number of lines (not counting separator).
    pub fn total_lines(&self) -> usize {
        self.comment_lines.len()
            + self.type_lines.len()
            + self.todo_lines.len()
            + self.import_lines.len()
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
        crate::comments::extract_first_comment(path).map(|text| MetadataBlock::from_comments(&text))
    }

    fn name(&self) -> &'static str {
        "comments"
    }
}

/// Order in which to display metadata types when both are enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetadataOrder {
    /// Comments first, then types (default)
    #[default]
    CommentsFirst,
    /// Types first, then comments
    TypesFirst,
}

/// Configuration for which metadata extractors to use.
#[derive(Debug, Clone, Default)]
pub struct MetadataConfig {
    /// Show comments
    pub comments: bool,
    /// Show type signatures (--types / -t)
    pub types: bool,
    /// Show TODO/FIXME markers (--todos)
    pub todos: bool,
    /// Show full metadata blocks (multi-line) vs first line only
    pub full: bool,
    /// Optional prefix to add before each metadata line (e.g., "# ")
    pub prefix: Option<String>,
    /// Order to display metadata when both comments and types are enabled
    pub order: MetadataOrder,
}

impl MetadataConfig {
    /// Create a config that shows comments only (default behavior).
    pub fn comments_only(full: bool) -> Self {
        Self {
            comments: true,
            types: false,
            todos: false,
            full,
            prefix: None,
            order: MetadataOrder::CommentsFirst,
        }
    }

    /// Create a config that shows type signatures only.
    pub fn types_only(full: bool) -> Self {
        Self {
            comments: false,
            types: true,
            todos: false,
            full,
            prefix: None,
            order: MetadataOrder::TypesFirst,
        }
    }

    /// Create a config that shows both comments and types.
    pub fn all(full: bool, order: MetadataOrder) -> Self {
        Self {
            comments: true,
            types: true,
            todos: false,
            full,
            prefix: None,
            order,
        }
    }

    /// Create a config that disables all metadata.
    pub fn none() -> Self {
        Self {
            comments: false,
            types: false,
            todos: false,
            full: false,
            prefix: None,
            order: MetadataOrder::CommentsFirst,
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
    fn test_metadata_block_from_comments() {
        let block = MetadataBlock::from_comments("line 1\nline 2\nline 3");
        assert_eq!(block.comment_lines.len(), 3);
        assert_eq!(block.comment_lines[0].content, "line 1");
        assert_eq!(block.comment_lines[1].content, "line 2");
        assert_eq!(block.comment_lines[2].content, "line 3");
        assert!(block.type_lines.is_empty());
    }

    #[test]
    fn test_metadata_block_from_types() {
        use crate::extractors::types::TypeSignature;
        let block = MetadataBlock::from_types(vec![
            TypeSignature::new("pub fn foo()".to_string(), "foo".to_string(), 0),
            TypeSignature::new("pub struct Bar".to_string(), "Bar".to_string(), 4),
        ]);
        assert!(block.comment_lines.is_empty());
        assert_eq!(block.type_lines.len(), 2);
        assert_eq!(block.type_lines[0].content, "pub fn foo()");
        assert_eq!(block.type_lines[0].style, LineStyle::TypeSignature);
        assert_eq!(block.type_lines[0].symbol_name, Some("foo".to_string()));
        assert_eq!(block.type_lines[0].indent, 0);
        assert_eq!(block.type_lines[1].symbol_name, Some("Bar".to_string()));
        assert_eq!(block.type_lines[1].indent, 4);
    }

    #[test]
    fn test_metadata_block_is_empty() {
        let empty = MetadataBlock::new();
        assert!(empty.is_empty());

        let with_comments = MetadataBlock::from_comments("content");
        assert!(!with_comments.is_empty());

        use crate::extractors::types::TypeSignature;
        let with_types = MetadataBlock::from_types(vec![TypeSignature::new(
            "fn foo()".to_string(),
            "foo".to_string(),
            0,
        )]);
        assert!(!with_types.is_empty());
    }

    #[test]
    fn test_metadata_block_lines_in_order() {
        let mut block = MetadataBlock::new();
        block.comment_lines = vec![MetadataLine::new("comment".to_string())];
        block.type_lines = vec![MetadataLine::with_style(
            "pub fn foo()".to_string(),
            LineStyle::TypeSignature,
        )];

        // Comments first order
        let lines = block.lines_in_order(MetadataOrder::CommentsFirst);
        assert_eq!(lines.len(), 3); // comment, separator, type
        assert_eq!(lines[0].content, "comment");
        assert_eq!(lines[1].content, ""); // separator
        assert_eq!(lines[2].content, "pub fn foo()");

        // Types first order
        let lines = block.lines_in_order(MetadataOrder::TypesFirst);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].content, "pub fn foo()");
        assert_eq!(lines[1].content, ""); // separator
        assert_eq!(lines[2].content, "comment");
    }

    #[test]
    fn test_metadata_block_first_section_single_line() {
        let mut block = MetadataBlock::new();
        block.comment_lines = vec![MetadataLine::new("single".to_string())];
        block.type_lines = vec![
            MetadataLine::with_style("fn a()".to_string(), LineStyle::TypeSignature),
            MetadataLine::with_style("fn b()".to_string(), LineStyle::TypeSignature),
        ];

        assert!(block.first_section_is_single_line(MetadataOrder::CommentsFirst));
        assert!(!block.first_section_is_single_line(MetadataOrder::TypesFirst));
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

        let all = MetadataConfig::all(false, MetadataOrder::CommentsFirst);
        assert!(all.comments);
        assert!(all.types);
        assert!(!all.full);
        assert_eq!(all.order, MetadataOrder::CommentsFirst);

        let all_types_first = MetadataConfig::all(true, MetadataOrder::TypesFirst);
        assert_eq!(all_types_first.order, MetadataOrder::TypesFirst);

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

    #[test]
    fn test_has_only_comments_excludes_imports() {
        let mut block = MetadataBlock::new();
        block.comment_lines = vec![MetadataLine::new("comment")];
        assert!(block.has_only_comments());

        // Adding imports should make has_only_comments return false
        block.import_lines = vec![MetadataLine::with_style("use foo", LineStyle::Import)];
        assert!(!block.has_only_comments());
    }

    #[test]
    fn test_has_only_types_excludes_imports() {
        let mut block = MetadataBlock::new();
        block.type_lines = vec![MetadataLine::with_style(
            "fn foo()",
            LineStyle::TypeSignature,
        )];
        assert!(block.has_only_types());

        // Adding imports should make has_only_types return false
        block.import_lines = vec![MetadataLine::with_style("use foo", LineStyle::Import)];
        assert!(!block.has_only_types());
    }

    #[test]
    fn test_has_only_imports() {
        let mut block = MetadataBlock::new();

        // Empty block is not "only imports"
        assert!(!block.has_only_imports());

        // Only imports
        block.import_lines = vec![MetadataLine::with_style("use foo", LineStyle::Import)];
        assert!(block.has_only_imports());

        // Adding comments should make has_only_imports return false
        block.comment_lines = vec![MetadataLine::new("comment")];
        assert!(!block.has_only_imports());
    }

    #[test]
    fn test_has_only_todos() {
        let mut block = MetadataBlock::new();

        // Empty block is not "only todos"
        assert!(!block.has_only_todos());

        // Only todos
        block.todo_lines = vec![MetadataLine::with_style("TODO: fix", LineStyle::Todo)];
        assert!(block.has_only_todos());

        // Adding imports should make has_only_todos return false
        block.import_lines = vec![MetadataLine::with_style("use foo", LineStyle::Import)];
        assert!(!block.has_only_todos());
    }
}
