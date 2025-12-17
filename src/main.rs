//! CLI entry point for fruit

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process;

use clap::{Parser, ValueEnum};
use fruit::{
    GitignoreFilter, MetadataConfig, OutputConfig, StreamingFormatter, StreamingWalker, TreeWalker,
    WalkerConfig, print_json,
};

/// Color output mode
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum ColorMode {
    /// Auto-detect based on terminal and environment
    #[default]
    Auto,
    /// Always use colors
    Always,
    /// Never use colors
    Never,
}

/// Determine whether to use color output based on mode and environment.
fn should_use_color(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => {
            // Respect NO_COLOR environment variable (https://no-color.org/)
            if std::env::var_os("NO_COLOR").is_some() {
                return false;
            }
            // Respect FORCE_COLOR environment variable
            if std::env::var_os("FORCE_COLOR").is_some() {
                return true;
            }
            // Respect TERM=dumb
            if std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false) {
                return false;
            }
            // Check if stdout is a TTY
            std::io::stdout().is_terminal()
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "fruit")]
#[command(about = "A tree command that respects .gitignore and shows file comments")]
#[command(version)]
struct Args {
    /// Directory to display
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Show all files (ignore .gitignore filtering)
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

    /// Control color output: auto, always, never
    #[arg(long = "color", value_name = "WHEN", default_value = "auto")]
    color: ColorMode,

    /// Disable comment extraction
    #[arg(long = "no-comments")]
    no_comments: bool,

    /// Wrap comments at column width (default: 100, 0 to disable)
    #[arg(short = 'w', long = "wrap", default_value = "100")]
    wrap: usize,

    /// Output in JSON format
    #[arg(long = "json")]
    json: bool,
}

fn main() {
    let args = Args::parse();

    let walker_config = WalkerConfig {
        show_all: args.all,
        max_depth: args.level,
        dirs_only: args.dirs_only,
        extract_comments: !args.no_comments,
        ignore_patterns: args.ignore.clone(),
    };

    let root = if args.path.is_absolute() {
        args.path.clone()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&args.path)
    };

    // JSON output requires full tree in memory (for serialization)
    // Console output uses streaming to reduce memory usage
    let result = if args.json {
        let mut walker = TreeWalker::new(walker_config);

        // Set up gitignore filter unless --all is specified
        if !args.all {
            if let Some(filter) = GitignoreFilter::new(&args.path) {
                walker = walker.with_gitignore_filter(filter);
            } else {
                eprintln!("fruit: warning: not a git repository, showing all files");
            }
        }

        let tree = match walker.walk(&root) {
            Some(t) => t,
            None => {
                eprintln!(
                    "fruit: cannot access '{}': No such file or directory",
                    args.path.display()
                );
                process::exit(1);
            }
        };
        print_json(&tree)
    } else {
        // Use streaming walker for console output - much lower memory usage
        let mut walker = StreamingWalker::new(walker_config);

        // Set up gitignore filter unless --all is specified
        if !args.all {
            if let Some(filter) = GitignoreFilter::new(&args.path) {
                walker = walker.with_gitignore_filter(filter);
            } else {
                eprintln!("fruit: warning: not a git repository, showing all files");
            }
        }

        let output_config = OutputConfig {
            use_color: should_use_color(args.color),
            metadata: if args.no_comments {
                MetadataConfig::none()
            } else {
                MetadataConfig::comments_only(args.full_comment)
            },
            wrap_width: if args.wrap == 0 {
                None
            } else {
                Some(args.wrap)
            },
        };
        let mut formatter = StreamingFormatter::new(output_config);

        match walker.walk_streaming(&root, &mut formatter) {
            Ok(Some(_)) => Ok(()),
            Ok(None) => {
                eprintln!(
                    "fruit: cannot access '{}': No such file or directory",
                    args.path.display()
                );
                process::exit(1);
            }
            Err(e) => Err(e),
        }
    };

    if let Err(e) = result {
        eprintln!("fruit: error writing output: {}", e);
        process::exit(1);
    }
}
