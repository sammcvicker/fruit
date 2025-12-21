//! Fruit - A tree command that respects .gitignore and shows file comments

// Legacy modules - kept for backward compatibility during transition
pub mod comments;
pub mod imports;
pub mod todos;
pub mod types;

// Core modules
pub mod extractors;
pub mod file_utils;
pub mod git;
pub mod language;
pub mod metadata;
pub mod output;
pub mod stats;
pub mod string_utils;
pub mod tree;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

// Re-exports from extractors module
pub use extractors::{
    ExtractionConfig,
    Extractor,
    comments::{CommentExtractor as ExtractorCommentExtractor, extract_first_comment},
    imports::{FileImports, ImportExtractor, extract_imports},
    todos::{TodoExtractor, TodoItem, extract_todos},
    types::{TypeExtractor, extract_type_signatures},
};

// Other re-exports
pub use git::GitignoreFilter;
pub use language::Language;
pub use metadata::{
    CommentExtractor, LineStyle, MetadataBlock, MetadataConfig,
    MetadataExtractor, MetadataLine, MetadataOrder,
};
pub use output::{
    MarkdownFormatter, OutputConfig, StreamingFormatter, TreeFormatter, print_json, print_markdown,
};
pub use stats::{
    CodebaseStats, LanguageStats, StatsCollector, StatsConfig, print_stats, print_stats_json,
};
pub use tree::{
    FileFilter, StreamingOutput, StreamingWalker, TreeNode, TreeWalker, WalkerConfig, format_size,
};
