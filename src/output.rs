//! Tree formatting and display

use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::metadata::{MetadataBlock, MetadataConfig};
use crate::tree::{StreamingOutput, TreeNode};

/// Print tree node as pretty-printed JSON to stdout.
pub fn print_json(node: &TreeNode) -> io::Result<()> {
    let json =
        serde_json::to_string_pretty(node).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    println!("{}", json);
    Ok(())
}

const DEFAULT_WRAP_WIDTH: usize = 100;

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub use_color: bool,
    /// Metadata display configuration
    pub metadata: MetadataConfig,
    pub wrap_width: Option<usize>,
}

impl OutputConfig {
    /// Check if full metadata blocks should be shown (vs first line only).
    pub fn show_full(&self) -> bool {
        self.metadata.full
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            use_color: true,
            metadata: MetadataConfig::comments_only(false),
            wrap_width: Some(DEFAULT_WRAP_WIDTH),
        }
    }
}

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

        // Full mode: multi-line metadata
        {
            // Full metadata mode: display in a block beneath filename
            writeln!(stdout)?; // End the filename line

            // Continuation prefix for lines below the filename
            let continuation_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            // Calculate available width for text wrapping
            let prefix_width = continuation_prefix.chars().count() + meta_prefix.chars().count();
            let wrap_width = self
                .config
                .wrap_width
                .map(|w| w.saturating_sub(prefix_width))
                .filter(|&w| w > 10);

            // Blank line before block
            stdout.reset()?;
            writeln!(stdout, "{}", continuation_prefix)?;

            // Metadata lines with per-line styling
            let mut prev_indent: Option<usize> = None;
            for (i, meta_line) in lines.iter().enumerate() {
                let content = meta_line.content.trim();

                // Empty line is a separator
                if content.is_empty() {
                    stdout.reset()?;
                    writeln!(stdout, "{}", continuation_prefix)?;
                    prev_indent = None; // Reset indent tracking after separator
                    continue;
                }

                // Check if next non-empty line is indented (this item has children)
                let has_indented_children = lines[i + 1..]
                    .iter()
                    .find(|l| !l.content.trim().is_empty())
                    .is_some_and(|next| next.indent > meta_line.indent);

                // Add blank line before baseline items that start a new "group":
                // - When returning to baseline after indented content
                // - When this baseline item has indented children (it's a group header)
                let dominated_previous = prev_indent.is_some_and(|p| p > 0);
                if meta_line.indent == 0 && (dominated_previous || has_indented_children) {
                    // But not before the very first item
                    if prev_indent.is_some() {
                        stdout.reset()?;
                        writeln!(stdout, "{}", continuation_prefix)?;
                    }
                }
                prev_indent = Some(meta_line.indent);

                let wrapped = if let Some(width) = wrap_width {
                    wrap_text(content, width)
                } else {
                    vec![content.to_string()]
                };

                for wrapped_line in wrapped.iter() {
                    stdout.reset()?;
                    write!(stdout, "{}{}", continuation_prefix, meta_prefix)?;
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
            writeln!(stdout, "{}", continuation_prefix)?;
        }
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

        // Continuation prefix for lines below the filename
        let continuation_prefix = if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };

        // Calculate available width for text wrapping
        let prefix_width = continuation_prefix.chars().count() + meta_prefix.chars().count();
        let wrap_width = self
            .config
            .wrap_width
            .map(|w| w.saturating_sub(prefix_width))
            .filter(|&w| w > 10);

        // Blank line before block
        output.push_str(&continuation_prefix);
        output.push('\n');

        // Metadata lines
        for line in &lines {
            let content = line.content.trim();

            // Empty line is a separator
            if content.is_empty() {
                output.push_str(&continuation_prefix);
                output.push('\n');
                continue;
            }

            let wrapped = if let Some(width) = wrap_width {
                wrap_text(content, width)
            } else {
                vec![content.to_string()]
            };

            for wrapped_line in wrapped.iter() {
                output.push_str(&continuation_prefix);
                output.push_str(meta_prefix);
                output.push_str(wrapped_line);
                output.push('\n');
            }
        }

        // Blank line after block
        output.push_str(&continuation_prefix);
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

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

/// Write a metadata line, highlighting the symbol name in bold red if present.
/// The `indent` parameter specifies the number of spaces to prepend for hierarchy display.
fn write_metadata_line_with_symbol(
    stdout: &mut StandardStream,
    content: &str,
    symbol_name: Option<&str>,
    base_color: Color,
    is_intense: bool,
    indent: usize,
) -> io::Result<()> {
    // Write indentation spaces
    if indent > 0 {
        write!(stdout, "{:indent$}", "", indent = indent)?;
    }

    if let Some(sym) = symbol_name {
        // Find the symbol in the content and highlight it
        if let Some(pos) = content.find(sym) {
            // Write part before symbol
            let before = &content[..pos];
            if !before.is_empty() {
                stdout.set_color(
                    ColorSpec::new()
                        .set_fg(Some(base_color))
                        .set_intense(is_intense),
                )?;
                write!(stdout, "{}", before)?;
            }

            // Write symbol in bold red
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
            write!(stdout, "{}", sym)?;

            // Write part after symbol
            let after = &content[pos + sym.len()..];
            if !after.is_empty() {
                stdout.set_color(
                    ColorSpec::new()
                        .set_fg(Some(base_color))
                        .set_intense(is_intense),
                )?;
                write!(stdout, "{}", after)?;
            }
        } else {
            // Symbol not found in content, just write normally
            stdout.set_color(
                ColorSpec::new()
                    .set_fg(Some(base_color))
                    .set_intense(is_intense),
            )?;
            write!(stdout, "{}", content)?;
        }
    } else {
        // No symbol to highlight
        stdout.set_color(
            ColorSpec::new()
                .set_fg(Some(base_color))
                .set_intense(is_intense),
        )?;
        write!(stdout, "{}", content)?;
    }
    Ok(())
}

/// Streaming output formatter - outputs directly to stdout without buffering.
/// Implements the StreamingOutput trait for use with StreamingWalker.
pub struct StreamingFormatter {
    config: OutputConfig,
    stdout: StandardStream,
}

impl StreamingFormatter {
    pub fn new(config: OutputConfig) -> Self {
        let choice = if config.use_color {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };
        Self {
            config,
            stdout: StandardStream::stdout(choice),
        }
    }

    /// Print a metadata block with colors to stdout.
    fn print_metadata_block(
        &mut self,
        block: &MetadataBlock,
        prefix: &str,
        is_last: bool,
    ) -> io::Result<()> {
        if block.is_empty() {
            writeln!(self.stdout)?;
            return Ok(());
        }

        let meta_prefix = self.config.metadata.prefix_str();
        let order = self.config.metadata.order;

        // Check if the first section (based on order) is a single line
        let first_is_single = block.first_section_is_single_line(order);
        let total_lines = block.total_lines();

        // Not in full mode: show first line inline only
        if !self.config.show_full() {
            if let Some(first) = block.first_line(order) {
                write!(self.stdout, "  {}", meta_prefix)?;
                write_metadata_line_with_symbol(
                    &mut self.stdout,
                    first_line(&first.content),
                    first.symbol_name.as_deref(),
                    first.style.color(),
                    first.style.is_intense(),
                    first.indent,
                )?;
            }
            writeln!(self.stdout)?;
            self.stdout.reset()?;
            return Ok(());
        }

        // Full mode: show all metadata
        // Get lines in the configured order (with separator if both types present)
        let lines = block.lines_in_order(order);

        // If total is just 1 line, show inline
        if total_lines == 1 {
            if let Some(first) = block.first_line(order) {
                write!(self.stdout, "  {}", meta_prefix)?;
                write_metadata_line_with_symbol(
                    &mut self.stdout,
                    first_line(&first.content),
                    first.symbol_name.as_deref(),
                    first.style.color(),
                    first.style.is_intense(),
                    first.indent,
                )?;
            }
            writeln!(self.stdout)?;
            self.stdout.reset()?;
            return Ok(());
        }

        // If first section is single line and there's more content, show first inline then rest below
        if first_is_single {
            if let Some(first) = block.first_line(order) {
                write!(self.stdout, "  {}", meta_prefix)?;
                write_metadata_line_with_symbol(
                    &mut self.stdout,
                    first_line(&first.content),
                    first.symbol_name.as_deref(),
                    first.style.color(),
                    first.style.is_intense(),
                    first.indent,
                )?;
            }
            writeln!(self.stdout)?;
            self.stdout.reset()?;

            // Continuation prefix for lines below the filename
            let continuation_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            // Calculate available width for text wrapping
            let prefix_width = continuation_prefix.chars().count() + meta_prefix.chars().count();
            let wrap_width = self
                .config
                .wrap_width
                .map(|w| w.saturating_sub(prefix_width))
                .filter(|&w| w > 10);

            // Skip the first line (already shown inline) and the separator after it
            let skip_count = if block.has_both() { 2 } else { 1 };
            let remaining_lines: Vec<_> = lines.iter().skip(skip_count).collect();

            // Blank line before remaining content
            self.stdout.reset()?;
            writeln!(self.stdout, "{}", continuation_prefix)?;

            let mut prev_indent: Option<usize> = None;
            for (i, meta_line) in remaining_lines.iter().enumerate() {
                let content = meta_line.content.trim();

                // Empty line is a separator between sections
                if content.is_empty() {
                    self.stdout.reset()?;
                    writeln!(self.stdout, "{}", continuation_prefix)?;
                    prev_indent = None;
                    continue;
                }

                // Check if next non-empty line is indented (this item has children)
                let has_indented_children = remaining_lines[i + 1..]
                    .iter()
                    .find(|l| !l.content.trim().is_empty())
                    .is_some_and(|next| next.indent > meta_line.indent);

                // Add blank line before baseline items that start a new "group"
                let dominated_previous = prev_indent.is_some_and(|p| p > 0);
                if meta_line.indent == 0 && (dominated_previous || has_indented_children) {
                    if prev_indent.is_some() {
                        self.stdout.reset()?;
                        writeln!(self.stdout, "{}", continuation_prefix)?;
                    }
                }
                prev_indent = Some(meta_line.indent);

                let wrapped = if let Some(width) = wrap_width {
                    wrap_text(content, width)
                } else {
                    vec![content.to_string()]
                };

                for wrapped_line in wrapped.iter() {
                    self.stdout.reset()?;
                    write!(self.stdout, "{}{}", continuation_prefix, meta_prefix)?;
                    write_metadata_line_with_symbol(
                        &mut self.stdout,
                        wrapped_line,
                        meta_line.symbol_name.as_deref(),
                        meta_line.style.color(),
                        meta_line.style.is_intense(),
                        meta_line.indent,
                    )?;
                    writeln!(self.stdout)?;
                }
            }

            // Blank line after block
            self.stdout.reset()?;
            writeln!(self.stdout, "{}", continuation_prefix)?;
        } else {
            // First section has multiple lines, show everything below
            writeln!(self.stdout)?; // End the filename line

            // Continuation prefix for lines below the filename
            let continuation_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            // Calculate available width for text wrapping
            let prefix_width = continuation_prefix.chars().count() + meta_prefix.chars().count();
            let wrap_width = self
                .config
                .wrap_width
                .map(|w| w.saturating_sub(prefix_width))
                .filter(|&w| w > 10);

            // Blank line before block
            self.stdout.reset()?;
            writeln!(self.stdout, "{}", continuation_prefix)?;

            // Metadata lines with per-line styling
            let mut prev_indent: Option<usize> = None;
            for (i, meta_line) in lines.iter().enumerate() {
                let content = meta_line.content.trim();

                // Empty line is a separator between sections
                if content.is_empty() {
                    self.stdout.reset()?;
                    writeln!(self.stdout, "{}", continuation_prefix)?;
                    prev_indent = None;
                    continue;
                }

                // Check if next non-empty line is indented (this item has children)
                let has_indented_children = lines[i + 1..]
                    .iter()
                    .find(|l| !l.content.trim().is_empty())
                    .is_some_and(|next| next.indent > meta_line.indent);

                // Add blank line before baseline items that start a new "group"
                let dominated_previous = prev_indent.is_some_and(|p| p > 0);
                if meta_line.indent == 0 && (dominated_previous || has_indented_children) {
                    if prev_indent.is_some() {
                        self.stdout.reset()?;
                        writeln!(self.stdout, "{}", continuation_prefix)?;
                    }
                }
                prev_indent = Some(meta_line.indent);

                let wrapped = if let Some(width) = wrap_width {
                    wrap_text(content, width)
                } else {
                    vec![content.to_string()]
                };

                for wrapped_line in wrapped.iter() {
                    self.stdout.reset()?;
                    write!(self.stdout, "{}{}", continuation_prefix, meta_prefix)?;
                    write_metadata_line_with_symbol(
                        &mut self.stdout,
                        wrapped_line,
                        meta_line.symbol_name.as_deref(),
                        meta_line.style.color(),
                        meta_line.style.is_intense(),
                        meta_line.indent,
                    )?;
                    writeln!(self.stdout)?;
                }
            }

            // Blank line after block
            self.stdout.reset()?;
            writeln!(self.stdout, "{}", continuation_prefix)?;
        }
        self.stdout.reset()?;
        Ok(())
    }
}

impl StreamingOutput for StreamingFormatter {
    fn output_node(
        &mut self,
        name: &str,
        metadata: Option<MetadataBlock>,
        is_dir: bool,
        is_last: bool,
        prefix: &str,
        is_root: bool,
    ) -> io::Result<()> {
        let connector = if is_last { "└── " } else { "├── " };

        if is_dir {
            if is_root {
                self.stdout
                    .set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
                writeln!(self.stdout, "{}", name)?;
                self.stdout.reset()?;
            } else {
                write!(self.stdout, "{}{}", prefix, connector)?;
                self.stdout
                    .set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
                writeln!(self.stdout, "{}", name)?;
                self.stdout.reset()?;
            }
        } else {
            // File
            write!(self.stdout, "{}{}", prefix, connector)?;
            self.stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
            write!(self.stdout, "{}", name)?;
            self.stdout.reset()?;

            if let Some(block) = metadata {
                self.print_metadata_block(&block, prefix, is_last)?;
            } else {
                writeln!(self.stdout)?;
            }
        }
        Ok(())
    }

    fn finish(&mut self, dir_count: usize, file_count: usize) -> io::Result<()> {
        writeln!(self.stdout)?;
        writeln!(
            self.stdout,
            "{} directories, {} files",
            dir_count, file_count
        )?;
        Ok(())
    }
}

/// Wrap text to fit within max_width, preferring word boundaries.
/// Uses character count (not byte count) to properly handle UTF-8.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_len = 0; // Character count of current_line

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        if current_line.is_empty() {
            // First word on line - may need character wrap if too long
            if word_len > max_width {
                // Character wrap for very long words
                let mut chars = word.chars().peekable();
                while chars.peek().is_some() {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    let chunk_len = chunk.chars().count();
                    if chars.peek().is_some() {
                        lines.push(chunk);
                    } else {
                        current_line = chunk;
                        current_len = chunk_len;
                    }
                }
            } else {
                current_line = word.to_string();
                current_len = word_len;
            }
        } else if current_len + 1 + word_len <= max_width {
            // Word fits on current line
            current_line.push(' ');
            current_line.push_str(word);
            current_len += 1 + word_len;
        } else {
            // Start new line
            lines.push(std::mem::take(&mut current_line));
            current_len = 0;
            // Handle long words
            if word_len > max_width {
                let mut chars = word.chars().peekable();
                while chars.peek().is_some() {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    let chunk_len = chunk.chars().count();
                    if chars.peek().is_some() {
                        lines.push(chunk);
                    } else {
                        current_line = chunk;
                        current_len = chunk_len;
                    }
                }
            } else {
                current_line = word.to_string();
                current_len = word_len;
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> TreeNode {
        TreeNode::Dir {
            name: ".".to_string(),
            path: ".".into(),
            children: vec![
                TreeNode::File {
                    name: "Cargo.toml".to_string(),
                    path: "Cargo.toml".into(),
                    comment: Some("Package manifest".to_string()),
                    types: None,
                },
                TreeNode::Dir {
                    name: "src".to_string(),
                    path: "src".into(),
                    children: vec![
                        TreeNode::File {
                            name: "main.rs".to_string(),
                            path: "src/main.rs".into(),
                            comment: Some("CLI entry point".to_string()),
                            types: None,
                        },
                        TreeNode::File {
                            name: "lib.rs".to_string(),
                            path: "src/lib.rs".into(),
                            comment: None,
                            types: None,
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
    fn test_wrap_text_utf8() {
        // Test that emoji don't cause panics (they're 4 bytes each)
        let emoji_text = "🎉🎊🎁🎂🎃";
        let wrapped = wrap_text(emoji_text, 3);
        assert_eq!(wrapped, vec!["🎉🎊🎁", "🎂🎃"]);

        // Test CJK characters (3 bytes each)
        let cjk_text = "你好世界";
        let wrapped = wrap_text(cjk_text, 2);
        assert_eq!(wrapped, vec!["你好", "世界"]);

        // Test mixed content
        let mixed = "Hello 世界 🎉";
        let wrapped = wrap_text(mixed, 8);
        assert_eq!(wrapped, vec!["Hello 世界", "🎉"]);
    }
}
