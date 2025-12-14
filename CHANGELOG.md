# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added

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
