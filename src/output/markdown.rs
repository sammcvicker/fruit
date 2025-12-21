//! Markdown output formatting
//!
//! This module provides `MarkdownFormatter` which outputs tree content
//! as a nested markdown list, suitable for documentation or LLM context.

use std::io;

use crate::metadata::MetadataBlock;
use crate::tree::StreamingOutput;

use super::config::OutputConfig;
use super::utils::first_line;

/// Markdown output formatter - outputs tree as nested markdown list.
/// Implements the StreamingOutput trait for use with StreamingWalker.
pub struct MarkdownFormatter {
    config: OutputConfig,
    output: String,
}

impl MarkdownFormatter {
    pub fn new(config: OutputConfig) -> Self {
        Self {
            config,
            output: String::new(),
        }
    }

    /// Get the formatted output string.
    pub fn output(&self) -> &str {
        &self.output
    }

    /// Take ownership of the output string.
    pub fn into_output(self) -> String {
        self.output
    }
}

impl StreamingOutput for MarkdownFormatter {
    fn output_node(
        &mut self,
        name: &str,
        metadata: Option<MetadataBlock>,
        is_dir: bool,
        _is_last: bool,
        prefix: &str,
        is_root: bool,
        size: Option<u64>,
    ) -> io::Result<()> {
        // Calculate indentation level from prefix length
        // Each level is 4 spaces to match tree output format
        let indent_level = if is_root { 0 } else { (prefix.len() / 4) + 1 };
        let indent = "    ".repeat(indent_level);

        if is_dir {
            // Directories in bold
            self.output.push_str(&indent);
            self.output.push_str("- **");
            self.output.push_str(name);
            self.output.push_str("/**\n");
        } else {
            // Files with optional metadata
            self.output.push_str(&indent);
            self.output.push_str("- `");
            self.output.push_str(name);
            self.output.push('`');

            // Show file size if provided
            if let Some(bytes) = size {
                self.output.push_str(" (");
                self.output.push_str(&crate::tree::format_size(bytes));
                self.output.push(')');
            }

            // Add metadata if present
            if let Some(ref block) = metadata {
                if !block.is_empty() {
                    let order = self.config.metadata.order;

                    if !self.config.show_full() {
                        // Inline mode: show first line after filename
                        if let Some(first) = block.first_line(order) {
                            self.output.push_str(" - ");
                            self.output.push_str(first_line(&first.content));
                        }
                    } else {
                        // Full mode: show first line inline, rest as nested content
                        if let Some(first) = block.first_line(order) {
                            self.output.push_str(" - ");
                            self.output.push_str(first_line(&first.content));
                        }

                        // If there's more than one line, show the rest as a nested block
                        let lines = block.lines_in_order(order);
                        if lines.len() > 1 {
                            self.output.push('\n');
                            let nested_indent = "    ".repeat(indent_level + 1);
                            self.output.push_str(&nested_indent);
                            self.output.push('\n');
                            self.output.push_str(&nested_indent);
                            self.output.push_str("> ");

                            // Skip the first line (already shown inline) and format the rest
                            let remaining: Vec<_> = lines
                                .iter()
                                .skip(1)
                                .filter(|l| !l.content.trim().is_empty())
                                .collect();

                            for (i, line) in remaining.iter().enumerate() {
                                if i > 0 {
                                    self.output.push('\n');
                                    self.output.push_str(&nested_indent);
                                    self.output.push_str("> ");
                                }
                                self.output.push_str(line.content.trim());
                            }
                            self.output.push('\n');
                        }
                    }
                }
            }
            self.output.push('\n');
        }
        Ok(())
    }

    fn finish(&mut self, dir_count: usize, file_count: usize) -> io::Result<()> {
        self.output.push('\n');
        self.output.push_str(&format!(
            "*{} directories, {} files*\n",
            dir_count, file_count
        ));
        Ok(())
    }
}

