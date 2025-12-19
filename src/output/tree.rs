//! Tree formatter for buffered output
//!
//! This module provides `TreeFormatter` which formats a complete `TreeNode`
//! tree structure into a string or prints it with colors.

use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::metadata::{LineStyle, MetadataBlock, MetadataLine};
use crate::tree::TreeNode;

use super::config::OutputConfig;
use super::utils::{
    MetadataRenderResult, RenderedLine, calculate_wrap_width, continuation_prefix,
    render_metadata_block, write_metadata_line_with_symbol,
};

/// Formatter for buffered tree output.
pub struct TreeFormatter {
    config: OutputConfig,
}

impl TreeFormatter {
    pub fn new(config: OutputConfig) -> Self {
        Self { config }
    }

    /// Build a MetadataBlock from TreeNode::File fields.
    /// This provides a unified way to construct metadata from all available fields
    /// (comment, types, todos, imports) stored in the TreeNode.
    fn build_metadata_block(
        comment: Option<&String>,
        types: Option<&Vec<String>>,
        todos: Option<&Vec<crate::tree::JsonTodoItem>>,
        imports: Option<&crate::imports::FileImports>,
    ) -> Option<MetadataBlock> {
        let mut block = MetadataBlock::new();

        // Add comment lines
        if let Some(c) = comment {
            block.comment_lines = c
                .lines()
                .map(|line| MetadataLine::new(line.to_string()))
                .collect();
        }

        // Add type signature lines
        if let Some(type_sigs) = types {
            block.type_lines = type_sigs
                .iter()
                .map(|sig| MetadataLine::with_style(sig.clone(), LineStyle::TypeSignature))
                .collect();
        }

        // Add TODO lines
        if let Some(todo_items) = todos {
            block.todo_lines = todo_items
                .iter()
                .map(|todo| {
                    let content =
                        format!("{}: {} (line {})", todo.marker_type, todo.text, todo.line);
                    MetadataLine::with_style(content, LineStyle::Todo)
                })
                .collect();
        }

        // Add import lines
        if let Some(file_imports) = imports {
            let summary = file_imports.summary();
            if !summary.is_empty() {
                block.import_lines = vec![MetadataLine::with_style(
                    format!("imports: {}", summary),
                    LineStyle::Import,
                )];
            }
        }

        if block.is_empty() { None } else { Some(block) }
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

    /// Write a rendered line with colors to stdout.
    fn write_rendered_line(
        &self,
        stdout: &mut StandardStream,
        line: &RenderedLine,
        cont_prefix: &str,
        meta_prefix: &str,
    ) -> io::Result<()> {
        match line {
            RenderedLine::Separator => {
                stdout.reset()?;
                writeln!(stdout, "{}", cont_prefix)?;
            }
            RenderedLine::Content {
                text,
                symbol_name,
                style,
                indent,
            } => {
                stdout.reset()?;
                write!(stdout, "{}{}", cont_prefix, meta_prefix)?;
                write_metadata_line_with_symbol(
                    stdout,
                    text,
                    symbol_name.as_deref(),
                    style.color(),
                    style.is_intense(),
                    *indent,
                )?;
                writeln!(stdout)?;
            }
        }
        Ok(())
    }

    /// Write inline content with colors (first line on same line as filename).
    fn write_inline_content(
        &self,
        stdout: &mut StandardStream,
        line: &RenderedLine,
        meta_prefix: &str,
    ) -> io::Result<()> {
        if let RenderedLine::Content {
            text,
            symbol_name,
            style,
            indent,
        } = line
        {
            write!(stdout, "  {}", meta_prefix)?;
            write_metadata_line_with_symbol(
                stdout,
                text,
                symbol_name.as_deref(),
                style.color(),
                style.is_intense(),
                *indent,
            )?;
        }
        writeln!(stdout)?;
        stdout.reset()?;
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
        let meta_prefix = self.config.metadata.prefix_str();
        let order = self.config.metadata.order;
        let show_full = self.config.show_full();

        let cont_prefix = continuation_prefix(prefix, is_last);
        let wrap_width = calculate_wrap_width(
            self.config.wrap_width,
            cont_prefix.chars().count(),
            meta_prefix.chars().count(),
        );

        let result = render_metadata_block(block, order, show_full, wrap_width);

        match result {
            MetadataRenderResult::Empty => {
                writeln!(stdout)?;
            }
            MetadataRenderResult::Inline { first } => {
                self.write_inline_content(stdout, &first, meta_prefix)?;
            }
            MetadataRenderResult::InlineWithBlock { first, block_lines } => {
                self.write_inline_content(stdout, &first, meta_prefix)?;
                for line in &block_lines {
                    self.write_rendered_line(stdout, line, &cont_prefix, meta_prefix)?;
                }
                stdout.reset()?;
            }
            MetadataRenderResult::Block { lines } => {
                writeln!(stdout)?; // End the filename line
                for line in &lines {
                    self.write_rendered_line(stdout, line, &cont_prefix, meta_prefix)?;
                }
                stdout.reset()?;
            }
        }
        Ok(())
    }

    /// Format a rendered line to plain text.
    fn format_rendered_line(
        &self,
        output: &mut String,
        line: &RenderedLine,
        cont_prefix: &str,
        meta_prefix: &str,
    ) {
        match line {
            RenderedLine::Separator => {
                output.push_str(cont_prefix);
                output.push('\n');
            }
            RenderedLine::Content { text, .. } => {
                output.push_str(cont_prefix);
                output.push_str(meta_prefix);
                output.push_str(text);
                output.push('\n');
            }
        }
    }

    /// Format inline content to plain text.
    fn format_inline_content(&self, output: &mut String, line: &RenderedLine, meta_prefix: &str) {
        if let RenderedLine::Content { text, .. } = line {
            output.push_str("  ");
            output.push_str(meta_prefix);
            output.push_str(text);
        }
        output.push('\n');
    }

    /// Format a metadata block to plain text output.
    fn format_metadata_block_plain(
        &self,
        output: &mut String,
        block: &MetadataBlock,
        prefix: &str,
        is_last: bool,
    ) {
        let meta_prefix = self.config.metadata.prefix_str();
        let order = self.config.metadata.order;
        let show_full = self.config.show_full();

        let cont_prefix = continuation_prefix(prefix, is_last);
        let wrap_width = calculate_wrap_width(
            self.config.wrap_width,
            cont_prefix.chars().count(),
            meta_prefix.chars().count(),
        );

        let result = render_metadata_block(block, order, show_full, wrap_width);

        match result {
            MetadataRenderResult::Empty => {
                output.push('\n');
            }
            MetadataRenderResult::Inline { first } => {
                self.format_inline_content(output, &first, meta_prefix);
            }
            MetadataRenderResult::InlineWithBlock { first, block_lines } => {
                self.format_inline_content(output, &first, meta_prefix);
                for line in &block_lines {
                    self.format_rendered_line(output, line, &cont_prefix, meta_prefix);
                }
            }
            MetadataRenderResult::Block { lines } => {
                output.push('\n'); // End the filename line
                for line in &lines {
                    self.format_rendered_line(output, line, &cont_prefix, meta_prefix);
                }
            }
        }
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
            TreeNode::File {
                name,
                comment,
                types,
                todos,
                imports,
                ..
            } => {
                output.push_str(prefix);
                output.push_str(connector);
                output.push_str(name);

                // Build metadata block from all available fields
                if let Some(block) = Self::build_metadata_block(
                    comment.as_ref(),
                    types.as_ref(),
                    todos.as_ref(),
                    imports.as_ref(),
                ) {
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
            TreeNode::File {
                name,
                comment,
                types,
                todos,
                imports,
                ..
            } => {
                write!(stdout, "{}{}", prefix, connector)?;
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
                write!(stdout, "{}", name)?;
                stdout.reset()?;

                // Build metadata block from all available fields
                if let Some(block) = Self::build_metadata_block(
                    comment.as_ref(),
                    types.as_ref(),
                    todos.as_ref(),
                    imports.as_ref(),
                ) {
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

    #[test]
    fn test_format_with_all_metadata() {
        use crate::imports::FileImports;
        use crate::tree::JsonTodoItem;

        let tree = TreeNode::Dir {
            name: ".".to_string(),
            path: PathBuf::from("."),
            children: vec![TreeNode::File {
                name: "app.rs".to_string(),
                path: PathBuf::from("app.rs"),
                comment: Some("Main application module".to_string()),
                types: Some(vec![
                    "pub fn main()".to_string(),
                    "pub struct App".to_string(),
                ]),
                todos: Some(vec![JsonTodoItem {
                    marker_type: "TODO".to_string(),
                    text: "Add error handling".to_string(),
                    line: 42,
                }]),
                imports: Some(FileImports {
                    std: vec!["std::io".to_string()],
                    external: vec!["serde".to_string()],
                    internal: vec![],
                }),
                size_bytes: None,
                size_human: None,
            }],
        };

        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: MetadataConfig::all(true, crate::metadata::MetadataOrder::CommentsFirst),
            wrap_width: None,
        });
        let output = formatter.format(&tree);

        // Verify all metadata types are present in output
        assert!(
            output.contains("Main application module"),
            "Should contain comment"
        );
        assert!(
            output.contains("pub fn main()"),
            "Should contain type signature"
        );
        assert!(
            output.contains("pub struct App"),
            "Should contain type signature"
        );
        assert!(
            output.contains("TODO: Add error handling (line 42)"),
            "Should contain TODO"
        );
        assert!(
            output.contains("imports: serde, std::{std::io}"),
            "Should contain import summary"
        );
    }
}
