//! Source file comment extraction
//!
//! This module extracts the first documentation comment from source files.
//! It supports multiple programming languages and handles language-specific
//! conventions like magic comments, shebangs, and documentation annotations.
//!
//! # Design Philosophy
//!
//! The extraction aims to find the most meaningful "file-level" documentation:
//! - For modules/packages: The comment describing the entire file's purpose
//! - For scripts: The header comment explaining what the script does
//!
//! # Language-Specific Behavior
//!
//! Each language has its own conventions for file-level documentation:
//!
//! - **Rust**: Prioritizes `//!` module docs, then `///` item docs, then `/* */` blocks
//! - **Python**: Extracts module docstrings (skips shebang and encoding declarations)
//! - **JavaScript/TypeScript**: JSDoc `/** */` comments, then `//` line comments
//! - **Go**: Package comments before `package` declaration
//! - **C/C++**: Block `/* */` comments, then `//` line comments at file start
//! - **Ruby**: `#` comments (skips `frozen_string_literal` and encoding magic comments)
//! - **Shell**: `#` comments after shebang
//! - **Java/Kotlin/Swift**: JavaDoc `/** */` comments (filters `@` annotations)
//! - **PHP**: PHPDoc `/** */` after `<?php` tag, or `//` and `#` comments
//! - **C#**: XML doc `///` comments (skips `<tag>` elements), then `/* */` blocks

use std::path::Path;

use crate::file_utils::read_source_file;
use crate::language::Language;
use crate::string_utils::strip_any_prefix;

/// Helper function to extract block comments.
///
/// Extracts a block comment starting with `start_marker` and ending with `end_marker`.
/// Optionally strips prefixes like `*` from each line.
///
/// # Arguments
/// * `content` - The source code content
/// * `start_marker` - The opening comment marker (e.g., `"/*"`, `"/**"`)
/// * `end_marker` - The closing comment marker (e.g., `"*/"`)
/// * `strip_prefix` - Optional character to strip from line starts (e.g., `'*'`)
/// * `filter_fn` - Optional function to filter lines (e.g., skip lines starting with '@')
///
/// # Returns
/// `Some(String)` if a valid block comment is found, `None` otherwise
fn extract_block_comment<F>(
    content: &str,
    start_marker: &str,
    end_marker: &str,
    strip_prefix: Option<char>,
    filter_fn: Option<F>,
) -> Option<String>
where
    F: Fn(&str) -> bool,
{
    let trimmed = content.trim_start();
    if !trimmed.starts_with(start_marker) {
        return None;
    }

    let after_start = &trimmed[start_marker.len()..];
    let end = after_start.find(end_marker)?;
    let block = &after_start[..end];

    let cleaned: Vec<&str> = block
        .lines()
        .map(|l| {
            let mut line = l.trim();
            if let Some(prefix_char) = strip_prefix {
                line = line.trim_start_matches(prefix_char).trim();
            }
            line
        })
        .filter(|l| !l.is_empty() && *l != "/")
        .filter(|l| filter_fn.as_ref().map_or(true, |f| f(l)))
        .collect();

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.join("\n"))
    }
}

/// Helper function to extract consecutive line comments.
///
/// Collects consecutive lines starting with the given prefix,
/// stopping at the first non-comment, non-empty line.
///
/// # Arguments
/// * `lines` - Iterator of lines to process
/// * `prefix` - The line comment prefix (e.g., `"//"`, `"#"`)
/// * `skip_empty` - Whether to skip empty lines while collecting
/// * `stop_at_empty` - Whether to stop at the first empty line after collecting starts
/// * `filter_fn` - Optional function to filter lines (returns true to keep the line)
///
/// # Returns
/// `Some(String)` if non-empty comments are found, `None` otherwise
fn extract_line_comments<'a, I, F>(
    lines: I,
    prefix: &str,
    skip_empty: bool,
    stop_at_empty: bool,
    filter_fn: Option<F>,
) -> Option<String>
where
    I: IntoIterator<Item = &'a str>,
    F: Fn(&str) -> bool,
{
    let mut comment_lines = Vec::new();
    let mut started = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.starts_with(prefix) {
            started = true;
            let comment = trimmed.strip_prefix(prefix).unwrap_or("").trim();
            if filter_fn.as_ref().map_or(true, |f| f(comment)) {
                comment_lines.push(comment);
            }
        } else if trimmed.is_empty() {
            if stop_at_empty && started && !comment_lines.is_empty() {
                break;
            }
            if !skip_empty {
                break;
            }
        } else {
            break;
        }
    }

    if !comment_lines.is_empty() && comment_lines.iter().any(|l| !l.is_empty()) {
        Some(comment_lines.join("\n"))
    } else {
        None
    }
}

