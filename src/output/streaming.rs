//! Streaming output formatter
//!
//! This module provides `StreamingFormatter` which outputs tree content
//! directly to stdout without buffering, for use with `StreamingWalker`.

use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::metadata::{MetadataBlock, MetadataLine};
use crate::tree::StreamingOutput;

use super::config::OutputConfig;
use super::utils::{
    calculate_wrap_width, continuation_prefix, first_line, has_indented_children,
    should_insert_group_separator, wrap_text, write_metadata_line_with_symbol,
};

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
        first: &MetadataLine,
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
        lines: &[&MetadataLine],
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
        size: Option<u64>,
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

            // Show file size if provided
            if let Some(bytes) = size {
                write!(self.stdout, "  ")?;
                self.stdout
                    .set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                write!(self.stdout, "[{}]", crate::tree::format_size(bytes))?;
                self.stdout.reset()?;
            }

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
