//! JSON serialization types for tree output

use std::path::PathBuf;

use serde::Serialize;

use crate::imports::FileImports;

/// Serializable TODO item for JSON output.
///
/// # Output Format Difference
///
/// JSON output provides structured fields (`type`, `text`, `line`) for programmatic access,
/// while console/markdown output combines these into a single formatted string
/// (e.g., "TODO: Fix this bug (line 42)") for human readability.
///
/// This intentional difference serves the needs of each format:
/// - JSON: Machine-parseable, allowing filtering by type, line number extraction, etc.
/// - Console: Human-readable, optimized for quick scanning
#[derive(Debug, Clone, Serialize)]
pub struct JsonTodoItem {
    #[serde(rename = "type")]
    pub marker_type: String,
    pub text: String,
    pub line: usize,
}

impl From<&crate::todos::TodoItem> for JsonTodoItem {
    fn from(item: &crate::todos::TodoItem) -> Self {
        Self {
            marker_type: item.marker_type.clone(),
            text: item.text.clone(),
            line: item.line,
        }
    }
}

/// TreeNode for JSON output - builds full tree in memory.
/// For large repos, use StreamingWalker instead for console output.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TreeNode {
    File {
        name: String,
        path: PathBuf,
        #[serde(skip_serializing_if = "Option::is_none")]
        comments: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        types: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        todos: Option<Vec<JsonTodoItem>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        imports: Option<FileImports>,
        #[serde(skip_serializing_if = "Option::is_none")]
        size_bytes: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        size_human: Option<String>,
    },
    Dir {
        name: String,
        path: PathBuf,
        children: Vec<TreeNode>,
    },
}

impl TreeNode {
    pub fn name(&self) -> &str {
        match self {
            TreeNode::File { name, .. } => name,
            TreeNode::Dir { name, .. } => name,
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, TreeNode::Dir { .. })
    }
}