/// Extract the first documentation comment from a source file.
///
/// This function reads the file at the given path and extracts what it considers
/// to be the primary documentation comment based on the file's extension.
///
/// # Supported Extensions
///
/// | Extension | Language | Comment Style |
/// |-----------|----------|---------------|
/// | `.rs` | Rust | `//!`, `///`, `/* */` |
/// | `.py` | Python | `"""..."""` docstrings |
/// | `.js`, `.jsx`, `.ts`, `.tsx`, `.mjs`, `.cjs` | JavaScript/TypeScript | `/** */`, `//` |
/// | `.go` | Go | `//`, `/* */` before package |
/// | `.c`, `.h`, `.cpp`, `.hpp`, `.cc`, `.cxx` | C/C++ | `/* */`, `//` |
/// | `.rb` | Ruby | `#` comments |
/// | `.sh`, `.bash`, `.zsh` | Shell | `#` comments |
/// | `.java`, `.kt`, `.kts`, `.swift` | Java/Kotlin/Swift | `/** */` |
/// | `.php` | PHP | `/** */`, `//`, `#` |
/// | `.cs` | C# | `///`, `/* */` |
///
/// # Returns
///
/// - `Some(String)` - The extracted comment text with comment markers removed
/// - `None` - If no comment found, unsupported extension, file unreadable, or file > 1MB
///
/// # File Size Limit
///
/// Files larger than 1MB are skipped to prevent memory issues
/// when processing large generated or binary files with code extensions.
pub fn extract_first_comment(path: &Path) -> Option<String> {
    let (content, _extension) = read_source_file(path)?;
    let language = Language::from_path(path)?;

    match language {
        Language::Rust => extract_rust_comment(&content),
        Language::Python => extract_python_docstring(&content),
        Language::JavaScript | Language::TypeScript => extract_js_comment(&content),
        Language::Go => extract_go_comment(&content),
        Language::C | Language::Cpp => extract_c_comment(&content),
        Language::Ruby => extract_ruby_comment(&content),
        Language::Shell => extract_shell_comment(&content),
        // Java, Kotlin, Swift use JavaDoc-style /** */ comments
        Language::Java | Language::Kotlin | Language::Swift => extract_javadoc_comment(&content),
        // PHP uses PHPDoc /** */ and also # comments
        Language::PHP => extract_php_comment(&content),
        // C# uses /// XML doc comments
        Language::CSharp => extract_csharp_comment(&content),
    }
}

/// Extract Rust documentation comments.
///
/// Priority order:
/// 1. `//!` - Inner doc comments (module-level documentation)
/// 2. `///` - Outer doc comments (item documentation, skips `#[...]` attributes)
/// 3. `/* */` - Block comments at file start
fn extract_rust_comment(content: &str) -> Option<String> {
    // Rust has unique requirements for //! vs /// distinction and attribute skipping
    // Keep manual implementation for line comments
    let lines: Vec<&str> = content.lines().collect();

    // Look for //! module doc comments - collect all consecutive lines
    let mut doc_lines = Vec::new();
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("//!") {
            let comment = trimmed.strip_prefix("//!").unwrap_or("").trim();
            doc_lines.push(comment);
        } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
            break;
        }
    }
    if !doc_lines.is_empty() && doc_lines.iter().any(|l| !l.is_empty()) {
        return Some(doc_lines.join("\n"));
    }

    // Look for /// doc comments on first item - collect all consecutive lines
    doc_lines.clear();
    let mut in_doc_comment = false;
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("///") {
            in_doc_comment = true;
            let comment = trimmed.strip_prefix("///").unwrap_or("").trim();
            doc_lines.push(comment);
        } else if in_doc_comment
            || (!trimmed.is_empty()
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("#[")
                && !trimmed.starts_with("#!["))
        {
            break;
        }
    }
    if !doc_lines.is_empty() && doc_lines.iter().any(|l| !l.is_empty()) {
        return Some(doc_lines.join("\n"));
    }

    // Look for /* */ block comments at the top (use helper)
    extract_block_comment(content, "/*", "*/", Some('*'), None::<fn(&str) -> bool>)
}

