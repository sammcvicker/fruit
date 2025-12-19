//! TODO/FIXME/HACK comment extraction
//!
//! This module extracts task markers from comments across source files.
//! Supported markers: TODO, FIXME, HACK, XXX, BUG, NOTE

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::file_utils::read_source_file;

/// Pattern matches TODO, FIXME, HACK, XXX, BUG, NOTE at the start of comment text
/// followed by colon and the actual message.
static TODO_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^\s*(?://+|/?\*+|#+|--+|;+)\s*!?\s*(TODO|FIXME|HACK|XXX|BUG|NOTE)\s*:\s*(.+)",
    )
    .expect("TODO_PATTERN regex is invalid")
});

/// A single TODO/FIXME marker extracted from a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoItem {
    /// The type of marker (TODO, FIXME, HACK, XXX, BUG, NOTE)
    pub marker_type: String,
    /// The text content after the marker
    pub text: String,
    /// The line number where this TODO was found (1-indexed)
    pub line: usize,
}

/// Extract all TODO/FIXME markers from a source file.
///
/// Returns a vector of `TodoItem` structs containing the marker type,
/// text, and line number for each task marker found.
///
/// # Supported Markers
///
/// - `TODO`: Tasks to be done
/// - `FIXME`: Code that needs fixing
/// - `HACK`: Temporary workarounds
/// - `XXX`: Problematic or unclear code
/// - `BUG`: Known bugs
/// - `NOTE`: Important notes
///
/// # Pattern Matching
///
/// Markers are matched with optional colon and surrounding text:
/// - `TODO: fix this` → type="TODO", text="fix this"
/// - `FIXME - memory leak` → type="FIXME", text="memory leak"
/// - `// TODO: implement` → type="TODO", text="implement"
pub fn extract_todos(path: &Path) -> Option<Vec<TodoItem>> {
    // read_source_file handles extension filtering and case-normalization
    let (content, _extension) = read_source_file(path)?;

    let todos = extract_todos_from_content(&content);

    if todos.is_empty() { None } else { Some(todos) }
}

/// Extract TODO items from file content.
fn extract_todos_from_content(content: &str) -> Vec<TodoItem> {
    let mut todos = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        // Skip lines that don't look like comments
        let trimmed = line.trim();
        if !looks_like_comment(trimmed) {
            continue;
        }

        if let Some(caps) = TODO_PATTERN.captures(line) {
            // Group 1 contains the marker type (TODO, FIXME, etc.)
            // Group 2 contains the text after the colon
            let marker_type = caps
                .get(1)
                .map(|m| m.as_str().to_uppercase())
                .unwrap_or_else(|| "TODO".to_string());
            let text = caps
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            // Skip if the text is empty or just contains closing comment markers
            let cleaned_text = clean_comment_text(&text);

            // Skip lines that look like documentation examples or descriptions
            // These typically start with backticks, mention "markers", or have special patterns
            if is_documentation_example(&cleaned_text) {
                continue;
            }

            if !cleaned_text.is_empty() {
                todos.push(TodoItem {
                    marker_type,
                    text: cleaned_text,
                    line: line_idx + 1, // 1-indexed
                });
            }
        }
    }

    todos
}

/// Check if text looks like a documentation example rather than an actual TODO.
fn is_documentation_example(text: &str) -> bool {
    // Skip text that starts with backticks (code examples)
    if text.starts_with('`') {
        return true;
    }
    // Skip text that mentions "marker" (describing the feature)
    if text.to_lowercase().contains("marker") {
        return true;
    }
    // Skip text that is just describing what TODO types mean
    if text.starts_with("Tasks to be done")
        || text.starts_with("Code that needs fixing")
        || text.starts_with("Temporary workaround")
        || text.starts_with("Problematic or unclear")
        || text.starts_with("Known bug")
        || text.starts_with("Important note")
    {
        return true;
    }
    false
}

/// Check if a line looks like it might be a comment.
/// This is a heuristic to avoid matching TODOs in string literals.
fn looks_like_comment(line: &str) -> bool {
    // Check for common comment prefixes
    line.starts_with("//")
        || line.starts_with('#')
        || line.starts_with('*')
        || line.starts_with("/*")
        || line.starts_with("--")
        || line.starts_with(';')
        || line.starts_with("(*")
        || line.starts_with("'''")
        || line.starts_with("\"\"\"")
        || line.contains("//")
        || line.contains("/*")
        || line.contains('#')
}

