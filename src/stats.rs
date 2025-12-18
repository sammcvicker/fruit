//! Codebase statistics collection and display
//!
//! This module collects and formats aggregate statistics about a codebase:
//! file counts by type, line counts, and language breakdown.

use serde::Serialize;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Maximum file size for line counting (5MB).
const MAX_FILE_SIZE_FOR_LINES: u64 = 5_000_000;

/// Collected statistics about a codebase.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CodebaseStats {
    /// Total number of files
    pub files: usize,
    /// Total number of directories
    pub directories: usize,
    /// Total lines of code (if counted)
    pub total_lines: Option<usize>,
    /// Statistics by language
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_language: Vec<LanguageStats>,
}

/// Statistics for a single language.
#[derive(Debug, Clone, Serialize)]
pub struct LanguageStats {
    /// Language name
    pub language: String,
    /// Number of files
    pub files: usize,
    /// Number of lines (if counted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<usize>,
    /// File extensions for this language
    pub extensions: Vec<String>,
}

/// Configuration for statistics collection.
#[derive(Debug, Clone, Default)]
pub struct StatsConfig {
    /// Whether to count lines of code
    pub count_lines: bool,
}

/// Statistics collector that accumulates data during tree traversal.
#[derive(Debug, Default)]
pub struct StatsCollector {
    config: StatsConfig,
    files: usize,
    directories: usize,
    /// Maps extension -> (file_count, line_count)
    by_extension: HashMap<String, (usize, usize)>,
}

impl StatsCollector {
    pub fn new(config: StatsConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Record a file in the statistics.
    pub fn record_file(&mut self, path: &Path) {
        self.files += 1;

        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let entry = self.by_extension.entry(ext.clone()).or_insert((0, 0));
        entry.0 += 1;

        if self.config.count_lines {
            if let Some(lines) = count_lines(path) {
                entry.1 += lines;
            }
        }
    }

    /// Record a directory in the statistics.
    pub fn record_directory(&mut self) {
        self.directories += 1;
    }

    /// Finalize and return the collected statistics.
    pub fn finalize(self) -> CodebaseStats {
        // Group extensions by language
        let mut by_language: HashMap<&str, (Vec<String>, usize, usize)> = HashMap::new();

        for (ext, (file_count, line_count)) in &self.by_extension {
            let lang = extension_to_language(ext);
            let entry = by_language.entry(lang).or_insert((Vec::new(), 0, 0));
            if !ext.is_empty() && !entry.0.contains(&format!(".{}", ext)) {
                entry.0.push(format!(".{}", ext));
            }
            entry.1 += file_count;
            entry.2 += line_count;
        }

        // Convert to sorted vector
        let mut languages: Vec<LanguageStats> = by_language
            .into_iter()
            .map(|(lang, (mut exts, files, lines))| {
                exts.sort();
                LanguageStats {
                    language: lang.to_string(),
                    files,
                    lines: if self.config.count_lines {
                        Some(lines)
                    } else {
                        None
                    },
                    extensions: exts,
                }
            })
            .collect();

        // Sort by file count descending
        languages.sort_by(|a, b| b.files.cmp(&a.files));

        let total_lines = if self.config.count_lines {
            Some(languages.iter().filter_map(|l| l.lines).sum())
        } else {
            None
        };

        CodebaseStats {
            files: self.files,
            directories: self.directories,
            total_lines,
            by_language: languages,
        }
    }
}

/// Count lines in a file using efficient byte scanning.
fn count_lines(path: &Path) -> Option<usize> {
    // Skip large files
    if let Ok(metadata) = path.metadata() {
        if metadata.len() > MAX_FILE_SIZE_FOR_LINES {
            return None;
        }
    }

    // Read file and count newlines
    let content = std::fs::read(path).ok()?;
    let newlines = content.iter().filter(|&&b| b == b'\n').count();

    // Add 1 if file doesn't end with newline and has content
    Some(if content.is_empty() || content.last() == Some(&b'\n') {
        newlines
    } else {
        newlines + 1
    })
}

/// Map file extension to language name.
fn extension_to_language(ext: &str) -> &'static str {
    match ext {
        // Rust
        "rs" => "Rust",
        // JavaScript/TypeScript
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" | "mts" | "cts" => "TypeScript",
        "jsx" => "JSX",
        "tsx" => "TSX",
        // Python
        "py" | "pyw" | "pyi" => "Python",
        // Go
        "go" => "Go",
        // Java/Kotlin
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        // C/C++
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => "C++",
        // C#
        "cs" => "C#",
        // Swift
        "swift" => "Swift",
        // Ruby
        "rb" | "erb" => "Ruby",
        // PHP
        "php" => "PHP",
        // Shell
        "sh" | "bash" | "zsh" | "fish" => "Shell",
        // Web
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "scss" | "sass" => "Sass",
        "less" => "Less",
        "vue" => "Vue",
        "svelte" => "Svelte",
        // Data/Config
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "xml" => "XML",
        "ini" | "cfg" => "Config",
        // Documentation
        "md" | "markdown" => "Markdown",
        "txt" => "Text",
        "rst" => "reStructuredText",
        // Other
        "sql" => "SQL",
        "graphql" | "gql" => "GraphQL",
        "proto" => "Protocol Buffers",
        "lua" => "Lua",
        "r" => "R",
        "scala" => "Scala",
        "clj" | "cljs" | "cljc" => "Clojure",
        "ex" | "exs" => "Elixir",
        "erl" | "hrl" => "Erlang",
        "hs" | "lhs" => "Haskell",
        "ml" | "mli" => "OCaml",
        "fs" | "fsx" | "fsi" => "F#",
        "pl" | "pm" => "Perl",
        "dart" => "Dart",
        "zig" => "Zig",
        "nim" => "Nim",
        "" => "No Extension",
        _ => "Other",
    }
}

