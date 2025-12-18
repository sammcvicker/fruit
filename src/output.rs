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

/// Calculate the continuation prefix for lines below the filename.
/// Used by both TreeFormatter and StreamingFormatter.
fn continuation_prefix(prefix: &str, is_last: bool) -> String {
    if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    }
}

/// Calculate the available width for text wrapping after accounting for prefixes.
/// Returns None if wrapping is disabled or the available width is too small.
fn calculate_wrap_width(
    base_wrap_width: Option<usize>,
    continuation_prefix_len: usize,
    meta_prefix_len: usize,
) -> Option<usize> {
    base_wrap_width
        .map(|w| w.saturating_sub(continuation_prefix_len + meta_prefix_len))
        .filter(|&w| w > 10)
}

/// Check if the next non-empty line in a slice is indented relative to current indent.
fn has_indented_children(lines: &[&crate::metadata::MetadataLine], current_indent: usize) -> bool {
    lines
        .iter()
        .find(|l| !l.content.trim().is_empty())
        .is_some_and(|next| next.indent > current_indent)
}

/// Determine if a blank line should be inserted before a baseline item.
/// Returns true when returning to baseline after indented content or when
/// this baseline item has indented children (it's a group header).
fn should_insert_group_separator(
    current_indent: usize,
    prev_indent: Option<usize>,
    has_children: bool,
) -> bool {
    if current_indent != 0 {
        return false;
    }
    let dominated_previous = prev_indent.is_some_and(|p| p > 0);
    (dominated_previous || has_children) && prev_indent.is_some()
}

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
        let line_refs: Vec<_> = lines.iter().collect();
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

        // Metadata lines
        for line in &lines {
            let content = line.content.trim();

            // Empty line is a separator
            if content.is_empty() {
                output.push_str(&cont_prefix);
                output.push('\n');
                continue;
            }

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

    /// Write inline metadata (first line only, on same line as filename).
    fn write_inline_metadata(
        &mut self,
        meta_prefix: &str,
        first: &crate::metadata::MetadataLine,
    ) -> io::Result<()> {
        write!(self.stdout, "  {}", meta_prefix)?;
        write_metadata_line_with_symbol(
            &mut self.stdout,
            first_line(&first.content),
            first.symbol_name.as_deref(),
            first.style.color(),
            first.style.is_intense(),
            first.indent,
        )?;
        writeln!(self.stdout)?;
        self.stdout.reset()?;
        Ok(())
    }

    /// Print metadata lines in a block format with proper indentation and group separators.
    fn print_metadata_lines_block(
        &mut self,
        lines: &[&crate::metadata::MetadataLine],
        cont_prefix: &str,
        meta_prefix: &str,
        wrap_width: Option<usize>,
    ) -> io::Result<()> {
        // Blank line before block
        self.stdout.reset()?;
        writeln!(self.stdout, "{}", cont_prefix)?;

        let mut prev_indent: Option<usize> = None;
        for (i, meta_line) in lines.iter().enumerate() {
            let content = meta_line.content.trim();

            // Empty line is a separator between sections
            if content.is_empty() {
                self.stdout.reset()?;
                writeln!(self.stdout, "{}", cont_prefix)?;
                prev_indent = None;
                continue;
            }

            // Check if we should insert a group separator
            let has_children = has_indented_children(&lines[i + 1..], meta_line.indent);
            if should_insert_group_separator(meta_line.indent, prev_indent, has_children) {
                self.stdout.reset()?;
                writeln!(self.stdout, "{}", cont_prefix)?;
            }
            prev_indent = Some(meta_line.indent);

            let wrapped = if let Some(width) = wrap_width {
                wrap_text(content, width)
            } else {
                vec![content.to_string()]
            };

            for wrapped_line in wrapped.iter() {
                self.stdout.reset()?;
                write!(self.stdout, "{}{}", cont_prefix, meta_prefix)?;
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
        writeln!(self.stdout, "{}", cont_prefix)?;
        Ok(())
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

        // Copy config values to avoid borrow conflicts
        let meta_prefix = self.config.metadata.prefix_str().to_string();
        let order = self.config.metadata.order;
        let base_wrap_width = self.config.wrap_width;
        let show_full = self.config.show_full();

        // Check if the first section (based on order) is a single line
        let first_is_single = block.first_section_is_single_line(order);
        let total_lines = block.total_lines();

        // Not in full mode: show first line inline only
        if !show_full {
            if let Some(first) = block.first_line(order) {
                self.write_inline_metadata(&meta_prefix, first)?;
            } else {
                writeln!(self.stdout)?;
            }
            return Ok(());
        }

        // Full mode: show all metadata
        let lines = block.lines_in_order(order);

        // If total is just 1 line, show inline
        if total_lines == 1 {
            if let Some(first) = block.first_line(order) {
                self.write_inline_metadata(&meta_prefix, first)?;
            } else {
                writeln!(self.stdout)?;
            }
            return Ok(());
        }

        let cont_prefix = continuation_prefix(prefix, is_last);
        let wrap_width = calculate_wrap_width(
            base_wrap_width,
            cont_prefix.chars().count(),
            meta_prefix.chars().count(),
        );

        // If first section is single line and there's more content, show first inline then rest below
        if first_is_single {
            if let Some(first) = block.first_line(order) {
                self.write_inline_metadata(&meta_prefix, first)?;
            }

            // Skip the first line (already shown inline) and the separator after it
            let skip_count = if block.has_both() { 2 } else { 1 };
            let remaining_lines: Vec<_> = lines.iter().skip(skip_count).collect();
            self.print_metadata_lines_block(
                &remaining_lines,
                &cont_prefix,
                &meta_prefix,
                wrap_width,
            )?;
        } else {
            // First section has multiple lines, show everything below
            writeln!(self.stdout)?; // End the filename line

            let line_refs: Vec<_> = lines.iter().collect();
            self.print_metadata_lines_block(&line_refs, &cont_prefix, &meta_prefix, wrap_width)?;
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

    // ==================== Metadata Block Display Tests ====================

    #[test]
    fn test_metadata_block_inline_display_single_line() {
        // When not in full mode, only the first line should show inline
        let tree = TreeNode::File {
            name: "test.rs".to_string(),
            path: "test.rs".into(),
            comment: Some("Single line comment".to_string()),
            types: None,
        };

        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(false), // full = false
            wrap_width: None,
        });

        // Wrap in a directory so format_node runs properly
        let root = TreeNode::Dir {
            name: ".".to_string(),
            path: ".".into(),
            children: vec![tree],
        };
        let output = formatter.format(&root);

        // Should have comment inline on same line as filename
        assert!(
            output.contains("test.rs  Single line comment"),
            "Expected inline metadata, got: {}",
            output
        );
    }

    #[test]
    fn test_metadata_block_inline_display_multiline_comment_first_only() {
        // When not in full mode, multiline comments should only show first line
        let tree = TreeNode::File {
            name: "test.rs".to_string(),
            path: "test.rs".into(),
            comment: Some("First line\nSecond line\nThird line".to_string()),
            types: None,
        };

        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(false), // full = false
            wrap_width: None,
        });

        let root = TreeNode::Dir {
            name: ".".to_string(),
            path: ".".into(),
            children: vec![tree],
        };
        let output = formatter.format(&root);

        // Should only have first line inline
        assert!(
            output.contains("test.rs  First line"),
            "Expected first line inline, got: {}",
            output
        );
        // Should NOT contain other lines
        assert!(
            !output.contains("Second line"),
            "Should not contain second line, got: {}",
            output
        );
    }

    #[test]
    fn test_metadata_block_multiline_display() {
        // When in full mode, all lines should appear in block format
        let tree = TreeNode::File {
            name: "test.rs".to_string(),
            path: "test.rs".into(),
            comment: Some("First line\nSecond line\nThird line".to_string()),
            types: None,
        };

        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(true), // full = true
            wrap_width: None,
        });

        let root = TreeNode::Dir {
            name: ".".to_string(),
            path: ".".into(),
            children: vec![tree],
        };
        let output = formatter.format(&root);

        // Should have all lines in output
        assert!(
            output.contains("First line"),
            "Expected first line, got: {}",
            output
        );
        assert!(
            output.contains("Second line"),
            "Expected second line, got: {}",
            output
        );
        assert!(
            output.contains("Third line"),
            "Expected third line, got: {}",
            output
        );
    }

    #[test]
    fn test_metadata_with_custom_prefix() {
        // Test that custom prefix is applied to metadata lines
        let tree = TreeNode::File {
            name: "test.rs".to_string(),
            path: "test.rs".into(),
            comment: Some("Comment text".to_string()),
            types: None,
        };

        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(false).with_prefix("# "),
            wrap_width: None,
        });

        let root = TreeNode::Dir {
            name: ".".to_string(),
            path: ".".into(),
            children: vec![tree],
        };
        let output = formatter.format(&root);

        // Should have prefix before comment
        assert!(
            output.contains("# Comment text"),
            "Expected prefix '# ' before comment, got: {}",
            output
        );
    }

    #[test]
    fn test_metadata_block_empty_displays_no_extra_content() {
        // When comment is None, no metadata should appear
        let tree = TreeNode::File {
            name: "test.rs".to_string(),
            path: "test.rs".into(),
            comment: None,
            types: None,
        };

        let formatter = TreeFormatter::new(OutputConfig {
            use_color: false,
            metadata: MetadataConfig::comments_only(true),
            wrap_width: None,
        });

        let root = TreeNode::Dir {
            name: ".".to_string(),
            path: ".".into(),
            children: vec![tree],
        };
        let output = formatter.format(&root);

        // Line should just be the filename
        let lines: Vec<&str> = output.lines().collect();
        // Find the line with test.rs
        let test_line = lines.iter().find(|l| l.contains("test.rs")).unwrap();
        // Should end with "test.rs" (possibly with tree connector)
        assert!(
            test_line.trim().ends_with("test.rs"),
            "Expected line to end with just filename, got: {}",
            test_line
        );
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_continuation_prefix_last_item() {
        let prefix = continuation_prefix("", true);
        assert_eq!(prefix, "    "); // 4 spaces for last item

        let prefix = continuation_prefix("│   ", true);
        assert_eq!(prefix, "│       "); // parent prefix + 4 spaces
    }

    #[test]
    fn test_continuation_prefix_not_last_item() {
        let prefix = continuation_prefix("", false);
        assert_eq!(prefix, "│   "); // vertical line + 3 spaces

        let prefix = continuation_prefix("│   ", false);
        assert_eq!(prefix, "│   │   "); // parent prefix + vertical + 3 spaces
    }

    #[test]
    fn test_calculate_wrap_width_enabled() {
        // With base width of 100, should subtract prefixes
        let width = calculate_wrap_width(Some(100), 4, 2);
        assert_eq!(width, Some(94)); // 100 - 4 - 2 = 94
    }

    #[test]
    fn test_calculate_wrap_width_disabled() {
        // When wrap_width is None, should return None
        let width = calculate_wrap_width(None, 4, 2);
        assert!(width.is_none());
    }

    #[test]
    fn test_calculate_wrap_width_too_small() {
        // When resulting width is <= 10, should return None
        let width = calculate_wrap_width(Some(15), 10, 4);
        assert!(width.is_none()); // 15 - 10 - 4 = 1, which is <= 10
    }

    #[test]
    fn test_first_line_extracts_first() {
        assert_eq!(first_line("first\nsecond\nthird"), "first");
        assert_eq!(first_line("only one"), "only one");
        assert_eq!(first_line(""), "");
    }

    #[test]
    fn test_wrap_text_empty() {
        let wrapped = wrap_text("", 50);
        assert_eq!(wrapped, vec![""]);
    }

    #[test]
    fn test_wrap_text_zero_width() {
        // Zero width should return original text
        let wrapped = wrap_text("some text", 0);
        assert_eq!(wrapped, vec!["some text"]);
    }

    #[test]
    fn test_wrap_text_long_word() {
        // Word longer than max_width should be character-wrapped
        let wrapped = wrap_text("supercalifragilisticexpialidocious", 10);
        assert_eq!(wrapped.len(), 4);
        assert_eq!(wrapped[0], "supercalif");
        assert_eq!(wrapped[1], "ragilistic");
        assert_eq!(wrapped[2], "expialidoc");
        assert_eq!(wrapped[3], "ious");
    }

    #[test]
    fn test_wrap_text_preserves_word_boundaries() {
        let wrapped = wrap_text("hello world foo bar", 11);
        assert_eq!(wrapped, vec!["hello world", "foo bar"]);
    }
}
