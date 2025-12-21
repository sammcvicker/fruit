# Fruit

Tree but just the juicy bits.

A `tree` replacement that understands your git repository and shows you what your code does at a glance.

## Features

- **Git-aware by default** - Only shows tracked files, hiding build artifacts, node_modules, and other noise
- **Comment extraction** - Displays the first line of module-level comments, so you can see what each file does
- **Full comment mode** - Show complete multiline comments with proper alignment and tree continuation
- **Text wrapping** - Long comments wrap at configurable column width

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

# Wrap comments at 80 columns
fruit -f -w 80
```

## Example Output

Basic output with first-line comments:

```
.
├── Cargo.toml        # A tree command that respects .gitignore
├── src
│   ├── main.rs       # CLI entry point
│   ├── lib.rs        # Core library exports
│   ├── git.rs        # Git repository integration
│   ├── tree.rs       # Directory tree walking logic
│   ├── comments.rs   # Source file comment extraction
│   └── output.rs     # Tree formatting and display
└── tests
    └── integration.rs

2 directories, 8 files
```

With `-f` for full multiline comments:

```
├── __init__.py  # Daemon-based embedding service for instant model loading.
│
│                  This package implements a persistent daemon process that keeps the embedding
│                  model loaded in memory, providing near-instant embeddings for CLI commands.
│
│                  Architecture:
│                  - protocol.py: JSON-RPC communication protocol
│                  - server.py: Daemon server process (keeps model loaded)
│                  - client.py: Client adapter (implements Embedder protocol)
│                  - lifecycle.py: Daemon lifecycle management (start/stop/status)
│
└── other.py     # Another module.
```

Comments are shown in dim text after the filename. Multiline comments maintain proper alignment with tree continuation characters.

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
  -w, --wrap <N>       Wrap comments at column width [default: 100, 0 to disable]
  -I, --ignore <PAT>   Ignore files matching pattern
      --no-color       Disable colorized output
      --no-comments    Disable comment extraction
  -h, --help           Print help
  -V, --version        Print version
```

## File Ordering

Files and directories within each directory are sorted **alphabetically** for deterministic, consistent output across all systems and filesystems. This ensures:

- Reproducible output regardless of underlying filesystem order
- Consistent diffs when comparing outputs
- Predictable file locations in large directory trees

## Comparison with tree

| Feature | tree | fruit |
|---------|------|-------|
| Basic tree display | Yes | Yes |
| Colorized output | Yes | Yes |
| Git-aware filtering | No | Yes (default) |
| Comment extraction | No | Yes |
| Multiline comments | No | Yes (-f) |
| Comment wrapping | No | Yes (-w) |
| Respects .gitignore | With flag | Default |
| Alphabetical sorting | Optional | Always |

## Supported Languages for Comments

| Language | Extensions | What's extracted |
|----------|------------|------------------|
| Rust | `.rs` | `//!` module docs, `///` item docs |
| Python | `.py` | Module docstrings (`"""..."""`) |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` | Top JSDoc or `//` comments |
| TypeScript | `.ts`, `.tsx` | Top JSDoc or `//` comments |
| Go | `.go` | Package documentation |
| C/C++ | `.c`, `.h`, `.cpp`, `.hpp`, `.cc`, `.cxx` | Top block or line comments |
| Ruby | `.rb` | Top `#` comments (after magic comments) |
| Shell | `.sh`, `.bash`, `.zsh` | `#` comments after shebang |

## Output Formats

### TODO Items

TODO items (extracted with `--todos`) have different structures depending on the output format:

**Console/Markdown output** (human-readable):
```
TODO: Fix this bug (line 42)
```
A single formatted string combining marker type, text, and line number for easy readability.

**JSON output** (machine-parseable):
```json
{
  "type": "TODO",
  "text": "Fix this bug",
  "line": 42
}
```
Structured with separate fields for programmatic access, allowing consumers to filter by type, extract line numbers, or format the text as needed.

This intentional difference serves the needs of each format: console output prioritizes human readability, while JSON output provides structured data for scripts and tools.

## License

MIT
