//! Tree formatter for buffered output
//!
//! This module provides `TreeFormatter` which formats a complete `TreeNode`
//! tree structure into a string or prints it with colors.

use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::metadata::{MetadataBlock, MetadataLine};
use crate::tree::TreeNode;

use super::config::OutputConfig;
use super::utils::{
    calculate_wrap_width, continuation_prefix, first_line, has_indented_children,
    should_insert_group_separator, wrap_text, write_metadata_line_with_symbol,
};

/// Formatter for buffered tree output.
pub struct TreeFormatter {
    config: OutputConfig,
}

impl TreeFormatter {
    pub fn new(config: OutputConfig) -> Self {
        Self { config }
    }

    pub fn format(&self, node: &TreeNode) -> String {
        let mut output = String::new();
        let (dir_count, file_count) = self.format_node(node, &mut output, "", true, true);
        output.push_str(&format!(
            "\n{} directories, {} files\n",
            dir_count, file_count
        ));
        output
    }

    pub fn print(&self, node: &TreeNode) -> io::Result<()> {
        let choice = if self.config.use_color {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };
        let mut stdout = StandardStream::stdout(choice);
        let (dir_count, file_count) = self.print_node(node, &mut stdout, "", true, true)?;
        writeln!(stdout)?;
        writeln!(stdout, "{} directories, {} files", dir_count, file_count)?;
        Ok(())
    }

    /// Print a metadata block with colors to stdout.
    fn print_metadata_block(
        &self,
        stdout: &mut StandardStream,
        block: &MetadataBlock,
        prefix: &str,
        is_last: bool,
    ) -> io::Result<()> {
        if block.is_empty() {
            writeln!(stdout)?;
            return Ok(());
        }

        let meta_prefix = self.config.metadata.prefix_str();
        let order = self.config.metadata.order;
        let lines = block.lines_in_order(order);

        // Not in full mode: show first line inline only
        if !self.config.show_full() {
            if let Some(first) = block.first_line(order) {
                write!(stdout, "  {}", meta_prefix)?;
                write_metadata_line_with_symbol(
                    stdout,
                    first_line(&first.content),
                    first.symbol_name.as_deref(),
                    first.style.color(),
                    first.style.is_intense(),
                    first.indent,
                )?;
            }
            writeln!(stdout)?;
            stdout.reset()?;
            return Ok(());
        }

        // Full mode: display in a block beneath filename
        writeln!(stdout)?; // End the filename line

        let cont_prefix = continuation_prefix(prefix, is_last);
        let wrap_width = calculate_wrap_width(
            self.config.wrap_width,
            cont_prefix.chars().count(),
            meta_prefix.chars().count(),
        );

        // Blank line before block
        stdout.reset()?;
        writeln!(stdout, "{}", cont_prefix)?;

        // Metadata lines with per-line styling
        let line_refs: Vec<&MetadataLine> = lines.iter().collect();
        let mut prev_indent: Option<usize> = None;
        for (i, meta_line) in lines.iter().enumerate() {
            let content = meta_line.content.trim();

            // Empty line is a separator
            if content.is_empty() {
                stdout.reset()?;
                writeln!(stdout, "{}", cont_prefix)?;
                prev_indent = None; // Reset indent tracking after separator
                continue;
            }

            // Check if we should insert a group separator
            let has_children = has_indented_children(&line_refs[i + 1..], meta_line.indent);
            if should_insert_group_separator(meta_line.indent, prev_indent, has_children) {
                stdout.reset()?;
                writeln!(stdout, "{}", cont_prefix)?;
            }
            prev_indent = Some(meta_line.indent);

            let wrapped = if let Some(width) = wrap_width {
                wrap_text(content, width)
            } else {
                vec![content.to_string()]
            };

            for wrapped_line in wrapped.iter() {
                stdout.reset()?;
                write!(stdout, "{}{}", cont_prefix, meta_prefix)?;
                write_metadata_line_with_symbol(
                    stdout,
                    wrapped_line,
                    meta_line.symbol_name.as_deref(),
                    meta_line.style.color(),
                    meta_line.style.is_intense(),
                    meta_line.indent,
                )?;
                writeln!(stdout)?;
            }
        }

        // Blank line after block
        stdout.reset()?;
        writeln!(stdout, "{}", cont_prefix)?;
        stdout.reset()?;
        Ok(())
    }

