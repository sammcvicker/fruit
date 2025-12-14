# Claude Code Instructions for fruit

## Project Overview

`fruit` is a tree command that respects .gitignore and shows file comments. It's written in Rust.

## Build & Test

```bash
cargo build          # Development build
cargo build --release # Release build
cargo test           # Run all tests
cargo run -- [args]  # Run with arguments
```

## Code Style

- Follow existing patterns in the codebase
- Use `cargo fmt` before committing
- Ensure `cargo clippy` passes without warnings

## Changelog Requirements

**ALWAYS update `CHANGELOG.md` before committing if the change is user-facing.**

Add entries under the `[Unreleased]` section (or current version) using these categories:
- **Added**: New features or capabilities
- **Changed**: Changes to existing functionality
- **Fixed**: Bug fixes
- **Performance**: Speed or memory improvements (include benchmark numbers when available)
- **Deprecated**: Features that will be removed
- **Removed**: Features that were removed

**Skip changelog updates for:**
- Internal refactoring with no user-visible changes
- Documentation-only changes (unless it's new user-facing docs)
- Test-only changes

**Example entry:**
```markdown
### Fixed

- Skip files larger than 1MB for comment extraction to avoid slowdowns (#1)
```

## Git Workflow

- Work on feature branches: `issue-<number>-<short-description>`
- Reference issue numbers in commits: `Fix #<number>: <description>`
- Merge to `develop` branch when complete

## Architecture Notes

- `src/tree.rs`: Tree walking logic, including `StreamingWalker` for memory-efficient console output
- `src/git.rs`: Git repository integration and file filtering
- `src/output.rs`: Formatting and display (console and JSON)
- `src/comments.rs`: Source file comment extraction
- `src/main.rs`: CLI entry point

Console output uses `StreamingWalker` for O(depth) memory usage.
JSON output uses `TreeWalker` which builds the full tree (required for serialization).
