# Fruit

A smarter `tree` for developers. Fruit is a drop-in replacement for `tree` that understands your git repository and shows you what your code does at a glance.

## Features

- **Git-aware by default** - Only shows tracked files, hiding build artifacts, node_modules, and other noise
- **Comment extraction** - Displays the first line of module-level comments, so you can see what each file does

## Installation

```bash
cargo install fruit
```

Or build from source:

```bash
git clone https://github.com/user/fruit
cd fruit
cargo build --release
```

## Usage

```bash
# Show tree of current directory (git-tracked files only)
fruit

# Show tree of a specific directory
fruit src/

# Show all files, including untracked
fruit -a

# Limit depth
fruit -L 2

# Show full comments instead of just first line
fruit -f
```

## Example Output

```
.
├── Cargo.toml        # A tree command that respects .gitignore
├── src
│   ├── main.rs       # CLI entry point
│   ├── lib.rs        # Core library exports
│   ├── git.rs        # Git repository integration
│   ├── walker.rs     # Directory tree walking logic
│   ├── comments.rs   # Source file comment extraction
│   └── output.rs     # Tree formatting and display
└── tests
    └── integration.rs

3 directories, 8 files
```

Comments are shown in dim text after the filename. The comment shown is the first line of:
- Rust: `//!` doc comments or `///` on the first item
- Python: Module docstrings (`"""..."""`)
- JavaScript/TypeScript: JSDoc comments (`/** ... */`)
- Go: Package comments
- And more...

## Options

```
Usage: fruit [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory to display [default: .]

Options:
  -a, --all            Show all files (ignore git filtering)
  -L, --level <N>      Descend only N levels deep
  -d, --dirs-only      List directories only
  -f, --full-comment   Show full comment, not just first line
  -I, --ignore <PAT>   Ignore files matching pattern
      --no-color       Disable colorized output
      --no-comments    Disable comment extraction
  -h, --help           Print help
  -V, --version        Print version
```

## Comparison with tree

| Feature | tree | fruit |
|---------|------|-------|
| Basic tree display | Yes | Yes |
| Colorized output | Yes | Yes |
| Git-aware filtering | No | Yes (default) |
| Comment extraction | No | Yes |
| Respects .gitignore | With flag | Default |

## Supported Languages for Comments

| Language | Extensions | What's extracted |
|----------|------------|------------------|
| Rust | `.rs` | `//!` module docs |
| Python | `.py` | Module docstrings |
| JavaScript | `.js`, `.jsx` | Top JSDoc or `//` comments |
| TypeScript | `.ts`, `.tsx` | Top JSDoc or `//` comments |
| Go | `.go` | Package documentation |
| C/C++ | `.c`, `.h`, `.cpp` | Top block or line comments |
| Ruby | `.rb` | Top `#` comments |
| Shell | `.sh`, `.bash` | `#` comments after shebang |

## License

MIT
