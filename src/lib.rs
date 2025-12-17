//! Fruit - A tree command that respects .gitignore and shows file comments

pub mod comments;
pub mod git;
pub mod metadata;
pub mod output;
pub mod tree;
pub mod types;

pub use comments::extract_first_comment;
pub use git::{GitFilter, GitignoreFilter};
pub use metadata::{
    CommentExtractor, LineStyle, MetadataBlock, MetadataConfig, MetadataExtractor, MetadataLine,
    MetadataOrder,
};
pub use output::{OutputConfig, StreamingFormatter, TreeFormatter, print_json};
pub use tree::{FileFilter, StreamingOutput, StreamingWalker, TreeNode, TreeWalker, WalkerConfig};
pub use types::{TypeExtractor, extract_type_signatures};