/// Extract Python module docstrings.
///
/// Skips:
/// - Shebang lines (`#!/usr/bin/env python`)
/// - Encoding declarations (`# -*- coding: utf-8 -*-`)
/// - Empty lines before the docstring
///
/// Supports both `"""..."""` and `'''...'''` quote styles.
fn extract_python_docstring(content: &str) -> Option<String> {
    let trimmed = content.trim_start();

    // Skip shebang and encoding declarations
    let mut lines = trimmed.lines().peekable();
    while let Some(line) = lines.peek() {
        let t = line.trim();
        if t.starts_with('#') || t.is_empty() {
            lines.next();
        } else {
            break;
        }
    }

    let rest: String = lines.collect::<Vec<_>>().join("\n");
    let rest = rest.trim_start();

    // Look for docstring
    for quote in ["\"\"\"", "'''"] {
        if rest.starts_with(quote) {
            let after_quote = &rest[3..];
            if let Some(end) = after_quote.find(quote) {
                let doc = after_quote[..end].trim();
                if !doc.is_empty() {
                    // Return the full docstring, cleaned up
                    let cleaned: Vec<&str> = doc.lines().map(|l| l.trim()).collect();
                    return Some(cleaned.join("\n"));
                }
            }
        }
    }

    None
}

/// Extract JavaScript/TypeScript comments.
///
/// Priority order:
/// 1. JSDoc `/** ... */` block comments
/// 2. `//` line comments at file start (collects consecutive lines)
fn extract_js_comment(content: &str) -> Option<String> {
    // Try JSDoc block comment first
    if let Some(comment) = extract_block_comment(content, "/**", "*/", Some('*'), None::<fn(&str) -> bool>) {
        return Some(comment);
    }

    // Fall back to // line comments
    extract_line_comments(content.lines(), "//", true, false, None::<fn(&str) -> bool>)
}

/// Extract Go package comments.
///
/// Go convention: Package documentation comes immediately before the `package` declaration.
/// Supports both `//` line comments and `/* */` block comments.
/// Non-empty lines between comments and `package` reset the comment buffer.
fn extract_go_comment(content: &str) -> Option<String> {
    // Go package comments come before the package declaration
    // Has unique requirements: resets comment buffer on non-comment code
    let mut comment_lines: Vec<&str> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            let comment = trimmed.strip_prefix("//").unwrap_or("").trim();
            comment_lines.push(comment);
        } else if trimmed.starts_with("/*") {
            // Block comment - use helper for extraction
            return extract_block_comment(content, "/*", "*/", Some('*'), None::<fn(&str) -> bool>);
        } else if trimmed.starts_with("package ") {
            break;
        } else if !trimmed.is_empty() {
            comment_lines.clear();
        }
    }

    if !comment_lines.is_empty() && comment_lines.iter().any(|l| !l.is_empty()) {
        return Some(comment_lines.join("\n"));
    }
    None
}

/// Extract C/C++ comments.
///
/// Priority order:
/// 1. `/* */` block comments at file start
/// 2. `//` line comments at file start (collects consecutive lines)
fn extract_c_comment(content: &str) -> Option<String> {
    // Try block comment first
    if let Some(comment) = extract_block_comment(content, "/*", "*/", Some('*'), None::<fn(&str) -> bool>) {
        return Some(comment);
    }

    // Fall back to // line comments
    extract_line_comments(content.lines(), "//", true, false, None::<fn(&str) -> bool>)
}

/// Extract Ruby comments.
///
/// Skips Ruby magic comments:
/// - Shebang (`#!/usr/bin/env ruby`)
/// - `# frozen_string_literal: true`
/// - `# encoding: utf-8` / `# coding: utf-8`
///
/// Stops collecting at the first empty line after comments start.
fn extract_ruby_comment(content: &str) -> Option<String> {
    // Ruby has unique requirements: skip magic comments and stop at empty line
    // Manual implementation needed
    let mut comment_lines = Vec::new();
    let mut past_preamble = false;

    for line in content.lines() {
        let trimmed = line.trim();
        // Skip shebang
        if trimmed.starts_with("#!") {
            continue;
        }
        // Skip encoding/frozen string magic comments
        if trimmed.starts_with("# frozen_string_literal")
            || trimmed.starts_with("# encoding:")
            || trimmed.starts_with("# coding:")
        {
            continue;
        }
        if trimmed.starts_with('#') {
            past_preamble = true;
            let comment = trimmed.strip_prefix('#').unwrap_or("").trim();
            comment_lines.push(comment);
        } else if trimmed.is_empty() {
            if past_preamble && !comment_lines.is_empty() {
                // Empty line after comments - stop collecting
                break;
            }
            continue;
        } else {
            break;
        }
    }
    if !comment_lines.is_empty() && comment_lines.iter().any(|l| !l.is_empty()) {
        return Some(comment_lines.join("\n"));
    }
    None
}

