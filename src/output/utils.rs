//! Shared utility functions for output formatting

use std::io::{self, Write};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use crate::metadata::{LineStyle, MetadataBlock, MetadataLine, MetadataOrder};

/// Calculate the continuation prefix for lines below the filename.
/// Used by both TreeFormatter and StreamingFormatter.
pub fn continuation_prefix(prefix: &str, is_last: bool) -> String {
    if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    }
}

/// Calculate the available width for text wrapping after accounting for prefixes.
/// Returns None if wrapping is disabled or the available width is too small.
pub fn calculate_wrap_width(
    base_wrap_width: Option<usize>,
    continuation_prefix_len: usize,
    meta_prefix_len: usize,
) -> Option<usize> {
    base_wrap_width
        .map(|w| w.saturating_sub(continuation_prefix_len + meta_prefix_len))
        .filter(|&w| w > 10)
}

/// Check if the next non-empty line in a slice is indented relative to current indent.
pub fn has_indented_children(
    lines: &[&crate::metadata::MetadataLine],
    current_indent: usize,
) -> bool {
    lines
        .iter()
        .find(|l| !l.content.trim().is_empty())
        .is_some_and(|next| next.indent > current_indent)
}

/// Determine if a blank line should be inserted before a baseline item.
/// Returns true when returning to baseline after indented content or when
/// this baseline item has indented children (it's a group header).
pub fn should_insert_group_separator(
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

/// Extract the first line from a string.
pub fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

/// Write a metadata line, highlighting the symbol name in bold red if present.
/// The `indent` parameter specifies the number of spaces to prepend for hierarchy display.
pub fn write_metadata_line_with_symbol(
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

/// Wrap text to fit within max_width, preferring word boundaries.
/// Uses character count (not byte count) to properly handle UTF-8.
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
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

/// Write a rendered line with colors to stdout.
/// Used by both StreamingFormatter and TreeFormatter.
pub fn write_rendered_line(
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
/// Used by both StreamingFormatter and TreeFormatter.
pub fn write_inline_content(
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
/// Used by both StreamingFormatter and TreeFormatter.
pub fn print_metadata_block(
    stdout: &mut StandardStream,
    block: &MetadataBlock,
    prefix: &str,
    is_last: bool,
    meta_prefix: &str,
    order: MetadataOrder,
    show_full: bool,
    wrap_width: Option<usize>,
) -> io::Result<()> {
    let cont_prefix = continuation_prefix(prefix, is_last);
    let wrap_calc_width = calculate_wrap_width(
        wrap_width,
        cont_prefix.chars().count(),
        meta_prefix.chars().count(),
    );

    let result = render_metadata_block(block, order, show_full, wrap_calc_width);

    match result {
        MetadataRenderResult::Empty => {
            writeln!(stdout)?;
        }
        MetadataRenderResult::Inline { first } => {
            write_inline_content(stdout, &first, meta_prefix)?;
        }
        MetadataRenderResult::InlineWithBlock { first, block_lines } => {
            write_inline_content(stdout, &first, meta_prefix)?;
            for line in &block_lines {
                write_rendered_line(stdout, line, &cont_prefix, meta_prefix)?;
            }
            stdout.reset()?;
        }
        MetadataRenderResult::Block { lines } => {
            writeln!(stdout)?; // End the filename line
            for line in &lines {
                write_rendered_line(stdout, line, &cont_prefix, meta_prefix)?;
            }
            stdout.reset()?;
        }
    }
    Ok(())
}

/// A rendered line from a metadata block, ready for output.
/// This abstraction allows the same rendering logic to be used
/// for both colored (stdout) and plain (String) output.
#[derive(Debug, Clone)]
pub enum RenderedLine {
    /// A blank separator line (just continuation prefix)
    Separator,
    /// A content line with styling information
    Content {
        text: String,
        symbol_name: Option<String>,
        style: LineStyle,
        indent: usize,
    },
}

/// Result of rendering a metadata block.
#[derive(Debug)]
pub enum MetadataRenderResult {
    /// Block is empty, just end the filename line
    Empty,
    /// Show first line inline on the same line as filename
    Inline { first: RenderedLine },
    /// Show first line inline, then remaining lines in a block below
    InlineWithBlock {
        first: RenderedLine,
        block_lines: Vec<RenderedLine>,
    },
    /// Show all lines in a block below the filename
    Block { lines: Vec<RenderedLine> },
}

/// Render a metadata block into a structured result that formatters can write.
/// This centralizes the logic for determining inline vs block display and group separators.
pub fn render_metadata_block(
    block: &MetadataBlock,
    order: MetadataOrder,
    show_full: bool,
    wrap_width: Option<usize>,
) -> MetadataRenderResult {
    if block.is_empty() {
        return MetadataRenderResult::Empty;
    }

    let lines = block.lines_in_order(order);

    // Not in full mode: show first line inline only
    if !show_full {
        if let Some(first) = block.first_line(order) {
            return MetadataRenderResult::Inline {
                first: RenderedLine::Content {
                    text: first_line(&first.content).to_string(),
                    symbol_name: first.symbol_name.clone(),
                    style: first.style,
                    indent: first.indent,
                },
            };
        }
        return MetadataRenderResult::Empty;
    }

    // Full mode: check if we should show inline or as block
    let total_lines = block.total_lines();
    let first_is_single = block.first_section_is_single_line(order);

    // If total is just 1 line, show inline
    if total_lines == 1 {
        if let Some(first) = block.first_line(order) {
            return MetadataRenderResult::Inline {
                first: RenderedLine::Content {
                    text: first_line(&first.content).to_string(),
                    symbol_name: first.symbol_name.clone(),
                    style: first.style,
                    indent: first.indent,
                },
            };
        }
        return MetadataRenderResult::Empty;
    }

    // Helper to render a slice of MetadataLines into RenderedLines
    let render_lines = |meta_lines: &[&MetadataLine]| -> Vec<RenderedLine> {
        let mut result = Vec::new();
        let mut prev_indent: Option<usize> = None;

        // Add blank line before block
        result.push(RenderedLine::Separator);

        for (i, meta_line) in meta_lines.iter().enumerate() {
            let content = meta_line.content.trim();

            // Empty line is a separator
            if content.is_empty() {
                result.push(RenderedLine::Separator);
                prev_indent = None;
                continue;
            }

            // Check if we should insert a group separator
            let has_children = has_indented_children(&meta_lines[i + 1..], meta_line.indent);
            if should_insert_group_separator(meta_line.indent, prev_indent, has_children) {
                result.push(RenderedLine::Separator);
            }
            prev_indent = Some(meta_line.indent);

            // Wrap text if needed
            let wrapped = if let Some(width) = wrap_width {
                wrap_text(content, width)
            } else {
                vec![content.to_string()]
            };

            for wrapped_line in wrapped {
                result.push(RenderedLine::Content {
                    text: wrapped_line,
                    symbol_name: meta_line.symbol_name.clone(),
                    style: meta_line.style,
                    indent: meta_line.indent,
                });
            }
        }

        // Add blank line after block
        result.push(RenderedLine::Separator);

        result
    };

    // If first section is single line and there's more content, show first inline then rest below
    if first_is_single {
        if let Some(first) = block.first_line(order) {
            // Skip the first line (already shown inline) and the separator after it
            let skip_count = if block.has_both() { 2 } else { 1 };
            let remaining: Vec<_> = lines.iter().skip(skip_count).collect();
            let block_lines = render_lines(&remaining);

            return MetadataRenderResult::InlineWithBlock {
                first: RenderedLine::Content {
                    text: first_line(&first.content).to_string(),
                    symbol_name: first.symbol_name.clone(),
                    style: first.style,
                    indent: first.indent,
                },
                block_lines,
            };
        }
    }

    // First section has multiple lines, show everything below
    let line_refs: Vec<_> = lines.iter().collect();
    MetadataRenderResult::Block {
        lines: render_lines(&line_refs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // None in, None out
        let width = calculate_wrap_width(None, 4, 2);
        assert_eq!(width, None);
    }

    #[test]
    fn test_calculate_wrap_width_too_small() {
        // If result is <= 10, return None
        let width = calculate_wrap_width(Some(15), 4, 2);
        assert_eq!(width, None); // 15 - 4 - 2 = 9, which is <= 10
    }

    #[test]
    fn test_first_line_extracts_first() {
        assert_eq!(first_line("line1\nline2"), "line1");
        assert_eq!(first_line("single line"), "single line");
        assert_eq!(first_line(""), "");
    }

    #[test]
    fn test_wrap_text_preserves_word_boundaries() {
        let text = "hello world foo bar";
        let wrapped = wrap_text(text, 10);
        assert_eq!(wrapped, vec!["hello", "world foo", "bar"]);
    }

    #[test]
    fn test_wrap_text_long_word() {
        let text = "verylongword";
        let wrapped = wrap_text(text, 5);
        assert_eq!(wrapped, vec!["veryl", "ongwo", "rd"]);
    }

    #[test]
    fn test_wrap_text_empty() {
        let wrapped = wrap_text("", 10);
        assert_eq!(wrapped, vec![""]);
    }

    #[test]
    fn test_wrap_text_zero_width() {
        let wrapped = wrap_text("hello world", 0);
        assert_eq!(wrapped, vec!["hello world"]);
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

    #[test]
    fn test_has_indented_children_with_children() {
        use crate::metadata::{LineStyle, MetadataLine};

        let lines = vec![MetadataLine {
            content: "child".to_string(),
            style: LineStyle::TypeSignature,
            symbol_name: None,
            indent: 4,
        }];
        let line_refs: Vec<&MetadataLine> = lines.iter().collect();
        assert!(has_indented_children(&line_refs, 0));
    }

    #[test]
    fn test_has_indented_children_no_children() {
        use crate::metadata::{LineStyle, MetadataLine};

        let lines = vec![MetadataLine {
            content: "sibling".to_string(),
            style: LineStyle::TypeSignature,
            symbol_name: None,
            indent: 0,
        }];
        let line_refs: Vec<&MetadataLine> = lines.iter().collect();
        assert!(!has_indented_children(&line_refs, 0));
    }

    #[test]
    fn test_has_indented_children_empty() {
        let lines: Vec<&crate::metadata::MetadataLine> = vec![];
        assert!(!has_indented_children(&lines, 0));
    }

    #[test]
    fn test_has_indented_children_skips_empty_lines() {
        use crate::metadata::{LineStyle, MetadataLine};

        let lines = vec![
            MetadataLine {
                content: "   ".to_string(), // Empty/whitespace only
                style: LineStyle::TypeSignature,
                symbol_name: None,
                indent: 0,
            },
            MetadataLine {
                content: "child".to_string(),
                style: LineStyle::TypeSignature,
                symbol_name: None,
                indent: 4,
            },
        ];
        let line_refs: Vec<&MetadataLine> = lines.iter().collect();
        // Should skip the empty line and find the indented child
        assert!(has_indented_children(&line_refs, 0));
    }

    #[test]
    fn test_should_insert_group_separator_basic() {
        // At baseline (indent=0), no previous line - no separator
        assert!(!should_insert_group_separator(0, None, false));

        // At baseline, previous was at baseline, no children - no separator
        assert!(!should_insert_group_separator(0, Some(0), false));

        // At baseline, previous was indented - should insert separator
        assert!(should_insert_group_separator(0, Some(4), false));

        // At baseline, has children (is a group header) - should insert separator
        assert!(should_insert_group_separator(0, Some(0), true));
    }

    #[test]
    fn test_should_insert_group_separator_not_baseline() {
        // Not at baseline (indent > 0) - never insert separator
        assert!(!should_insert_group_separator(4, None, false));
        assert!(!should_insert_group_separator(4, Some(0), false));
        assert!(!should_insert_group_separator(4, Some(4), true));
    }
}
