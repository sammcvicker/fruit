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
        // Each level is 2 spaces in markdown list format
        let indent_level = if is_root { 0 } else { (prefix.len() / 4) + 1 };
        let indent = "  ".repeat(indent_level);

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
                            let nested_indent = "  ".repeat(indent_level + 1);
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
