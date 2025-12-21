//! Directory tree walking logic
//!
//! This module provides tree walking capabilities for displaying directory structures.
//! It supports two main modes:
//!
//! - `TreeWalker`: Builds full tree in memory, required for JSON output
//! - `StreamingWalker`: Streams output directly, uses O(depth) memory for console output

mod config;
mod filter;
mod json_types;
mod streaming;
mod traversal;
mod utils;
mod walker;

// Re-export public types
pub use config::WalkerConfig;
pub use filter::FileFilter;
pub use json_types::{JsonTodoItem, JsonTypeItem, TreeNode};
pub use streaming::{StreamingOutput, StreamingWalker};
pub use utils::format_size;
pub use walker::TreeWalker;

// Re-export MetadataOrder for convenience
pub use crate::metadata::MetadataOrder;
