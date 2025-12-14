//! Fruit - A tree command that respects .gitignore and shows file comments

pub mod comments;
pub mod git;
pub mod output;
pub mod tree;

pub use comments::extract_first_comment;
pub use git::GitFilter;
pub use output::{print_json, OutputConfig, StreamingFormatter, TreeFormatter};
pub use tree::{StreamingOutput, StreamingWalker, TreeNode, TreeWalker, WalkerConfig};
