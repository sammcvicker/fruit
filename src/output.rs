//! Tree formatting and display

use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::tree::TreeNode;

/// Print tree node as pretty-printed JSON to stdout.
pub fn print_json(node: &TreeNode) -> io::Result<()> {
    let json = serde_json::to_string_pretty(node)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    println!("{}", json);
    Ok(())
}

const DEFAULT_WRAP_WIDTH: usize = 100;

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub use_color: bool,
    pub show_full_comment: bool,
    pub wrap_width: Option<usize>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            use_color: true,
            show_full_comment: false,
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
                    if self.config.show_full_comment {
                        // Calculate padding for continuation lines
                        let continuation_prefix = if is_last {
                            format!("{}    ", prefix)
                        } else {
                            format!("{}│   ", prefix)
                        };
                        let padding_len = name.len() + 4; // "  # " align with text start
                        let padding = " ".repeat(padding_len);

                        // Calculate available width for text wrapping
                        let prefix_width = continuation_prefix.chars().count() + padding_len;
                        let wrap_width = self.config.wrap_width
                            .map(|w| w.saturating_sub(prefix_width))
                            .filter(|&w| w > 10);

                        let comment = c.trim();
                        let has_multiple_lines = comment.contains('\n');

                        let mut first_line_done = false;
                        for line in comment.lines() {
                            let wrapped = if let Some(width) = wrap_width {
                                wrap_text(line, width)
                            } else {
                                vec![line.to_string()]
                            };

                            for (i, wrapped_line) in wrapped.iter().enumerate() {
                                if !first_line_done && i == 0 {
                                    output.push_str("  # ");
                                    output.push_str(wrapped_line);
                                    output.push('\n');
                                    first_line_done = true;
                                } else {
                                    output.push_str(&continuation_prefix);
                                    output.push_str(&padding);
                                    output.push_str(wrapped_line);
                                    output.push('\n');
                                }
                            }
                        }

                        // Add blank line after multiline comments
                        if has_multiple_lines {
                            output.push_str(&continuation_prefix);
                            output.push('\n');
                        }
                    } else {
                        output.push_str("  # ");
                        output.push_str(first_line(c));
                        output.push('\n');
                    }
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
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
                    if self.config.show_full_comment {
                        // Calculate padding for continuation lines
                        let continuation_prefix = if is_last {
                            format!("{}    ", prefix)
                        } else {
                            format!("{}│   ", prefix)
                        };
                        let padding_len = name.len() + 4; // "  # " align with text start
                        let padding = " ".repeat(padding_len);

                        // Calculate available width for text wrapping
                        let prefix_width = continuation_prefix.chars().count() + padding_len;
                        let wrap_width = self.config.wrap_width
                            .map(|w| w.saturating_sub(prefix_width))
                            .filter(|&w| w > 10); // Don't wrap if too narrow

                        let comment = c.trim();
                        let has_multiple_lines = comment.contains('\n');

                        let mut first_line_done = false;
                        for line in comment.lines() {
                            let wrapped = if let Some(width) = wrap_width {
                                wrap_text(line, width)
                            } else {
                                vec![line.to_string()]
                            };

                            for (i, wrapped_line) in wrapped.iter().enumerate() {
                                if !first_line_done && i == 0 {
                                    writeln!(stdout, "  # {}", wrapped_line)?;
                                    first_line_done = true;
                                } else {
                                    stdout.reset()?;
                                    write!(stdout, "{}", continuation_prefix)?;
                                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Black)).set_intense(true))?;
                                    writeln!(stdout, "{}{}", padding, wrapped_line)?;
                                }
                            }
                        }

                        // Add blank line after multiline comments for readability
                        if has_multiple_lines {
                            stdout.reset()?;
                            writeln!(stdout, "{}", continuation_prefix)?;
                        }
                    } else {
                        writeln!(stdout, "  # {}", first_line(c))?;
                    }
                    stdout.reset()?;
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
                    let (d, f) = self.print_node(child, stdout, &new_prefix, child_is_last, false)?;
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
            show_full_comment: false,
            wrap_width: None,
        });
        let output = formatter.format(&tree);

        assert!(output.contains("."));
        assert!(output.contains("├── Cargo.toml"));
        assert!(output.contains("# Package manifest"));
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