/// Print markdown output to stdout.
pub fn print_markdown(formatter: &MarkdownFormatter) -> io::Result<()> {
    print!("{}", formatter.output());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{LineStyle, MetadataConfig, MetadataLine, MetadataOrder};

    fn make_config(full: bool) -> OutputConfig {
        OutputConfig {
            use_color: false,
            metadata: MetadataConfig {
                comments: true,
                types: false,
                todos: false,
                full,
                prefix: None,
                order: MetadataOrder::CommentsFirst,
            },
            wrap_width: None,
        }
    }

    #[test]
    fn test_markdown_directory_format() {
        let config = make_config(false);
        let mut formatter = MarkdownFormatter::new(config);

        // Output root directory
        formatter
            .output_node("my_project", None, true, true, "", true, None)
            .unwrap();

        let output = formatter.output();
        assert!(
            output.contains("**my_project/**"),
            "directory should be bold with trailing slash: {}",
            output
        );
    }

    #[test]
    fn test_markdown_file_format() {
        let config = make_config(false);
        let mut formatter = MarkdownFormatter::new(config);

        // Output a file (not root)
        formatter
            .output_node("main.rs", None, false, true, "    ", false, None)
            .unwrap();

        let output = formatter.output();
        assert!(
            output.contains("`main.rs`"),
            "filename should be in backticks: {}",
            output
        );
    }

    #[test]
    fn test_markdown_file_with_size() {
        let config = make_config(false);
        let mut formatter = MarkdownFormatter::new(config);

        // Output a file with size
        formatter
            .output_node("main.rs", None, false, true, "    ", false, Some(1024))
            .unwrap();

        let output = formatter.output();
        assert!(
            output.contains("`main.rs`"),
            "filename should be in backticks: {}",
            output
        );
        // Size is formatted by format_size which uses "1.0K" format
        assert!(output.contains("1.0K"), "should show file size: {}", output);
    }

    #[test]
    fn test_markdown_file_with_comment() {
        let config = make_config(false);
        let mut formatter = MarkdownFormatter::new(config);

        let mut block = MetadataBlock::new();
        block.comment_lines = vec![MetadataLine::new("This is a module comment")];

        formatter
            .output_node("lib.rs", Some(block), false, true, "    ", false, None)
            .unwrap();

        let output = formatter.output();
        assert!(
            output.contains("`lib.rs`"),
            "filename should be in backticks: {}",
            output
        );
        assert!(
            output.contains("This is a module comment"),
            "should contain comment: {}",
            output
        );
        assert!(
            output.contains(" - "),
            "should have separator before comment: {}",
            output
        );
    }

    #[test]
    fn test_markdown_nested_indentation() {
        let config = make_config(false);
        let mut formatter = MarkdownFormatter::new(config);

        // Simulate nested structure
        // Prefix represents tree prefix characters (4 chars per level: "    " or "│   ")
        formatter
            .output_node("project", None, true, true, "", true, None)
            .unwrap();
        formatter
            .output_node("src", None, true, false, "    ", false, None)
            .unwrap();
        formatter
            .output_node("main.rs", None, false, true, "        ", false, None)
            .unwrap();

        let output = formatter.output();
        let lines: Vec<&str> = output.lines().collect();

        // Root should have no indentation
        assert!(
            lines[0].starts_with("- **"),
            "root should start with '- **': {}",
            lines[0]
        );
        // First level: prefix is "    " (4 chars) -> indent_level = 1 + 1 = 2 -> 8 spaces
        assert!(
            lines[1].starts_with("        - **"),
            "first level dir should have 8 spaces: {}",
            lines[1]
        );
        // Second level: prefix is "        " (8 chars) -> indent_level = 2 + 1 = 3 -> 12 spaces
        assert!(
            lines[2].starts_with("            - `"),
            "second level file should have 12 spaces: {}",
            lines[2]
        );
    }

    #[test]
    fn test_markdown_multiline_comment_full_mode() {
        let config = make_config(true); // full mode
        let mut formatter = MarkdownFormatter::new(config);

        let mut block = MetadataBlock::new();
        block.comment_lines = vec![
            MetadataLine::new("First line of comment"),
            MetadataLine::new("Second line of comment"),
            MetadataLine::new("Third line of comment"),
        ];

        formatter
            .output_node("lib.rs", Some(block), false, true, "    ", false, None)
            .unwrap();

        let output = formatter.output();
        assert!(
            output.contains("First line of comment"),
            "should contain first line: {}",
            output
        );
        // In full mode, remaining lines are shown as blockquote
        assert!(
            output.contains("> "),
            "should use blockquote for additional lines: {}",
            output
        );
        assert!(
            output.contains("Second line of comment"),
            "should contain second line: {}",
            output
        );
        assert!(
            output.contains("Third line of comment"),
            "should contain third line: {}",
            output
        );
    }

    #[test]
    fn test_markdown_finish_summary() {
        let config = make_config(false);
        let mut formatter = MarkdownFormatter::new(config);

        formatter.finish(5, 23).unwrap();

        let output = formatter.output();
        assert!(
            output.contains("*5 directories, 23 files*"),
            "should show italicized summary: {}",
            output
        );
    }

    #[test]
    fn test_markdown_special_filename_chars() {
        let config = make_config(false);
        let mut formatter = MarkdownFormatter::new(config);

        // Test filename with underscores and dots (common in Rust/Python)
        formatter
            .output_node("my_module.test.rs", None, false, true, "    ", false, None)
            .unwrap();

        let output = formatter.output();
        assert!(
            output.contains("`my_module.test.rs`"),
            "filename with special chars should be preserved: {}",
            output
        );
    }

    #[test]
    fn test_markdown_type_signatures() {
        let config = OutputConfig {
            use_color: false,
            metadata: MetadataConfig {
                comments: false,
                types: true,
                todos: false,
                full: false,
                prefix: None,
                order: MetadataOrder::TypesFirst,
            },
            wrap_width: None,
        };
        let mut formatter = MarkdownFormatter::new(config);

        let mut block = MetadataBlock::new();
        block.type_lines = vec![MetadataLine::with_style(
            "pub fn main()",
            LineStyle::TypeSignature,
        )];

        formatter
            .output_node("main.rs", Some(block), false, true, "    ", false, None)
            .unwrap();

        let output = formatter.output();
        assert!(
            output.contains("pub fn main()"),
            "should show type signature: {}",
            output
        );
    }

    #[test]
    fn test_markdown_todo_markers() {
        let config = OutputConfig {
            use_color: false,
            metadata: MetadataConfig {
                comments: false,
                types: false,
                todos: true,
                full: false,
                prefix: None,
                order: MetadataOrder::CommentsFirst,
            },
            wrap_width: None,
        };
        let mut formatter = MarkdownFormatter::new(config);

        let mut block = MetadataBlock::new();
        block.todo_lines = vec![MetadataLine::with_style("TODO: fix this", LineStyle::Todo)];

        formatter
            .output_node("main.rs", Some(block), false, true, "    ", false, None)
            .unwrap();

        let output = formatter.output();
        assert!(
            output.contains("TODO: fix this"),
            "should show TODO marker: {}",
            output
        );
    }
}
