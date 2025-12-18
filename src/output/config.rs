//! Output configuration types

use crate::metadata::MetadataConfig;

const DEFAULT_WRAP_WIDTH: usize = 100;

/// Configuration for output formatting.
#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub use_color: bool,
    /// Metadata display configuration
    pub metadata: MetadataConfig,
    pub wrap_width: Option<usize>,
}

impl OutputConfig {
    /// Check if full metadata blocks should be shown (vs first line only).
    pub fn show_full(&self) -> bool {
        self.metadata.full
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            use_color: true,
            metadata: MetadataConfig::comments_only(false),
            wrap_width: Some(DEFAULT_WRAP_WIDTH),
        }
    }
}