/// Clean up TODO text by removing trailing comment markers.
fn clean_comment_text(text: &str) -> String {
    let mut result = text.to_string();

    // Remove trailing */ and similar
    if let Some(idx) = result.find("*/") {
        result = result[..idx].trim().to_string();
    }

    // Remove trailing --> (XML comments)
    if let Some(idx) = result.find("-->") {
        result = result[..idx].trim().to_string();
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_todos_basic() {
        let content = r#"
// TODO: implement this function
fn foo() {}

// FIXME: memory leak here
fn bar() {}
"#;
        let todos = extract_todos_from_content(content);
        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].marker_type, "TODO");
        assert_eq!(todos[0].text, "implement this function");
        assert_eq!(todos[0].line, 2);
        assert_eq!(todos[1].marker_type, "FIXME");
        assert_eq!(todos[1].text, "memory leak here");
        assert_eq!(todos[1].line, 5);
    }

    #[test]
    fn test_extract_todos_all_markers() {
        let content = r#"
# TODO: task one
# FIXME: fix this
# HACK: temporary workaround
# XXX: problematic code
# BUG: known issue
# NOTE: important note
"#;
        let todos = extract_todos_from_content(content);
        assert_eq!(todos.len(), 6);
        assert_eq!(todos[0].marker_type, "TODO");
        assert_eq!(todos[1].marker_type, "FIXME");
        assert_eq!(todos[2].marker_type, "HACK");
        assert_eq!(todos[3].marker_type, "XXX");
        assert_eq!(todos[4].marker_type, "BUG");
        assert_eq!(todos[5].marker_type, "NOTE");
    }

    #[test]
    fn test_extract_todos_case_insensitive() {
        let content = r#"
// todo: lowercase
// Todo: mixed case
// TODO: uppercase
"#;
        let todos = extract_todos_from_content(content);
        assert_eq!(todos.len(), 3);
        // All should be normalized to uppercase
        assert!(todos.iter().all(|t| t.marker_type == "TODO"));
    }

    #[test]
    fn test_extract_todos_requires_colon() {
        // Without colon, TODOs should not be matched (prevents false positives)
        let content = r#"
// TODO implement without colon
// FIXME - with dash
"#;
        let todos = extract_todos_from_content(content);
        assert!(todos.is_empty());
    }

    #[test]
    fn test_extract_todos_with_colon() {
        let content = r#"
// TODO: implement with colon
// FIXME: fix with colon
"#;
        let todos = extract_todos_from_content(content);
        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].text, "implement with colon");
        assert_eq!(todos[1].text, "fix with colon");
    }

    #[test]
    fn test_extract_todos_block_comment() {
        let content = r#"
/* TODO: in block comment */
/*
 * FIXME: multi-line block
 */
"#;
        let todos = extract_todos_from_content(content);
        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].text, "in block comment");
        assert_eq!(todos[1].text, "multi-line block");
    }

    #[test]
    fn test_extract_todos_python_comments() {
        let content = r#"
# TODO: task in hash comment
# FIXME: another task
"#;
        let todos = extract_todos_from_content(content);
        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].text, "task in hash comment");
        assert_eq!(todos[1].text, "another task");
    }

    #[test]
    fn test_extract_todos_empty_text_skipped() {
        let content = r#"
// TODO:
// TODO
"#;
        let todos = extract_todos_from_content(content);
        assert!(todos.is_empty());
    }

    #[test]
    fn test_looks_like_comment() {
        assert!(looks_like_comment("// comment"));
        assert!(looks_like_comment("# comment"));
        assert!(looks_like_comment("/* comment */"));
        assert!(looks_like_comment("* in block comment"));
        assert!(looks_like_comment("code // inline comment"));
        assert!(looks_like_comment("code # python inline"));
    }

    #[test]
    fn test_clean_comment_text() {
        assert_eq!(clean_comment_text("text */"), "text");
        assert_eq!(clean_comment_text("text -->"), "text");
        assert_eq!(clean_comment_text("  spaced  "), "spaced");
    }

    // Edge case tests for issue #61

    #[test]
    fn test_todo_with_trailing_punctuation() {
        let content = "// TODO: fix this bug!!!\n";
        let todos = extract_todos_from_content(content);
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].text, "fix this bug!!!");
    }

    #[test]
    fn test_todo_inside_doc_comment() {
        // Doc comments with TODOs should still be captured
        let content = "/// TODO: document this function\nfn foo() {}";
        let todos = extract_todos_from_content(content);
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].text, "document this function");
    }

    #[test]
    fn test_multiple_todos_same_line_type() {
        let content = "// TODO: first\n// TODO: second\n// TODO: third";
        let todos = extract_todos_from_content(content);
        assert_eq!(todos.len(), 3);
        assert_eq!(todos[0].line, 1);
        assert_eq!(todos[1].line, 2);
        assert_eq!(todos[2].line, 3);
    }

    #[test]
    fn test_todo_preserves_line_numbers() {
        let content = "\n\n\n// TODO: on line 4\n\n// FIXME: on line 6";
        let todos = extract_todos_from_content(content);
        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].line, 4);
        assert_eq!(todos[1].line, 6);
    }

    #[test]
    fn test_note_marker() {
        let content = "# NOTE: important observation\n";
        let todos = extract_todos_from_content(content);
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].marker_type, "NOTE");
    }

    #[test]
    fn test_empty_content() {
        let todos = extract_todos_from_content("");
        assert!(todos.is_empty());
    }

    #[test]
    fn test_content_without_todos() {
        let content = "// Regular comment\nfn main() {}\n// Another comment";
        let todos = extract_todos_from_content(content);
        assert!(todos.is_empty());
    }
}
