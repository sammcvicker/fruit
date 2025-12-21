# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **BREAKING**: Renamed JSON field from `comment` to `comments` for consistency (#124)
  - JSON output now uses `comments` (plural) instead of `comment` (singular)
  - Aligns with other plural fields: `types`, `todos`, `imports`
  - This is a breaking change for JSON consumers who parse the output
  - Migration: Update JSON parsing code to use `comments` instead of `comment`

### Added

- Added negation flags for symmetric feature control (#120)
  - `--no-types`: Explicitly disable type extraction
  - `--no-todos`: Explicitly disable TODO marker extraction
  - `--no-imports`: Explicitly disable import extraction
  - Complements existing `--no-comments` flag for consistent CLI interface
  - Enables clearer intent and future config file support

### Changed

- Organized help text into logical groups for better discoverability (#123)
  - Flags are now grouped by purpose: Tree Display, Metadata Extraction, Filtering, Output Format, Performance, Statistics
  - Makes `--help` output easier to scan and understand feature relationships
  - No behavior changes, only improved CLI documentation
- **BREAKING**: Metadata flags are now additive and independent (#119)
  - `-t` (types) now ADDS type information without hiding comments (previously showed only types)
  - `--todos` now ADDS TODO markers without hiding comments (previously showed only TODOs when used alone)
  - Default behavior unchanged: comments are still shown by default
  - Use `--no-comments` to explicitly disable comments when using `-t` or other flags
  - This makes flag behavior predictable and eliminates the complex interaction logic
  - **Migration**: If you used `-t` alone expecting no comments, add `--no-comments` explicitly
- `--todos-only` now automatically implies `--todos` (#121)
  - Users no longer need to specify both `--todos` and `--todos-only`
  - Old syntax `--todos --todos-only` still works for backward compatibility
  - New simpler syntax: just use `--todos-only`
- Improved clarity of `--newer` and `--older` flag help text (#122)
  - Replaced awkward phrasing "more recently than DURATION ago" with clearer "within the last DURATION"
  - Added examples in help text: "e.g., 7d for last week" and "e.g., 30d for older than a month"
  - No behavior changes, only improved documentation for better user experience
- Documented implicit full mode behavior in help text for `-t`, `--todos`, and `-i` flags (#118)
  - These flags now clearly state "(enables full output mode)" in their help descriptions
  - Users can now understand why output changes when using these metadata flags
  - No behavior changes, only improved documentation

### Added

- Rust type extraction now captures impl blocks and associated functions (#117)
  - Extracts `impl Type` blocks and their methods
  - Extracts `impl Trait for Type` blocks and their trait implementations
  - Captures both `pub fn` and private `fn` methods inside impl blocks
  - Supports generic impl blocks (e.g., `impl<T: Clone> Container<T>`)
  - Handles async functions within impl blocks
  - Methods are properly indented relative to their impl block

### Fixed

- Python type extraction now captures decorated functions (#116)
  - Functions with `@property`, `@staticmethod`, `@classmethod`, and other decorators are now extracted
  - Decorators are included in the signature display (e.g., `@property def name(self) -> str`)
  - Multiple decorators are supported and displayed in order
  - Private decorated functions (starting with `_`) are still correctly skipped

### Changed

- Added custom Default implementation for WalkerConfig with sensible defaults (#114)
  - `extract_comments` now defaults to `true` (matches default CLI behavior)
  - Using `..Default::default()` pattern in main.rs improves maintainability
  - New config fields will automatically get default values without breaking existing code
- Deduplicated metadata rendering methods between StreamingFormatter and TreeFormatter (#112)
  - Extracted `write_rendered_line()`, `write_inline_content()`, and `print_metadata_block()` into shared utility functions
  - Both formatters now use the same rendering logic from `output/utils.rs`
  - Improves maintainability: bug fixes and formatting changes only need to be made in one place
  - Better extensibility: new formatters can reuse existing rendering utilities
- Centralized language detection and extension mapping into `Language` enum (#109)
  - Created new `language.rs` module with `Language` enum for all supported languages
  - Removed duplicated extension-to-language mappings from `comments.rs`, `types.rs`, `imports.rs`, and `file_utils.rs`
  - Extension detection now uses single source of truth: `Language::from_extension()`
  - Improves maintainability: adding a new language now requires changes in one place only
  - Better extensibility: clearer API for future plugin support

### Performance

- Removed unnecessary HashSet clone in GitignoreFilter::new() (#115)

### Added

- `--max-file-size` flag to configure maximum file size for metadata extraction (#76)
  - Default remains 1MB, use suffixes like `5M`, `100K`, `1G` to customize
  - Files larger than the limit are skipped to prevent excessive memory usage
  - Example: `fruit --max-file-size 5M` to allow files up to 5MB

### Changed

- Modularized `output.rs` into separate submodules for better maintainability (#70)
  - `output/config.rs` - Output configuration types
  - `output/utils.rs` - Shared utility functions (text wrapping, prefix calculation)
  - `output/tree.rs` - Buffered tree formatter
  - `output/streaming.rs` - Streaming console formatter
  - `output/markdown.rs` - Markdown output formatter
  - `output/json.rs` - JSON output
- Modularized `tree.rs` into separate submodules for better maintainability (#71)
  - `tree/config.rs` - WalkerConfig type
  - `tree/filter.rs` - FileFilter enum
  - `tree/json_types.rs` - JSON serialization types (JsonTodoItem, TreeNode)
  - `tree/walker.rs` - TreeWalker implementation
  - `tree/streaming.rs` - StreamingWalker implementation
  - `tree/utils.rs` - Shared utilities (glob matching, file size formatting)
- Simplified duration parsing to use `humantime` crate directly, removing redundant custom parsing (#64)
- Consolidated file-reading logic into shared `file_utils` module (#58, #65)
- Aligned plain text metadata block formatting with colored output to ensure consistent group separators (#60)
- Extended Python standard library list with comprehensive module coverage (#63)

### Fixed

- TreeFormatter now displays type signatures, TODOs, and imports in JSON/buffered output mode (#108)
  - Previously only comments were shown; now all metadata types are properly rendered
  - Unified metadata block construction ensures consistency with streaming output
- Go block comment extraction no longer panics on edge cases with `*/` (#67)
- TODO marker extraction now uses `unwrap_or_else` instead of fragile `unwrap()` (#68)
- Test code now uses `expect()` with descriptive messages instead of bare `unwrap()` (#69)
- Regex `unwrap()` calls in lazy statics now use `expect()` with descriptive messages (#75)
- Go block comment extraction now correctly handles multiple block comments (#74)
- Test utilities now use descriptive panic messages with context (method name, paths, errors) (#72)
- Added integration tests for large file handling and git edge cases (#73)
- `MetadataBlock.total_lines()` now includes import lines in the count (#59)
- Removed unused `repo_root` field from `GitFilter` struct (#62)
- Removed unused `LineStyle` variants (`ClassName`, `MethodName`, `Docstring`) from metadata.rs (#57)

### Added

- `--newer` and `--older` flags to filter files by modification time (#50)
  - `--newer 1h` shows only files modified within the last hour
  - `--older 7d` shows only files not modified in the last 7 days
  - Supports duration formats: `30s`, `5m`, `1h`, `7d`, `2w`, `3M`, `1y`
  - Combine filters: `--newer 7d --older 1d` for files between 1-7 days old
  - Useful for finding recently changed code or detecting stale files
- `--imports` / `-i` flag to show import/dependency statements (#49)
  - Extracts imports from Rust, TypeScript, JavaScript, Python, and Go files
  - Categorizes imports as external (packages), std (standard library), or internal (project)
  - Console output shows `imports: clap, serde, std::{path, io}, crate::{git, tree}`
  - JSON output includes categorized `imports` object with `external`, `std`, `internal` arrays
  - Displayed in magenta for easy visual distinction
- `--size` / `-s` flag to display file sizes (#48)
  - Shows human-readable sizes (e.g., `1.2K`, `3.5M`) next to filenames
  - Displayed in green `[size]` brackets in console output
  - JSON output includes `size_bytes` and `size_human` fields
  - Markdown output shows size in parentheses: `` `file.txt` (1.2K) ``
- `--stats` flag to show codebase statistics (#47)
  - Displays file counts, directory counts, and line counts by language
  - `--stats --json` outputs statistics as JSON for scripting
  - `--stats --no-lines` skips line counting for faster output
  - Language detection based on file extension
- `--markdown` / `-m` flag for Markdown output format (#46)
  - Outputs tree as nested markdown list, ideal for documentation and LLM context
  - Directories shown in bold (`**name/**`), files in code spans (`` `name` ``)
  - Comments shown inline or as blockquotes in full mode
  - Supports combining with `-c`, `-t`, `--todos` for metadata
- `--todos` flag to extract and display TODO/FIXME/HACK/XXX/BUG/NOTE markers from comments (#45)
  - Shows task markers beneath file entries with line numbers
  - Combines with `-c` to show both comments and TODOs
  - JSON output includes `todos` array with `type`, `text`, and `line` fields
  - Requires colon after marker to reduce false positives (e.g., `// TODO: fix this`)
- Type signatures in JSON output when using `-t/--types` flag (#29)
  - `--json -t` now includes a `types` array in each file object
  - `--json -c -t` includes both `comment` and `types` fields
  - Maintains consistency between console and JSON output modes
- `-j/--jobs` flag for parallel metadata extraction (#22)
  - `-j0` (default): auto-detect CPU count, use all available cores
  - `-j1`: sequential mode (original behavior)
  - `-jN`: use N worker threads
  - Uses rayon for work-stealing parallelism
  - Output order preserved regardless of parallelism level
- Indentation hierarchy preserved in type signature display (#25)
  - Methods and nested items now display indented under their parent types
  - Source indentation is preserved (tabs normalized to 4 spaces)
  - Blank lines separate groups (classes with methods are visually distinct)
  - Makes class/method relationships visible at a glance
- `--types` / `-t` flag to show exported type signatures (#21)
  - Extracts public/exported APIs using regex patterns
  - Supported languages: Rust (`pub fn`, `pub struct`, etc.), TypeScript/JavaScript (`export`), Python (typed functions and classes), Go (capitalized exports)
  - When `-t` is specified alone, shows types only in full mode (all signatures)
  - Type signatures display in cyan with **bold red symbol names** for easy scanning
- `--comments` / `-c` flag to explicitly enable comments
  - Combine with types: `fruit -c -t` shows comments first, then types
  - Flag order determines display order: `-t -c` shows types first

- `--prefix` / `-p` flag to specify a custom prefix for metadata lines (#24)
  - Example: `fruit --prefix "# "` for hash prefix, `fruit -p "// "` for C-style
- Generic metadata block abstraction for extensible file info display (#19)
  - `MetadataBlock` and `MetadataLine` types for structured metadata
  - `MetadataExtractor` trait for pluggable metadata sources
  - `CommentExtractor` implementation for existing comment extraction
  - `LineStyle` enum for per-line coloring (enables future type signatures, etc.)
  - Foundation for future features: type signatures, code structure display

### Changed

- `-t/--types` alone now implies full mode and types-only (no comments unless `-c` added)
- Metadata display respects flag order with blank line separator between sections (#21)
  - `-c -t` shows comments first, `-t -c` shows types first
- Single-line first metadata always displays inline (to the right of filename)
- Redesigned full comment display (`-f`) to use metadata block pattern (#18)
  - Comments now appear on separate lines beneath the filename
  - Each comment line has its own `#` prefix for clarity
  - Multi-line comments get a visual buffer line for separation
  - Default mode (inline first-line) unchanged
- Refactored `OutputConfig` to use `MetadataConfig` for cleaner configuration (#19)

## [0.2.0] - 2025-12-15

### Changed

- Switch to `.gitignore` pattern-based filtering by default (#16)
  - Now respects `.gitignore` patterns directly instead of git tracking status
  - Nested `.gitignore` files are properly handled
  - Global gitignore (`~/.config/git/ignore`) is respected
  - Negation patterns (`!important.log`) work correctly
  - Untracked files are now shown (unless ignored by `.gitignore`)
  - Uses the `ignore` crate (from ripgrep) for battle-tested performance

### Added

- Criterion benchmarks for performance testing (#13)
  - Comment extraction benchmarks for Rust, Python, JavaScript, Go, and Java
  - Git filter initialization benchmarks (10, 100, 500 files)
  - `is_tracked` lookup benchmarks for files and directories
  - Run with `cargo bench`
- Performance regression test (1000 files under 10 seconds) (#13)
- Comprehensive edge case and error handling test suite (#14)
  - Symlink handling tests (to file, to directory, to parent, broken, self-referential)
  - Permission error handling tests (unreadable directories and files)
  - Special filename tests (spaces, unicode, special characters)
  - Comment extraction edge cases (empty files, binary files, unknown extensions)
  - Output edge cases (deep nesting, many files, wrap width edge cases)

- Initial tree command with git-aware filtering (respects .gitignore)
- Automatic comment extraction for source files:
  - Rust (`//!`, `///`, `/*! */`)
  - Python (docstrings)
  - JavaScript/TypeScript (JSDoc, `//`)
  - Shell/Ruby (`#`)
  - Go (`// Package`)
  - Java/Kotlin (Javadoc `/** */`)
  - Swift (`///`)
  - PHP (`/** */`)
  - C# (`///`)
- Full comment display mode (`-f`) with text wrapping
- Depth limiting (`-L`)
- Directories-only mode (`-d`)
- Ignore patterns (`-I`)
- JSON output format (`--json`) for machine-readable output (#11)
- TTY auto-detection with `--color` flag (auto/always/never) (#17)
- Warning when running outside a git repository (#5)

### Fixed

- Skip files larger than 1MB for comment extraction to avoid slowdowns (#1)
- Skip symlinks to prevent infinite loops and directory traversal issues (#3)
- UTF-8 character-aware text wrapping (handles emoji and CJK correctly) (#4)
- Proper glob pattern matching using the `glob` crate (#7)

### Performance

- O(1) directory tracking instead of O(n) path prefix scanning (#8)
- Streaming output for console display - 12.7x faster on large repositories (#2)
  - Memory usage reduced from O(files) to O(depth) for tree structure
  - Tested: 294ms → 23ms on a large codebase

### Changed

- Removed unused `walkdir` dependency (#6)