    /// Format a metadata block to plain text output.
    fn format_metadata_block_plain(
        &self,
        output: &mut String,
        block: &MetadataBlock,
        prefix: &str,
        is_last: bool,
    ) {
        if block.is_empty() {
            output.push('\n');
            return;
        }

        let meta_prefix = self.config.metadata.prefix_str();
        let order = self.config.metadata.order;
        let lines = block.lines_in_order(order);

        // Not in full mode: show first line inline only
        if !self.config.show_full() {
            if let Some(first) = block.first_line(order) {
                output.push_str("  ");
                output.push_str(meta_prefix);
                output.push_str(first_line(&first.content));
            }
            output.push('\n');
            return;
        }

        // Full mode: display in a block beneath filename
        output.push('\n'); // End the filename line

        let cont_prefix = continuation_prefix(prefix, is_last);
        let wrap_width = calculate_wrap_width(
            self.config.wrap_width,
            cont_prefix.chars().count(),
            meta_prefix.chars().count(),
        );

        // Blank line before block
        output.push_str(&cont_prefix);
        output.push('\n');

        // Metadata lines with group separators (matches colored output logic)
        let line_refs: Vec<&MetadataLine> = lines.iter().collect();
        let mut prev_indent: Option<usize> = None;
        for (i, meta_line) in lines.iter().enumerate() {
            let content = meta_line.content.trim();

            // Empty line is a separator
            if content.is_empty() {
                output.push_str(&cont_prefix);
                output.push('\n');
                prev_indent = None; // Reset indent tracking after separator
                continue;
            }

            // Check if we should insert a group separator
            let has_children = has_indented_children(&line_refs[i + 1..], meta_line.indent);
            if should_insert_group_separator(meta_line.indent, prev_indent, has_children) {
                output.push_str(&cont_prefix);
                output.push('\n');
            }
            prev_indent = Some(meta_line.indent);

            let wrapped = if let Some(width) = wrap_width {
                wrap_text(content, width)
            } else {
                vec![content.to_string()]
            };

            for wrapped_line in wrapped.iter() {
                output.push_str(&cont_prefix);
                output.push_str(meta_prefix);
                output.push_str(wrapped_line);
                output.push('\n');
            }
        }

        // Blank line after block
        output.push_str(&cont_prefix);
        output.push('\n');
    }

