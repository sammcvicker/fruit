//! Streaming output formatter
//!
//! This module provides `StreamingFormatter` which outputs tree content
//! directly to stdout without buffering, for use with `StreamingWalker`.

use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::metadata::MetadataBlock;
use crate::tree::StreamingOutput;

use super::config::OutputConfig;
use super::utils::{
    calculate_wrap_width, continuation_prefix, render_metadata_block, write_metadata_line_with_symbol,
    MetadataRenderResult, RenderedLine,
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

    /// Write a rendered line with colors.
    fn write_rendered_line(
        &mut self,
        line: &RenderedLine,
        cont_prefix: &str,
        meta_prefix: &str,
    ) -> io::Result<()> {
        match line {
            RenderedLine::Separator => {
                self.stdout.reset()?;
                writeln!(self.stdout, "{}", cont_prefix)?;
            }
            RenderedLine::Content { text, symbol_name, style, indent } => {
                self.stdout.reset()?;
                write!(self.stdout, "{}{}", cont_prefix, meta_prefix)?;
                write_metadata_line_with_symbol(
                    &mut self.stdout,
                    text,
                    symbol_name.as_deref(),
                    style.color(),
                    style.is_intense(),
                    *indent,
                )?;
                writeln!(self.stdout)?;
            }
        }
        Ok(())
    }

    /// Write inline content (first line on same line as filename).
    fn write_inline_content(&mut self, line: &RenderedLine, meta_prefix: &str) -> io::Result<()> {
        if let RenderedLine::Content { text, symbol_name, style, indent } = line {
            write!(self.stdout, "  {}", meta_prefix)?;
            write_metadata_line_with_symbol(
                &mut self.stdout,
                text,
                symbol_name.as_deref(),
                style.color(),
                style.is_intense(),
                *indent,
            )?;
            writeln!(self.stdout)?;
            self.stdout.reset()?;
        }
        Ok(())
    }

    /// Print a metadata block with colors to stdout.
    fn print_metadata_block(
        &mut self,
        block: &MetadataBlock,
        prefix: &str,
        is_last: bool,
    ) -> io::Result<()> {
        let meta_prefix = self.config.metadata.prefix_str().to_string();
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
                writeln!(self.stdout)?;
            }
            MetadataRenderResult::Inline { first } => {
                self.write_inline_content(&first, &meta_prefix)?;
            }
            MetadataRenderResult::InlineWithBlock { first, block_lines } => {
                self.write_inline_content(&first, &meta_prefix)?;
                for line in &block_lines {
                    self.write_rendered_line(line, &cont_prefix, &meta_prefix)?;
                }
                self.stdout.reset()?;
            }
            MetadataRenderResult::Block { lines } => {
                writeln!(self.stdout)?; // End the filename line
                for line in &lines {
                    self.write_rendered_line(line, &cont_prefix, &meta_prefix)?;
                }
                self.stdout.reset()?;
            }
        }
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