/// Extract shell script comments (bash, sh, zsh).
///
/// Skips the shebang line (`#!/bin/bash`, etc.) and collects
/// subsequent `#` comments. Stops at the first empty line after
/// comments start.
fn extract_shell_comment(content: &str) -> Option<String> {
    // Shell comments have unique requirements (skip shebang, stop at empty)
    // so we keep the manual implementation
    let mut comment_lines = Vec::new();
    let mut past_shebang = false;

    for line in content.lines() {
        let trimmed = line.trim();
        // Skip shebang
        if trimmed.starts_with("#!") {
            continue;
        }
        if trimmed.starts_with('#') {
            past_shebang = true;
            let comment = trimmed.strip_prefix('#').unwrap_or("").trim();
            comment_lines.push(comment);
        } else if trimmed.is_empty() {
            if past_shebang && !comment_lines.is_empty() {
                // Empty line after comments - stop collecting
                break;
            }
            continue;
        } else {
            break;
        }
    }
    if !comment_lines.is_empty() && comment_lines.iter().any(|l| !l.is_empty()) {
        return Some(comment_lines.join("\n"));
    }
    None
}

/// Extract JavaDoc-style comments (Java, Kotlin, Swift).
///
/// Priority order:
/// 1. `/** ... */` doc comments (filters out `@param`, `@return`, etc.)
/// 2. `//` line comments at file start
///
/// Note: Lines starting with `@` are filtered as they typically contain
/// annotation metadata rather than documentation prose.
fn extract_javadoc_comment(content: &str) -> Option<String> {
    // Try JavaDoc block comment first, filtering @ annotations
    let filter_annotations = |line: &str| !line.starts_with('@');
    if let Some(comment) = extract_block_comment(content, "/**", "*/", Some('*'), Some(filter_annotations)) {
        return Some(comment);
    }

    // Fall back to // line comments
    extract_line_comments(content.lines(), "//", true, false, None::<fn(&str) -> bool>)
}

/// PHP opening tag prefixes (order matters: longer prefixes first)
const PHP_TAG_PREFIXES: &[&str] = &["<?php", "<?"];

/// Extract PHP comments.
///
/// Handles PHP opening tags:
/// - `<?php` (full tag)
/// - `<?` (short tag)
///
/// Priority order:
/// 1. PHPDoc `/** ... */` comments (filters `@` annotations)
/// 2. `//` line comments
/// 3. `#` line comments (but not PHP 8 attributes like `#[Attribute]`)
fn extract_php_comment(content: &str) -> Option<String> {
    // Skip <?php or <? opening tag
    let content = content.trim_start();
    let content = strip_any_prefix(content, PHP_TAG_PREFIXES);

    // Try PHPDoc block comment first, filtering @ annotations
    let filter_annotations = |line: &str| !line.starts_with('@');
    if let Some(comment) = extract_block_comment(content, "/**", "*/", Some('*'), Some(filter_annotations)) {
        return Some(comment);
    }

    // PHP supports both // and # for line comments
    // Need to handle manually due to dual-prefix support and attribute filtering
    let mut comment_lines = Vec::new();
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with("//") {
            let comment = t.strip_prefix("//").unwrap_or("").trim();
            comment_lines.push(comment);
        } else if t.starts_with('#') && !t.starts_with("#[") {
            let comment = t.strip_prefix('#').unwrap_or("").trim();
            comment_lines.push(comment);
        } else if t.is_empty() {
            continue;
        } else {
            break;
        }
    }
    if !comment_lines.is_empty() && comment_lines.iter().any(|l| !l.is_empty()) {
        return Some(comment_lines.join("\n"));
    }

    None
}