    fn format_node(
        &self,
        node: &TreeNode,
        output: &mut String,
        prefix: &str,
        is_last: bool,
        is_root: bool,
    ) -> (usize, usize) {
        let connector = if is_last { "└── " } else { "├── " };

        match node {
            TreeNode::File { name, comment, .. } => {
                output.push_str(prefix);
                output.push_str(connector);
                output.push_str(name);
                if let Some(c) = comment {
                    // Convert comment to metadata block for unified handling
                    let block = MetadataBlock::from_comments(c);
                    self.format_metadata_block_plain(output, &block, prefix, is_last);
                } else {
                    output.push('\n');
                }
                (0, 1)
            }
            TreeNode::Dir { name, children, .. } => {
                if is_root {
                    // Root node - print without connector
                    output.push_str(name);
                    output.push('\n');
                } else {
                    output.push_str(prefix);
                    output.push_str(connector);
                    output.push_str(name);
                    output.push('\n');
                }

                let new_prefix = if is_root {
                    String::new()
                } else if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}│   ", prefix)
                };

                let mut dir_count = 0;
                let mut file_count = 0;

                for (i, child) in children.iter().enumerate() {
                    let child_is_last = i == children.len() - 1;
                    let (d, f) = self.format_node(child, output, &new_prefix, child_is_last, false);
                    dir_count += d;
                    file_count += f;
                    if child.is_dir() {
                        dir_count += 1;
                    }
                }

                (dir_count, file_count)
            }
        }
    }

    fn print_node(
        &self,
        node: &TreeNode,
        stdout: &mut StandardStream,
        prefix: &str,
        is_last: bool,
        is_root: bool,
    ) -> io::Result<(usize, usize)> {
        let connector = if is_last { "└── " } else { "├── " };

        match node {
            TreeNode::File { name, comment, .. } => {
                write!(stdout, "{}{}", prefix, connector)?;
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
                write!(stdout, "{}", name)?;
                stdout.reset()?;

                if let Some(c) = comment {
                    // Convert comment to metadata block for unified handling
                    let block = MetadataBlock::from_comments(c);
                    self.print_metadata_block(stdout, &block, prefix, is_last)?;
                } else {
                    writeln!(stdout)?;
                }
                Ok((0, 1))
            }
            TreeNode::Dir { name, children, .. } => {
                if is_root {
                    // Root node - print without connector
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
                    writeln!(stdout, "{}", name)?;
                    stdout.reset()?;
                } else {
                    write!(stdout, "{}{}", prefix, connector)?;
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
                    writeln!(stdout, "{}", name)?;
                    stdout.reset()?;
                }

                let new_prefix = if is_root {
                    String::new()
                } else if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}│   ", prefix)
                };

                let mut dir_count = 0;
                let mut file_count = 0;

                for (i, child) in children.iter().enumerate() {
                    let child_is_last = i == children.len() - 1;
                    let (d, f) =
                        self.print_node(child, stdout, &new_prefix, child_is_last, false)?;
                    dir_count += d;
                    file_count += f;
                    if child.is_dir() {
                        dir_count += 1;
                    }
                }

                Ok((dir_count, file_count))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::metadata::MetadataConfig;

    use super::*;

    fn sample_tree() -> TreeNode {
        TreeNode::Dir {
            name: ".".to_string(),
            path: PathBuf::from("."),
            children: vec![
                TreeNode::File {
                    name: "Cargo.toml".to_string(),
                    path: PathBuf::from("Cargo.toml"),
                    comment: Some("Package manifest".to_string()),
                    types: None,
                    todos: None,
                    imports: None,
                    size_bytes: None,
                    size_human: None,
                },
                TreeNode::Dir {
                    name: "src".to_string(),
                    path: PathBuf::from("src"),
                    children: vec![
                        TreeNode::File {
                            name: "main.rs".to_string(),
                            path: PathBuf::from("src/main.rs"),
                            comment: Some("CLI entry point".to_string()),
                            types: None,
                            todos: None,
                            imports: None,
                            size_bytes: None,
                            size_human: None,
                        },
                        TreeNode::File {
                            name: "lib.rs".to_string(),
                            path: PathBuf::from("src/lib.rs"),
                            comment: None,
                            types: None,
                            todos: None,
                            imports: None,
                            size_bytes: None,
                            size_human: None,
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn test_format_output() {
        let tree = sample_tree();
        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(false),
            wrap_width: None,
        });
        let output = formatter.format(&tree);

        assert!(output.contains("."));
        assert!(output.contains("├── Cargo.toml"));
        assert!(output.contains("Package manifest"));
        assert!(output.contains("└── src"));
        assert!(output.contains("├── main.rs"));
        assert!(output.contains("└── lib.rs"));
        assert!(output.contains("directories"));
        assert!(output.contains("files"));
    }

    #[test]
    fn test_dir_count() {
        let tree = sample_tree();
        let formatter = TreeFormatter::new(OutputConfig::default());
        let output = formatter.format(&tree);

        // Should count 1 directory (src) - root is not counted
        assert!(output.contains("1 directories, 3 files"));
    }
}
