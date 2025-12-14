//! Source file comment extraction

use std::path::Path;

/// Maximum file size for comment extraction (1MB)
const MAX_FILE_SIZE: u64 = 1_000_000;

pub fn extract_first_comment(path: &Path) -> Option<String> {
    // Skip files that are too large to avoid OOM on large files
    if let Ok(metadata) = path.metadata() {
        if metadata.len() > MAX_FILE_SIZE {
            return None;
        }
    }

    let extension = path.extension()?.to_str()?;
    let content = std::fs::read_to_string(path).ok()?;

    match extension {
        "rs" => extract_rust_comment(&content),
        "py" => extract_python_docstring(&content),
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => extract_js_comment(&content),
        "go" => extract_go_comment(&content),
        "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" => extract_c_comment(&content),
        "rb" => extract_ruby_comment(&content),
        "sh" | "bash" | "zsh" => extract_shell_comment(&content),
        // Java, Kotlin, Swift use JavaDoc-style /** */ comments
        "java" | "kt" | "kts" | "swift" => extract_javadoc_comment(&content),
        // PHP uses PHPDoc /** */ and also # comments
        "php" => extract_php_comment(&content),
        // C# uses /// XML doc comments
        "cs" => extract_csharp_comment(&content),
        _ => None,
    }
}

fn extract_rust_comment(content: &str) -> Option<String> {
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
        } else if in_doc_comment {
            break;
        } else if !trimmed.is_empty()
            && !trimmed.starts_with("//")
            && !trimmed.starts_with("#[")
            && !trimmed.starts_with("#![")
        {
            break;
        }
    }
    if !doc_lines.is_empty() && doc_lines.iter().any(|l| !l.is_empty()) {
        return Some(doc_lines.join("\n"));
    }

    // Look for /* */ block comments at the top
    let trimmed = content.trim_start();
    if trimmed.starts_with("/*") {
        if let Some(end) = trimmed.find("*/") {
            let block = &trimmed[2..end];
            let cleaned: Vec<&str> = block
                .lines()
                .map(|l| l.trim().trim_start_matches('*').trim())
                .filter(|l| !l.is_empty())
                .collect();
            if !cleaned.is_empty() {
                return Some(cleaned.join("\n"));
            }
        }
    }

    None
}

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

fn extract_js_comment(content: &str) -> Option<String> {
    let trimmed = content.trim_start();

    // Check for JSDoc /** ... */
    if trimmed.starts_with("/**") {
        if let Some(end) = trimmed.find("*/") {
            let block = &trimmed[3..end];
            let cleaned: Vec<&str> = block
                .lines()
                .map(|l| l.trim().trim_start_matches('*').trim())
                .filter(|l| !l.is_empty() && *l != "/")
                .collect();
            if !cleaned.is_empty() {
                return Some(cleaned.join("\n"));
            }
        }
    }

    // Check for // comments at the top - collect all consecutive
    let mut comment_lines = Vec::new();
    for line in trimmed.lines() {
        let t = line.trim();
        if t.starts_with("//") {
            let comment = t.strip_prefix("//").unwrap_or("").trim();
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

fn extract_go_comment(content: &str) -> Option<String> {
    // Go package comments come before the package declaration
    let mut comment_lines: Vec<&str> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            let comment = trimmed.strip_prefix("//").unwrap_or("").trim();
            comment_lines.push(comment);
        } else if trimmed.starts_with("/*") {
            // Block comment
            if let Some(end_idx) = content.find("*/") {
                let start_idx = content.find("/*").unwrap();
                let block = &content[start_idx + 2..end_idx];
                let cleaned: Vec<&str> = block
                    .lines()
                    .map(|l| l.trim().trim_start_matches('*').trim())
                    .filter(|l| !l.is_empty())
                    .collect();
                if !cleaned.is_empty() {
                    return Some(cleaned.join("\n"));
                }
            }
            break;
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

fn extract_c_comment(content: &str) -> Option<String> {
    let trimmed = content.trim_start();

    // Block comment /* */
    if trimmed.starts_with("/*") {
        if let Some(end) = trimmed.find("*/") {
            let block = &trimmed[2..end];
            let cleaned: Vec<&str> = block
                .lines()
                .map(|l| l.trim().trim_start_matches('*').trim())
                .filter(|l| !l.is_empty())
                .collect();
            if !cleaned.is_empty() {
                return Some(cleaned.join("\n"));
            }
        }
    }

    // Line comments //
    let mut comment_lines = Vec::new();
    for line in trimmed.lines() {
        let t = line.trim();
        if t.starts_with("//") {
            let comment = t.strip_prefix("//").unwrap_or("").trim();
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

fn extract_ruby_comment(content: &str) -> Option<String> {
    let mut comment_lines = Vec::new();
    let mut past_preamble = false;

    for line in content.lines() {
        let trimmed = line.trim();
        // Skip shebang
        if trimmed.starts_with("#!") {
            continue;
        }
        // Skip encoding/frozen string comments
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

fn extract_shell_comment(content: &str) -> Option<String> {
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

fn extract_javadoc_comment(content: &str) -> Option<String> {
    let trimmed = content.trim_start();

    // Check for JavaDoc/KDoc/Swift doc /** ... */
    if trimmed.starts_with("/**") {
        if let Some(end) = trimmed.find("*/") {
            let block = &trimmed[3..end];
            let cleaned: Vec<&str> = block
                .lines()
                .map(|l| l.trim().trim_start_matches('*').trim())
                .filter(|l| !l.is_empty() && !l.starts_with('@'))
                .collect();
            if !cleaned.is_empty() {
                return Some(cleaned.join("\n"));
            }
        }
    }

    // Check for // comments at the top
    let mut comment_lines = Vec::new();
    for line in trimmed.lines() {
        let t = line.trim();
        if t.starts_with("//") {
            let comment = t.strip_prefix("//").unwrap_or("").trim();
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

fn extract_php_comment(content: &str) -> Option<String> {
    // Skip <?php tag
    let content = content.trim_start();
    let content = if content.starts_with("<?php") {
        &content[5..]
    } else if content.starts_with("<?") {
        &content[2..]
    } else {
        content
    };
    let trimmed = content.trim_start();

    // Check for PHPDoc /** ... */
    if trimmed.starts_with("/**") {
        if let Some(end) = trimmed.find("*/") {
            let block = &trimmed[3..end];
            let cleaned: Vec<&str> = block
                .lines()
                .map(|l| l.trim().trim_start_matches('*').trim())
                .filter(|l| !l.is_empty() && !l.starts_with('@'))
                .collect();
            if !cleaned.is_empty() {
                return Some(cleaned.join("\n"));
            }
        }
    }

    // Check for // or # comments at the top
    let mut comment_lines = Vec::new();
    for line in trimmed.lines() {
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

fn extract_csharp_comment(content: &str) -> Option<String> {
    let trimmed = content.trim_start();

    // C# uses /// for XML doc comments
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
    if trimmed.starts_with("/*") {
        if let Some(end) = trimmed.find("*/") {
            let block = &trimmed[2..end];
            let cleaned: Vec<&str> = block
                .lines()
                .map(|l| l.trim().trim_start_matches('*').trim())
                .filter(|l| !l.is_empty())
                .collect();
            if !cleaned.is_empty() {
                return Some(cleaned.join("\n"));
            }
        }
    }

    None
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
}
