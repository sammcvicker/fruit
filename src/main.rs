//! CLI entry point for fruit

use std::path::PathBuf;
use std::process;

use clap::Parser;
use fruit::{GitFilter, OutputConfig, TreeFormatter, TreeWalker, WalkerConfig};

#[derive(Parser, Debug)]
#[command(name = "fruit")]
#[command(about = "A tree command that respects .gitignore and shows file comments")]
#[command(version)]
struct Args {
    /// Directory to display
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Show all files (ignore git filtering)
    #[arg(short, long)]
    all: bool,

    /// Descend only N levels deep
    #[arg(short = 'L', long = "level")]
    level: Option<usize>,

    /// List directories only
    #[arg(short = 'd', long = "dirs-only")]
    dirs_only: bool,

    /// Show full comment, not just first line
    #[arg(short = 'f', long = "full-comment")]
    full_comment: bool,

    /// Ignore files matching pattern (can be used multiple times)
    #[arg(short = 'I', long = "ignore")]
    ignore: Vec<String>,

    /// Disable colorized output
    #[arg(long = "no-color")]
    no_color: bool,

    /// Disable comment extraction
    #[arg(long = "no-comments")]
    no_comments: bool,

    /// Wrap comments at column width (default: 100, 0 to disable)
    #[arg(short = 'w', long = "wrap", default_value = "100")]
    wrap: usize,
}

fn main() {
    let args = Args::parse();

    let walker_config = WalkerConfig {
        show_all: args.all,
        max_depth: args.level,
        dirs_only: args.dirs_only,
        extract_comments: !args.no_comments,
        ignore_patterns: args.ignore,
    };

    let mut walker = TreeWalker::new(walker_config);

    // Set up git filter unless --all is specified
    if !args.all {
        if let Some(filter) = GitFilter::new(&args.path) {
            walker = walker.with_git_filter(filter);
        }
    }

    let root = if args.path.is_absolute() {
        args.path.clone()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&args.path)
    };

    let tree = match walker.walk(&root) {
        Some(t) => t,
        None => {
            eprintln!("fruit: cannot access '{}': No such file or directory", args.path.display());
            process::exit(1);
        }
    };

    let output_config = OutputConfig {
        use_color: !args.no_color,
        show_full_comment: args.full_comment,
        wrap_width: if args.wrap == 0 { None } else { Some(args.wrap) },
    };

    let formatter = TreeFormatter::new(output_config);
    if let Err(e) = formatter.print(&tree) {
        eprintln!("fruit: error writing output: {}", e);
        process::exit(1);
    }
}
