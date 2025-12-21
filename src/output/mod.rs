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
pub use markdown::{MarkdownFormatter, print_markdown};
pub use streaming::StreamingFormatter;
pub use tree::TreeFormatter;

// Re-export utility functions used by tests
pub use utils::{
    calculate_wrap_width, continuation_prefix, first_line, has_indented_children,
    print_metadata_block, should_insert_group_separator, wrap_text, write_inline_content,
    write_rendered_line,
};

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::metadata::MetadataConfig;
    use crate::tree::{StreamingOutput, TreeNode};

    use super::*;

    // ==================== Metadata Block Display Tests ====================

    #[test]
    fn test_metadata_block_inline_display_single_line() {
        // When not in full mode, only the first line should show inline
        let tree = TreeNode::File {
            name: "test.rs".to_string(),
            path: PathBuf::from("test.rs"),
            comments: Some("Single line comment".to_string()),
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
            comments: Some("First line\nSecond line\nThird line".to_string()),
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
            comments: Some("First line\nSecond line".to_string()),
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
            comments: None,
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
            comments: Some("Comment".to_string()),
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

    // ==================== Integration Tests: Cross-Format Consistency ====================

    /// Helper to build a test tree with various metadata types for integration tests.
    fn build_integration_test_tree() -> TreeNode {
        use crate::imports::FileImports;
        use crate::tree::{JsonTodoItem, JsonTypeItem};

        TreeNode::Dir {
            name: "test_project".to_string(),
            path: PathBuf::from("test_project"),
            children: vec![
                TreeNode::File {
                    name: "simple.txt".to_string(),
                    path: PathBuf::from("test_project/simple.txt"),
                    comments: None,
                    types: None,
                    todos: None,
                    imports: None,
                    size_bytes: Some(42),
                    size_human: Some("42B".to_string()),
                },
                TreeNode::File {
                    name: "app.rs".to_string(),
                    path: PathBuf::from("test_project/app.rs"),
                    comments: Some(
                        "Main application module\nProvides core functionality".to_string(),
                    ),
                    types: Some(vec![
                        JsonTypeItem::new("pub fn main()".to_string(), "main".to_string(), 0),
                        JsonTypeItem::new("pub struct App".to_string(), "App".to_string(), 0),
                    ]),
                    todos: Some(vec![
                        JsonTodoItem {
                            marker_type: "TODO".to_string(),
                            text: "Add error handling".to_string(),
                            line: 42,
                        },
                        JsonTodoItem {
                            marker_type: "FIXME".to_string(),
                            text: "Optimize performance".to_string(),
                            line: 108,
                        },
                    ]),
                    imports: Some(FileImports {
                        std: vec!["std::io".to_string(), "std::fs".to_string()],
                        external: vec!["serde".to_string()],
                        internal: vec!["crate::utils".to_string()],
                    }),
                    size_bytes: Some(2048),
                    size_human: Some("2.0K".to_string()),
                },
                TreeNode::Dir {
                    name: "src".to_string(),
                    path: PathBuf::from("test_project/src"),
                    children: vec![
                        TreeNode::File {
                            name: "lib.rs".to_string(),
                            path: PathBuf::from("test_project/src/lib.rs"),
                            comments: Some("Library module".to_string()),
                            types: None,
                            todos: None,
                            imports: None,
                            size_bytes: Some(512),
                            size_human: Some("512B".to_string()),
                        },
                        TreeNode::File {
                            name: "config.rs".to_string(),
                            path: PathBuf::from("test_project/src/config.rs"),
                            comments: None,
                            types: Some(vec![JsonTypeItem::new(
                                "pub struct Config".to_string(),
                                "Config".to_string(),
                                0,
                            )]),
                            todos: None,
                            imports: None,
                            size_bytes: Some(256),
                            size_human: Some("256B".to_string()),
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn test_json_serialization_stability() {
        // Build a test tree with all metadata types
        let tree = build_integration_test_tree();

        // Serialize multiple times to verify stability
        let json1 = serde_json::to_string_pretty(&tree).unwrap();
        let json2 = serde_json::to_string_pretty(&tree).unwrap();

        // Verify serialization is deterministic
        assert_eq!(json1, json2, "JSON serialization should be stable");

        // Verify the JSON is parseable as a valid JSON value
        let parsed: serde_json::Value = serde_json::from_str(&json1).unwrap();
        assert!(parsed.is_object(), "JSON should be a valid object");
    }

    #[test]
    fn test_all_formats_contain_same_files() {
        // Build a test tree
        let tree = build_integration_test_tree();

        // Generate outputs in different formats
        let config = OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(false),
            wrap_width: None,
        };

        let console_output = TreeFormatter::new(config.clone()).format(&tree);
        let json_output = serde_json::to_string_pretty(&tree).unwrap();

        let mut markdown_formatter = MarkdownFormatter::new(config);
        markdown_formatter
            .output_node("test_project", None, true, true, "", true, None)
            .unwrap();
        markdown_formatter
            .output_node("simple.txt", None, false, false, "", false, Some(42))
            .unwrap();
        markdown_formatter
            .output_node("app.rs", None, false, false, "", false, Some(2048))
            .unwrap();
        markdown_formatter
            .output_node("src", None, true, false, "    ", false, None)
            .unwrap();
        markdown_formatter
            .output_node("lib.rs", None, false, false, "        ", false, Some(512))
            .unwrap();
        markdown_formatter
            .output_node("config.rs", None, false, true, "        ", false, Some(256))
            .unwrap();
        let markdown_output = markdown_formatter.output();

        // Verify all formats contain the same file names
        let files = vec!["simple.txt", "app.rs", "lib.rs", "config.rs"];
        for file in &files {
            assert!(
                console_output.contains(file),
                "Console output should contain {}",
                file
            );
            assert!(
                json_output.contains(file),
                "JSON output should contain {}",
                file
            );
            assert!(
                markdown_output.contains(file),
                "Markdown output should contain {}",
                file
            );
        }

        // Verify directory names
        let dirs = vec!["test_project", "src"];
        for dir in &dirs {
            assert!(
                console_output.contains(dir),
                "Console output should contain directory {}",
                dir
            );
            assert!(
                json_output.contains(dir),
                "JSON output should contain directory {}",
                dir
            );
            assert!(
                markdown_output.contains(dir),
                "Markdown output should contain directory {}",
                dir
            );
        }
    }

    #[test]
    fn test_all_formats_contain_same_metadata() {
        // Build a test tree with metadata
        let tree = build_integration_test_tree();

        // Generate outputs with full metadata
        let config = OutputConfig {
            use_color: false,
            metadata: MetadataConfig::all(true, crate::metadata::MetadataOrder::CommentsFirst),
            wrap_width: None,
        };

        let console_output = TreeFormatter::new(config).format(&tree);
        let json_output = serde_json::to_string_pretty(&tree).unwrap();

        // Verify comments are in both outputs
        assert!(
            console_output.contains("Main application module"),
            "Console should contain comment"
        );
        assert!(
            json_output.contains("Main application module"),
            "JSON should contain comment"
        );
        assert!(
            console_output.contains("Provides core functionality"),
            "Console should contain multiline comment"
        );
        assert!(
            json_output.contains("Provides core functionality"),
            "JSON should contain multiline comment"
        );

        // Verify type signatures are in both outputs
        assert!(
            console_output.contains("pub fn main()"),
            "Console should contain type signature"
        );
        assert!(
            json_output.contains("pub fn main()"),
            "JSON should contain type signature"
        );
        assert!(
            console_output.contains("pub struct App"),
            "Console should contain struct"
        );
        assert!(
            json_output.contains("pub struct App"),
            "JSON should contain struct"
        );

        // Verify TODOs are in both outputs
        // Note: Console formats as "TODO: Add error handling (line 42)"
        // while JSON has structured fields
        assert!(
            console_output.contains("TODO") && console_output.contains("Add error handling"),
            "Console should contain TODO"
        );
        assert!(
            json_output.contains("TODO") && json_output.contains("Add error handling"),
            "JSON should contain TODO"
        );
        assert!(
            console_output.contains("FIXME") && console_output.contains("Optimize performance"),
            "Console should contain FIXME"
        );
        assert!(
            json_output.contains("FIXME") && json_output.contains("Optimize performance"),
            "JSON should contain FIXME"
        );

        // Verify imports are in both outputs
        assert!(
            console_output.contains("serde"),
            "Console should contain import"
        );
        assert!(json_output.contains("serde"), "JSON should contain import");
        assert!(
            console_output.contains("std::io"),
            "Console should contain std import"
        );
        assert!(
            json_output.contains("std::io"),
            "JSON should contain std import"
        );
    }

    #[test]
    fn test_all_formats_preserve_tree_structure() {
        // Build a nested tree
        let tree = build_integration_test_tree();

        let config = OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(false),
            wrap_width: None,
        };

        let console_output = TreeFormatter::new(config).format(&tree);

        // Verify structure in console output (tree connectors)
        assert!(
            console_output.contains("├──") || console_output.contains("└──"),
            "Console should have tree structure"
        );

        // Verify file/dir counts match across formats
        // Root is not counted, so only "src" counts as 1 directory
        assert!(
            console_output.contains("1 directories, 4 files"),
            "Console should show correct counts"
        );

        // Verify JSON structure has nested children
        let json_output = serde_json::to_string_pretty(&tree).unwrap();
        assert!(
            json_output.contains("\"children\""),
            "JSON should have children arrays"
        );
        assert!(
            json_output.contains("\"type\": \"dir\""),
            "JSON should have directory type"
        );
        assert!(
            json_output.contains("\"type\": \"file\""),
            "JSON should have file type"
        );
    }

    #[test]
    fn test_json_output_is_valid_and_parseable() {
        let tree = build_integration_test_tree();

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&tree).unwrap();

        // Parse as generic JSON value to verify it's valid
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify it has expected structure
        assert!(parsed.is_object(), "Root should be an object");
        assert_eq!(
            parsed.get("type").and_then(|v| v.as_str()),
            Some("dir"),
            "Root should be a directory"
        );
        assert_eq!(
            parsed.get("name").and_then(|v| v.as_str()),
            Some("test_project"),
            "Root should have correct name"
        );
        assert!(
            parsed.get("children").and_then(|v| v.as_array()).is_some(),
            "Root should have children array"
        );
    }

    #[test]
    fn test_console_and_markdown_have_consistent_file_counts() {
        let tree = build_integration_test_tree();

        let config = OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(false),
            wrap_width: None,
        };

        let console_output = TreeFormatter::new(config.clone()).format(&tree);

        // Console shows "1 directories, 4 files" (root not counted)
        assert!(console_output.contains("1 directories, 4 files"));

        // Markdown formatter tracks counts and outputs them in finish()
        let mut markdown_formatter = MarkdownFormatter::new(config);
        markdown_formatter
            .output_node("test_project", None, true, true, "", true, None)
            .unwrap();
        markdown_formatter
            .output_node("simple.txt", None, false, false, "", false, None)
            .unwrap();
        markdown_formatter
            .output_node("app.rs", None, false, false, "", false, None)
            .unwrap();
        markdown_formatter
            .output_node("src", None, true, false, "    ", false, None)
            .unwrap();
        markdown_formatter
            .output_node("lib.rs", None, false, false, "        ", false, None)
            .unwrap();
        markdown_formatter
            .output_node("config.rs", None, false, true, "        ", false, None)
            .unwrap();
        markdown_formatter.finish(1, 4).unwrap();

        let markdown_output = markdown_formatter.output();

        // Markdown shows "*1 directories, 4 files*"
        assert!(markdown_output.contains("*1 directories, 4 files*"));
    }
}
