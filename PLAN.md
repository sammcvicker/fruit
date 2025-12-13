# Fruit - Implementation Plan

## Overview

Fruit is a `tree` clone with two key enhancements:
1. Git-aware filtering (ignores untracked files by default)
2. Comment extraction (shows first line of module-level comments)

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                        CLI Layer                        │
│                    (clap argument parsing)              │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                      Core Library                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │  TreeWalker │  │ CommentParser│  │   GitFilter    │  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                    Output Formatter                     │
│         (tree-style output with colors/comments)        │
└─────────────────────────────────────────────────────────┘
```

## Module Breakdown

### `lib.rs` - Core Library Entry
- Re-exports all public types
- Keeps CLI separate from logic for testing

### `git.rs` - Git Integration
- `GitFilter` struct that wraps a git repository
- `is_tracked(&self, path: &Path) -> bool` - checks if path is tracked
- `tracked_entries(&self, dir: &Path) -> Vec<PathBuf>` - lists tracked items in a directory
- Uses `git2` crate for repository access

### `walker.rs` - Tree Walking
- `TreeWalker` struct with configuration
- `walk(&self, root: &Path) -> TreeNode` - returns hierarchical structure
- `TreeNode` enum: `File { name, comment }` or `Dir { name, children }`
- Respects git filtering when enabled

### `comments.rs` - Comment Extraction
- `extract_first_comment(path: &Path) -> Option<String>`
- Language detection by extension
- Supports: Rust, Python, JavaScript/TypeScript, Go, C/C++, Ruby, Shell, etc.
- Extracts first doc comment or module-level comment

### `output.rs` - Display Formatting
- `TreeFormatter` struct with style options
- `format(&self, node: &TreeNode) -> String`
- Tree drawing characters: `├──`, `└──`, `│`
- Color support via `termcolor` or similar
- Dim styling for comments

### `config.rs` - Configuration
- Command-line argument definitions
- Default behaviors
- Style options

## Comment Detection Rules

| Language | Extensions | Comment Patterns |
|----------|------------|------------------|
| Rust | `.rs` | `//!`, `///` at top, or `/* ... */` |
| Python | `.py` | `"""..."""` or `'''...'''` at top |
| JavaScript/TS | `.js`, `.ts`, `.jsx`, `.tsx` | `/** ... */` or `//` at top |
| Go | `.go` | `//` or `/* ... */` before package |
| C/C++ | `.c`, `.h`, `.cpp`, `.hpp` | `/* ... */` or `//` at top |
| Ruby | `.rb` | `#` at top |
| Shell | `.sh`, `.bash` | `#` after shebang |

"Top" means before any code (imports, declarations, etc.)

## CLI Interface

```
fruit [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory to display [default: .]

Options:
  -a, --all          Show all files (ignore git filtering)
  -L, --level <N>    Descend only N levels deep
  -d, --dirs-only    List directories only
  -f, --full-comment Show full comment, not just first line
  -I, --ignore <PAT> Ignore files matching pattern
  --no-color         Disable colorized output
  --no-comments      Disable comment extraction
  -h, --help         Print help
  -V, --version      Print version
```

## Testing Strategy

### Unit Tests
- `git.rs`: Mock git repos, test tracking detection
- `comments.rs`: Test each language's comment extraction
- `walker.rs`: Test tree building with mock filesystems
- `output.rs`: Test formatting output strings

### Integration Tests
- Create temporary directories with known structures
- Run full pipeline and compare output
- Test git integration with real git repos

### Test Harness
A `tests/harness.rs` module providing:
- `TestRepo::new()` - creates temp dir with git init
- `TestRepo::add_file(path, content)` - adds and stages file
- `TestRepo::add_untracked(path, content)` - adds without staging
- `TestRepo::run_fruit(args)` - runs fruit and captures output
- Snapshot testing for output comparison

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
git2 = "0.18"
termcolor = "1.4"
walkdir = "2"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

## Implementation Order

1. Project setup (Cargo.toml, basic structure)
2. `TreeNode` type and basic walker (no git, no comments)
3. Output formatter (tree display)
4. Git integration
5. Comment extraction (start with Rust, expand)
6. CLI with clap
7. Polish and edge cases

## Future Considerations

- `.fruitignore` file support
- Custom comment patterns via config
- JSON output mode
- Performance optimization for large repos
