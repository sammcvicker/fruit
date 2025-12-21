//! Shared utility functions for output formatting

use std::io::{self, Write};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use crate::metadata::{LineStyle, MetadataBlock, MetadataLine, MetadataOrder};

/// Calculate the continuation prefix for lines below the filename.
///
/// Creates a prefix string for metadata lines displayed below a file/directory entry,
/// maintaining the visual tree structure. Used by both TreeFormatter and StreamingFormatter.
///
/// # Arguments
///
/// * `prefix` - The tree connector prefix from parent levels
/// * `is_last` - Whether this is the last item in its directory (affects connector style)
///
/// # Returns
///
/// A string with either `"│   "` (for non-last items) or `"    "` (for last items),
/// appended to the parent prefix to maintain visual tree alignment.
///
/// # Examples
///
/// ```
/// # use fruit::output::continuation_prefix;
/// // For a last item with no parent prefix
/// assert_eq!(continuation_prefix("", true), "    ");
///
/// // For a non-last item with a parent prefix
/// assert_eq!(continuation_prefix("│   ", false), "│   │   ");
/// ```
pub fn continuation_prefix(prefix: &str, is_last: bool) -> String {
    if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    }
}

/// Calculate the available width for text wrapping after accounting for prefixes.
///
/// Subtracts the lengths of tree structure and metadata prefixes from the configured
/// wrap width to determine how much space is available for actual content. Returns
/// None if wrapping is disabled or the available width is too small to be useful.
///
/// # Arguments
///
/// * `base_wrap_width` - The configured wrap width from user settings (None = no wrapping)
/// * `continuation_prefix_len` - Length of the tree continuation prefix (e.g., "│   ")
/// * `meta_prefix_len` - Length of the metadata prefix (e.g., "# ")
///
/// # Returns
///
/// * `Some(width)` - The calculated available width for text content (> 10 characters)
/// * `None` - If wrapping is disabled or resulting width would be <= 10 characters
///
/// # Examples
///
/// ```
/// # use fruit::output::calculate_wrap_width;
/// // With sufficient width
/// assert_eq!(calculate_wrap_width(Some(100), 4, 2), Some(94));
///
/// // Width too small after subtracting prefixes
/// assert_eq!(calculate_wrap_width(Some(15), 4, 2), None);
///
/// // Wrapping disabled
/// assert_eq!(calculate_wrap_width(None, 4, 2), None);
/// ```
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
///
/// Scans through a slice of metadata lines to determine if the next non-empty line
/// has greater indentation than the current level. This is used to detect if a baseline
/// item is a "group header" that should have a separator line before it.
///
/// # Arguments
///
/// * `lines` - Slice of metadata lines to scan (typically remaining lines after current)
/// * `current_indent` - The indentation level to compare against
///
/// # Returns
///
/// `true` if the next non-empty line has indentation > `current_indent`, `false` otherwise
/// (including when there are no more non-empty lines)
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
///
/// Controls visual grouping in metadata blocks by inserting blank separator lines
/// at strategic points. Separators appear when returning to baseline (indent 0) after
/// indented content, or before baseline items that have indented children (group headers).
///
/// # Arguments
///
/// * `current_indent` - The indentation level of the current line (0 = baseline)
/// * `prev_indent` - The indentation level of the previous line (None if first line)
/// * `has_children` - Whether the current line has indented children following it
///
/// # Returns
///
/// `true` if a blank separator line should be inserted before this line, `false` otherwise
///
/// # Behavior
///
/// Returns `true` only when ALL of the following are true:
/// - Current line is at baseline (indent = 0)
/// - There was a previous line
/// - Either:
///   - Previous line was indented (returning to baseline), OR
///   - Current line has indented children (is a group header)
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
///
/// Returns the portion of the string up to (but not including) the first newline character.
/// Used for displaying only the first line of multi-line metadata when not in full mode.
///
/// # Arguments
///
/// * `s` - The input string to extract from
///
/// # Returns
///
/// A string slice containing just the first line, or the entire string if it contains
/// no newline characters. Returns empty string if input is empty.
///
/// # Examples
///
/// ```
/// # use fruit::output::first_line;
/// assert_eq!(first_line("First\nSecond\nThird"), "First");
/// assert_eq!(first_line("Single line"), "Single line");
/// assert_eq!(first_line(""), "");
/// ```
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
///
/// Breaks long text into multiple lines that fit within the specified width, attempting
/// to break at word boundaries (whitespace) when possible. For words longer than max_width,
/// performs character-level wrapping. Uses character count (not byte count) to properly
/// handle UTF-8 multi-byte characters like emoji and CJK text.
///
/// # Arguments
///
/// * `text` - The text to wrap
/// * `max_width` - Maximum width in characters per line (0 = no wrapping)
///
/// # Returns
///
/// A vector of strings, each fitting within max_width characters. Returns at least
/// one string (which may be empty if input was empty).
///
/// # Behavior
///
/// - If `max_width` is 0, returns the original text unwrapped
/// - Splits on whitespace to preserve word boundaries
/// - Words longer than max_width are split at character boundaries
/// - UTF-8 aware: counts characters, not bytes (emoji count as 1 character)
///
/// # Examples
///
/// ```
/// # use fruit::output::wrap_text;
/// // Word boundary wrapping
/// let wrapped = wrap_text("hello world foo", 10);
/// assert_eq!(wrapped, vec!["hello", "world foo"]);
///
/// // Character-level wrapping for long words
/// let wrapped = wrap_text("verylongword", 5);
/// assert_eq!(wrapped, vec!["veryl", "ongwo", "rd"]);
///
/// // UTF-8 handling (emoji are 1 character each)
/// let wrapped = wrap_text("🎉🎊🎁🎂🎃", 3);
/// assert_eq!(wrapped, vec!["🎉🎊🎁", "🎂🎃"]);
/// ```
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
///
/// Outputs a single rendered metadata line (either a separator or content line) to
/// the terminal with appropriate colors and prefixes. Used by both StreamingFormatter
/// and TreeFormatter for consistent rendering.
///
/// # Arguments
///
/// * `stdout` - Mutable reference to the terminal output stream
/// * `line` - The rendered line to write (separator or content with styling)
/// * `cont_prefix` - The continuation prefix for tree structure (e.g., "│   ")
/// * `meta_prefix` - The metadata type prefix (e.g., "# " for comments)
///
/// # Returns
///
/// `Ok(())` on success, or an IO error if writing to stdout fails
///
/// # Behavior
///
/// - For `RenderedLine::Separator`: writes just the continuation prefix and newline
/// - For `RenderedLine::Content`: writes continuation prefix, metadata prefix, colored
///   content (with symbol highlighting if present), and newline
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
///
/// Outputs the first line of metadata on the same line as the filename, typically used
/// when displaying a single-line comment or when not in full metadata mode. Used by both
/// StreamingFormatter and TreeFormatter for consistent inline rendering.
///
/// # Arguments
///
/// * `stdout` - Mutable reference to the terminal output stream
/// * `line` - The rendered line to write inline (typically first line of metadata)
/// * `meta_prefix` - The metadata type prefix (e.g., "# " for comments)
///
/// # Returns
///
/// `Ok(())` on success, or an IO error if writing to stdout fails
///
/// # Behavior
///
/// - Only writes content if `line` is `RenderedLine::Content` (ignores separators)
/// - Writes two spaces, metadata prefix, then colored content with symbol highlighting
/// - Ends the filename line with a newline and resets terminal colors
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
///
/// High-level function that renders and outputs a complete metadata block (comments, types,
/// TODOs, imports) for a file entry. Handles both inline and block display modes, text
/// wrapping, and visual grouping. Used by both StreamingFormatter and TreeFormatter.
///
/// # Arguments
///
/// * `stdout` - Mutable reference to the terminal output stream
/// * `block` - The metadata block to render (contains comments, types, etc.)
/// * `prefix` - Tree connector prefix from parent levels
/// * `is_last` - Whether this is the last item in its directory
/// * `meta_prefix` - Metadata type prefix (e.g., "# " for comments)
/// * `order` - Display order for metadata sections (comments first vs types first)
/// * `show_full` - Whether to show full metadata (true) or just first line (false)
/// * `wrap_width` - Optional text wrapping width in characters
///
/// # Returns
///
/// `Ok(())` on success, or an IO error if writing to stdout fails
///
/// # Behavior
///
/// Delegates to `render_metadata_block()` to determine display strategy, then outputs:
/// - `Empty`: Just ends the filename line with newline
/// - `Inline`: Shows first line inline on the same line as filename
/// - `InlineWithBlock`: Shows first line inline, remaining lines in block below
/// - `Block`: Shows all lines in a block below the filename
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
///
/// Core rendering logic that determines how to display a metadata block (inline vs block,
/// with or without separators) and produces a structured result with pre-styled lines.
/// This centralizes the logic for display strategy and group separators, allowing both
/// StreamingFormatter and TreeFormatter to use the same rendering rules.
///
/// # Arguments
///
/// * `block` - The metadata block to render (contains comments, types, TODOs, imports)
/// * `order` - Display order for metadata sections (comments first vs types first)
/// * `show_full` - Whether to show all metadata (true) or just first line (false)
/// * `wrap_width` - Optional text wrapping width in characters (after prefix adjustment)
///
/// # Returns
///
/// A `MetadataRenderResult` enum indicating the display strategy:
/// - `Empty`: No metadata to display
/// - `Inline { first }`: Single line to show inline with filename
/// - `InlineWithBlock { first, block_lines }`: First line inline, rest below
/// - `Block { lines }`: All lines displayed in a block below filename
///
/// # Behavior
///
/// Display strategy is determined by:
/// - If block is empty → `Empty`
/// - If not in full mode → `Inline` with first line only
/// - If total is 1 line → `Inline`
/// - If first section is single line and more content follows → `InlineWithBlock`
/// - Otherwise → `Block` with all lines below
///
/// Group separators are inserted automatically to visually organize metadata sections
/// and hierarchical structures.
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