/// Print statistics to stdout with optional color.
pub fn print_stats(stats: &CodebaseStats, use_color: bool) -> io::Result<()> {
    let color_choice = if use_color {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };
    let mut stdout = StandardStream::stdout(color_choice);

    // Header
    let mut bold = ColorSpec::new();
    bold.set_bold(true);
    stdout.set_color(&bold)?;
    writeln!(stdout, "Codebase Statistics")?;
    stdout.reset()?;
    writeln!(stdout, "───────────────────")?;

    // Summary
    writeln!(stdout, "Files:        {} total", stats.files)?;
    writeln!(stdout, "Directories:  {}", stats.directories)?;
    writeln!(stdout)?;

    // By language
    if !stats.by_language.is_empty() {
        stdout.set_color(&bold)?;
        writeln!(stdout, "By Language:")?;
        stdout.reset()?;

        let mut lang_color = ColorSpec::new();
        lang_color.set_fg(Some(Color::Cyan));

        for lang in &stats.by_language {
            write!(stdout, "  ")?;
            stdout.set_color(&lang_color)?;
            write!(stdout, "{:<14}", lang.language)?;
            stdout.reset()?;

            write!(stdout, "{:>4} files", lang.files)?;
            if let Some(lines) = lang.lines {
                write!(stdout, "  {:>8} lines", format_number(lines))?;
            }
            writeln!(stdout)?;
        }

        writeln!(stdout)?;
    }

    // Total lines
    if let Some(total) = stats.total_lines {
        stdout.set_color(&bold)?;
        write!(stdout, "Total:       ")?;
        stdout.reset()?;
        writeln!(stdout, "{} lines of code", format_number(total))?;
    }

    Ok(())
}

/// Format a number with thousand separators.
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();

    for (i, c) in chars.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, *c);
    }

    result
}

/// Print statistics as JSON.
pub fn print_stats_json(stats: &CodebaseStats) -> io::Result<()> {
    let json =
        serde_json::to_string_pretty(stats).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    println!("{}", json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_to_language() {
        assert_eq!(extension_to_language("rs"), "Rust");
        assert_eq!(extension_to_language("js"), "JavaScript");
        assert_eq!(extension_to_language("ts"), "TypeScript");
        assert_eq!(extension_to_language("py"), "Python");
        assert_eq!(extension_to_language("unknown"), "Other");
        assert_eq!(extension_to_language(""), "No Extension");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn test_stats_collector() {
        let mut collector = StatsCollector::new(StatsConfig { count_lines: false });
        collector.record_directory();
        collector.record_directory();

        let stats = collector.finalize();
        assert_eq!(stats.files, 0);
        assert_eq!(stats.directories, 2);
        assert!(stats.total_lines.is_none());
    }
}
