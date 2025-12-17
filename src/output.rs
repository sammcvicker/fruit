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
        let is_single_line = block.lines.len() == 1;

        // Single-line metadata always displays inline, regardless of full mode
        if is_single_line {
            if let Some(first) = block.lines.first() {
                stdout.set_color(
                    ColorSpec::new()
                        .set_fg(Some(first.style.color()))
                        .set_intense(first.style.is_intense()),
                )?;
                write!(stdout, "  {}{}", meta_prefix, first_line(&first.content))?;
            }
            writeln!(stdout)?;
            stdout.reset()?;
            return Ok(());
        }

        // Multi-line metadata
        if self.config.show_full() {
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
            for meta_line in &block.lines {
                let content = meta_line.content.trim();
                let wrapped = if let Some(width) = wrap_width {
                    wrap_text(content, width)
                } else {
                    vec![content.to_string()]
                };

                for wrapped_line in wrapped.iter() {
                    stdout.reset()?;
                    write!(stdout, "{}{}", continuation_prefix, meta_prefix)?;
                    stdout.set_color(
                        ColorSpec::new()
                            .set_fg(Some(meta_line.style.color()))
                            .set_intense(meta_line.style.is_intense()),
                    )?;
                    writeln!(stdout, "{}", wrapped_line)?;
                }
            }

            // Blank line after block
            stdout.reset()?;
            writeln!(stdout, "{}", continuation_prefix)?;
        } else {
            // Not in full mode: show first line inline
            if let Some(first) = block.lines.first() {
                stdout.set_color(
                    ColorSpec::new()
                        .set_fg(Some(first.style.color()))
                        .set_intense(first.style.is_intense()),
                )?;
                write!(stdout, "  {}{}", meta_prefix, first_line(&first.content))?;
            }
            writeln!(stdout)?;
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
        let is_single_line = block.lines.len() == 1;

        // Single-line metadata always displays inline, regardless of full mode
        if is_single_line {
            if let Some(first) = block.lines.first() {
                output.push_str("  ");
                output.push_str(meta_prefix);
                output.push_str(first_line(&first.content));
            }
            output.push('\n');
            return;
        }

        // Multi-line metadata
        if self.config.show_full() {
            // Full metadata mode: display in a block beneath filename
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
            for line in &block.lines {
                let content = line.content.trim();
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
        } else {
            // Not in full mode: show first line inline
            if let Some(first) = block.lines.first() {
                output.push_str("  ");
                output.push_str(meta_prefix);
                output.push_str(first_line(&first.content));
            }
            output.push('\n');
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
            TreeNode::File { name, comment, .. } => {
                output.push_str(prefix);
                output.push_str(connector);
                output.push_str(name);
                if let Some(c) = comment {
                    // Convert comment to metadata block for unified handling
                    let block = MetadataBlock::from_text("comments", c);
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
                    let block = MetadataBlock::from_text("comments", c);
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
        let is_single_line = block.lines.len() == 1;

        // Single-line metadata always displays inline, regardless of full mode
        if is_single_line {
            if let Some(first) = block.lines.first() {
                self.stdout.set_color(
                    ColorSpec::new()
                        .set_fg(Some(first.style.color()))
                        .set_intense(first.style.is_intense()),
                )?;
                write!(
                    self.stdout,
                    "  {}{}",
                    meta_prefix,
                    first_line(&first.content)
                )?;
            }
            writeln!(self.stdout)?;
            self.stdout.reset()?;
            return Ok(());
        }

        // Multi-line metadata
        if self.config.show_full() {
            // Full metadata mode: display in a block beneath filename
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
            for meta_line in &block.lines {
                let content = meta_line.content.trim();
                let wrapped = if let Some(width) = wrap_width {
                    wrap_text(content, width)
                } else {
                    vec![content.to_string()]
                };

                for wrapped_line in wrapped.iter() {
                    self.stdout.reset()?;
                    write!(self.stdout, "{}{}", continuation_prefix, meta_prefix)?;
                    self.stdout.set_color(
                        ColorSpec::new()
                            .set_fg(Some(meta_line.style.color()))
                            .set_intense(meta_line.style.is_intense()),
                    )?;
                    writeln!(self.stdout, "{}", wrapped_line)?;
                }
            }

            // Blank line after block
            self.stdout.reset()?;
            writeln!(self.stdout, "{}", continuation_prefix)?;
        } else {
            // Not in full mode: show first line inline
            if let Some(first) = block.lines.first() {
                self.stdout.set_color(
                    ColorSpec::new()
                        .set_fg(Some(first.style.color()))
                        .set_intense(first.style.is_intense()),
                )?;
                write!(
                    self.stdout,
                    "  {}{}",
                    meta_prefix,
                    first_line(&first.content)
                )?;
            }
            writeln!(self.stdout)?;
        }
        self.stdout.reset()?;
        Ok(())
    }
}

impl StreamingOutput for StreamingFormatter {
    fn output_node(
        &mut self,
        name: &str,
        comment: Option<&str>,
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

            if let Some(c) = comment {
                // Convert comment to metadata block for unified handling
                let block = MetadataBlock::from_text("comments", c);
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
                },
                TreeNode::Dir {
                    name: "src".to_string(),
                    path: "src".into(),
                    children: vec![
                        TreeNode::File {
                            name: "main.rs".to_string(),
                            path: "src/main.rs".into(),
                            comment: Some("CLI entry point".to_string()),
                        },
                        TreeNode::File {
                            name: "lib.rs".to_string(),
                            path: "src/lib.rs".into(),
                            comment: None,
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
