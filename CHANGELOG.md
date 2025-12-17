# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `--prefix` / `-p` flag to specify a custom prefix for metadata lines (#24)
  - Example: `fruit --prefix "# "` for hash prefix, `fruit -p "// "` for C-style
- Generic metadata block abstraction for extensible file info display (#19)
  - `MetadataBlock` and `MetadataLine` types for structured metadata
  - `MetadataExtractor` trait for pluggable metadata sources
  - `CommentExtractor` implementation for existing comment extraction
  - `LineStyle` enum for per-line coloring (enables future type signatures, etc.)
  - Foundation for future features: type signatures, code structure display

### Changed

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