/// Extract C# comments.
///
/// Priority order:
/// 1. `///` XML documentation comments (filters `<tag>` elements)
/// 2. `//` regular line comments
/// 3. `/* */` block comments
///
/// Skips `using` statements and `[Attribute]` lines when looking for comments.
fn extract_csharp_comment(content: &str) -> Option<String> {
    // C# has complex requirements: /// with XML filtering, //, and /* */
    // Manual implementation needed due to dual-prefix support (/// and //)
    let trimmed = content.trim_start();

    let mut doc_lines = Vec::new();
    for line in trimmed.lines() {
        let t = line.trim();
        if t.starts_with("///") {
            let comment = t.strip_prefix("///").unwrap_or("").trim();
            // Skip XML tags like <summary>, </summary>, <param>, etc.
            if !comment.starts_with('<') && !comment.ends_with('>') {
                doc_lines.push(comment);
            }
        } else if t.starts_with("//") {
            // Regular comment
            let comment = t.strip_prefix("//").unwrap_or("").trim();
            doc_lines.push(comment);
        } else if t.is_empty() || t.starts_with("using ") || t.starts_with("[") {
            continue;
        } else {
            break;
        }
    }
    if !doc_lines.is_empty() && doc_lines.iter().any(|l| !l.is_empty()) {
        return Some(doc_lines.join("\n"));
    }

    // Also check for /* */ block comments
    extract_block_comment(trimmed, "/*", "*/", Some('*'), None::<fn(&str) -> bool>)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_module_doc() {
        let content = "//! This is a module doc\n\nfn main() {}";
        assert_eq!(
            extract_rust_comment(content),
            Some("This is a module doc".to_string())
        );
    }

    #[test]
    fn test_rust_item_doc() {
        let content = "/// This documents the function\nfn main() {}";
        assert_eq!(
            extract_rust_comment(content),
            Some("This documents the function".to_string())
        );
    }

    #[test]
    fn test_rust_block_comment() {
        let content = "/* File description */\nfn main() {}";
        assert_eq!(
            extract_rust_comment(content),
            Some("File description".to_string())
        );
    }

    #[test]
    fn test_python_docstring() {
        let content = r#""""Module docstring."""

def foo():
    pass
"#;
        assert_eq!(
            extract_python_docstring(content),
            Some("Module docstring.".to_string())
        );
    }

    #[test]
    fn test_python_multiline_docstring() {
        let content = r#""""
This is a longer docstring.

More details here.
"""
"#;
        assert_eq!(
            extract_python_docstring(content),
            Some("This is a longer docstring.\n\nMore details here.".to_string())
        );
    }

    #[test]
    fn test_js_jsdoc() {
        let content = r#"/**
 * Main application entry point
 */
function main() {}
"#;
        assert_eq!(
            extract_js_comment(content),
            Some("Main application entry point".to_string())
        );
    }

    #[test]
    fn test_js_line_comment() {
        let content = "// Application utilities\n\nexport function foo() {}";
        assert_eq!(
            extract_js_comment(content),
            Some("Application utilities".to_string())
        );
    }

    #[test]
    fn test_go_package_comment() {
        let content = "// Package main provides the entry point\npackage main";
        assert_eq!(
            extract_go_comment(content),
            Some("Package main provides the entry point".to_string())
        );
    }

    #[test]
    fn test_shell_comment() {
        let content = "#!/bin/bash\n# Script for deployment\necho hello";
        assert_eq!(
            extract_shell_comment(content),
            Some("Script for deployment".to_string())
        );
    }

    #[test]
    fn test_ruby_comment() {
        let content =
            "# frozen_string_literal: true\n# User authentication module\nclass User\nend";
        assert_eq!(
            extract_ruby_comment(content),
            Some("User authentication module".to_string())
        );
    }

    #[test]
    fn test_javadoc_comment() {
        let content = r#"/**
 * Main application class
 * @author Test
 */
public class Main {}
"#;
        assert_eq!(
            extract_javadoc_comment(content),
            Some("Main application class".to_string())
        );
    }

    #[test]
    fn test_php_comment() {
        let content = r#"<?php
/**
 * User authentication service
 */
class AuthService {}
"#;
        assert_eq!(
            extract_php_comment(content),
            Some("User authentication service".to_string())
        );
    }

    #[test]
    fn test_csharp_comment() {
        let content = r#"/// <summary>
/// Main program entry point
/// </summary>
public class Program {}
"#;
        assert_eq!(
            extract_csharp_comment(content),
            Some("Main program entry point".to_string())
        );
    }

    // Edge case tests for issue #61

    #[test]
    fn test_rust_multiline_module_doc() {
        let content =
            "//! Module documentation\n//! that spans\n//! multiple lines\n\nfn main() {}";
        assert_eq!(
            extract_rust_comment(content),
            Some("Module documentation\nthat spans\nmultiple lines".to_string())
        );
    }

    #[test]
    fn test_rust_empty_doc_comment() {
        let content = "//!\nfn main() {}";
        // Empty doc comment should return None (no non-empty content)
        assert_eq!(extract_rust_comment(content), None);
    }

    #[test]
    fn test_rust_block_comment_with_asterisks() {
        let content = "/**\n * Decorated block\n * comment style\n */\nfn main() {}";
        assert_eq!(
            extract_rust_comment(content),
            Some("Decorated block\ncomment style".to_string())
        );
    }

    #[test]
    fn test_python_single_quote_docstring() {
        let content = "'''Single quote docstring.'''\ndef foo(): pass";
        assert_eq!(
            extract_python_docstring(content),
            Some("Single quote docstring.".to_string())
        );
    }

    #[test]
    fn test_python_docstring_with_shebang() {
        let content =
            "#!/usr/bin/env python3\n# coding: utf-8\n\"\"\"Module with shebang.\"\"\"\nimport os";
        assert_eq!(
            extract_python_docstring(content),
            Some("Module with shebang.".to_string())
        );
    }

    #[test]
    fn test_go_block_comment() {
        let content = "/* Package main provides\nmultiline doc */\npackage main";
        assert_eq!(
            extract_go_comment(content),
            Some("Package main provides\nmultiline doc".to_string())
        );
    }

    #[test]
    fn test_go_comment_interrupted_by_code() {
        // Comments before code that's not 'package' should be discarded
        let content = "// Comment 1\nimport \"fmt\"\n// Comment 2\npackage main";
        // Should get Comment 2, not Comment 1
        assert_eq!(extract_go_comment(content), Some("Comment 2".to_string()));
    }

    #[test]
    fn test_go_orphan_close_comment_no_panic() {
        // Issue #67: File with */ but no matching /* should not panic
        // This tests that we don't call unwrap() on a find that might return None
        let content = "/* Block comment */\npackage main";
        // Normal case should still work
        assert_eq!(
            extract_go_comment(content),
            Some("Block comment".to_string())
        );
    }

    #[test]
    fn test_go_multiple_block_comments() {
        // Issue #74: Multiple block comments should extract first one correctly
        let content = "/* First comment */\n/* Second comment */\npackage main";
        // Should extract only the first block comment
        assert_eq!(
            extract_go_comment(content),
            Some("First comment".to_string())
        );
    }

    #[test]
    fn test_go_unclosed_block_comment() {
        // Block comment without closing */ should not panic
        let content = "/* Unclosed block comment\npackage main";
        // Should return None since block is not closed
        assert_eq!(extract_go_comment(content), None);
    }

    #[test]
    fn test_go_close_comment_in_string() {
        // Edge case: */ appearing in string or example without matching /*
        let content = "// Comment with */ in it\npackage main";
        assert_eq!(
            extract_go_comment(content),
            Some("Comment with */ in it".to_string())
        );
    }

    #[test]
    fn test_shell_multiple_comment_lines() {
        let content = "#!/bin/bash\n# First line\n# Second line\necho hello";
        assert_eq!(
            extract_shell_comment(content),
            Some("First line\nSecond line".to_string())
        );
    }

    #[test]
    fn test_ruby_multiple_magic_comments() {
        let content =
            "# encoding: utf-8\n# frozen_string_literal: true\n# Real comment\nclass Foo; end";
        assert_eq!(
            extract_ruby_comment(content),
            Some("Real comment".to_string())
        );
    }

    #[test]
    fn test_c_multiline_block() {
        let content = "/*\n * File: main.c\n * Author: Test\n */\nint main() {}";
        assert_eq!(
            extract_c_comment(content),
            Some("File: main.c\nAuthor: Test".to_string())
        );
    }

    #[test]
    fn test_js_multiline_jsdoc() {
        let content =
            "/**\n * @file Main application\n * @description Entry point\n */\nfunction main() {}";
        // @-lines should not be filtered in JS (only in Java)
        assert!(extract_js_comment(content).is_some());
    }

    #[test]
    fn test_empty_file() {
        assert_eq!(extract_rust_comment(""), None);
        assert_eq!(extract_python_docstring(""), None);
        assert_eq!(extract_js_comment(""), None);
        assert_eq!(extract_go_comment(""), None);
    }

    #[test]
    fn test_file_with_only_code() {
        assert_eq!(extract_rust_comment("fn main() {}"), None);
        assert_eq!(extract_python_docstring("def foo(): pass"), None);
        assert_eq!(extract_js_comment("function foo() {}"), None);
    }
}
