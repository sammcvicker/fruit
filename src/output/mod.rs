//! Tree formatting and display
//!
//! This module provides formatters for outputting tree structures in various formats:
//! - Console output with colors (streaming or buffered)
//! - JSON output
//! - Markdown output
//!
//! # Module Structure
//!
//! - `config` - Output configuration types
//! - `utils` - Shared utility functions (text wrapping, prefix calculation)
//! - `tree` - Buffered tree formatter for complete tree structures
//! - `streaming` - Streaming formatter for console output
//! - `markdown` - Markdown output formatter
//! - `json` - JSON output

mod config;
mod json;
mod markdown;
mod streaming;
mod tree;
mod utils;

// Re-export public types and functions
pub use config::OutputConfig;
pub use json::print_json;
pub use markdown::{print_markdown, MarkdownFormatter};
pub use streaming::StreamingFormatter;
pub use tree::TreeFormatter;

// Re-export utility functions used by tests
pub use utils::{
    calculate_wrap_width, continuation_prefix, first_line, has_indented_children,
    should_insert_group_separator, wrap_text,
};

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::metadata::MetadataConfig;
    use crate::tree::TreeNode;

    use super::*;

    // ==================== Metadata Block Display Tests ====================

    #[test]
    fn test_metadata_block_inline_display_single_line() {
        // When not in full mode, only the first line should show inline
        let tree = TreeNode::File {
            name: "test.rs".to_string(),
            path: PathBuf::from("test.rs"),
            comment: Some("Single line comment".to_string()),
            types: None,
            todos: None,
            imports: None,
            size_bytes: None,
            size_human: None,
        };
        let root = TreeNode::Dir {
            name: ".".to_string(),
            path: PathBuf::from("."),
            children: vec![tree],
        };
        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(false), // Not full mode
            wrap_width: None,
        });
        let output = formatter.format(&root);

        // Should have the comment on the same line as the filename
        // No prefix is set, so it should just have "  " before the comment
        assert!(output.contains("test.rs  Single line comment"));
    }

    #[test]
    fn test_metadata_block_inline_display_multiline_comment_first_only() {
        // Even with multi-line comments, only first line should show when not in full mode
        let tree = TreeNode::File {
            name: "test.rs".to_string(),
            path: PathBuf::from("test.rs"),
            comment: Some("First line\nSecond line\nThird line".to_string()),
            types: None,
            todos: None,
            imports: None,
            size_bytes: None,
            size_human: None,
        };
        let root = TreeNode::Dir {
            name: ".".to_string(),
            path: PathBuf::from("."),
            children: vec![tree],
        };
        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(false), // Not full mode
            wrap_width: None,
        });
        let output = formatter.format(&root);

        // Should only show first line inline
        // No prefix is set, so it should just have "  " before the comment
        assert!(output.contains("test.rs  First line"));
        // Should NOT contain the other lines
        assert!(!output.contains("Second line"));
        assert!(!output.contains("Third line"));
    }

    #[test]
    fn test_metadata_block_multiline_display() {
        // In full mode, all lines should display below the filename
        let tree = TreeNode::File {
            name: "test.rs".to_string(),
            path: PathBuf::from("test.rs"),
            comment: Some("First line\nSecond line".to_string()),
            types: None,
            todos: None,
            imports: None,
            size_bytes: None,
            size_human: None,
        };
        let root = TreeNode::Dir {
            name: ".".to_string(),
            path: PathBuf::from("."),
            children: vec![tree],
        };
        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(true), // Full mode
            wrap_width: None,
        });
        let output = formatter.format(&root);

        // Should contain both lines
        assert!(output.contains("First line"));
        assert!(output.contains("Second line"));
    }

    #[test]
    fn test_metadata_block_empty_displays_no_extra_content() {
        // When a file has no metadata, output should be clean
        let tree = TreeNode::File {
            name: "test.rs".to_string(),
            path: PathBuf::from("test.rs"),
            comment: None,
            types: None,
            todos: None,
            imports: None,
            size_bytes: None,
            size_human: None,
        };
        let root = TreeNode::Dir {
            name: ".".to_string(),
            path: PathBuf::from("."),
            children: vec![tree],
        };
        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(false),
            wrap_width: None,
        });
        let output = formatter.format(&root);

        // Line should just be the filename
        let lines: Vec<&str> = output.lines().collect();
        // Find the line with test.rs
        let test_line = lines
            .iter()
            .find(|l| l.contains("test.rs"))
            .expect("Expected output to contain a line with 'test.rs'");
        // Should end with "test.rs" (possibly with tree connector)
        assert!(
            test_line.trim().ends_with("test.rs"),
            "Expected line to end with just filename, got: {}",
            test_line
        );
    }

    #[test]
    fn test_metadata_with_custom_prefix() {
        let tree = TreeNode::File {
            name: "test.rs".to_string(),
            path: PathBuf::from("test.rs"),
            comment: Some("Comment".to_string()),
            types: None,
            todos: None,
            imports: None,
            size_bytes: None,
            size_human: None,
        };
        let root = TreeNode::Dir {
            name: ".".to_string(),
            path: PathBuf::from("."),
            children: vec![tree],
        };
        let config = MetadataConfig::comments_only(false).with_prefix("// ".to_string());
        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: config,
            wrap_width: None,
        });
        let output = formatter.format(&root);

        // Should use the custom prefix
        assert!(output.contains("// Comment"));
    }
}
