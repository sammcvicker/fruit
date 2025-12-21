//! CLI entry point for fruit

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{Duration, SystemTime};

use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser, ValueEnum};
use fruit::{
    CodebaseStats, GitignoreFilter, MarkdownFormatter, MetadataConfig, MetadataOrder, OutputConfig,
    StatsCollector, StatsConfig, StreamingFormatter, StreamingWalker, TreeWalker, WalkerConfig,
    print_json, print_markdown, print_stats, print_stats_json,
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

    /// Show file comments (enabled by default unless -t is specified)
    #[arg(short = 'c', long = "comments")]
    comments: bool,

    /// Disable comment extraction (for backwards compatibility)
    #[arg(long = "no-comments", conflicts_with = "comments")]
    no_comments: bool,

    /// Show exported type signatures (functions, classes, interfaces, etc.)
    /// When specified alone, shows only types (enables full output mode)
    #[arg(short = 't', long = "types")]
    types: bool,

    /// Show TODO/FIXME/HACK/XXX markers from comments (enables full output mode)
    /// When specified, extracts task markers and displays them beneath file entries
    #[arg(long = "todos")]
    todos: bool,

    /// Show only files containing TODO/FIXME markers (requires --todos)
    #[arg(long = "todos-only", requires = "todos")]
    todos_only: bool,

    /// Show import/dependency statements from source files (enables full output mode)
    /// Extracts and categorizes imports (external, std, internal)
    #[arg(short = 'i', long = "imports")]
    imports: bool,

    /// Wrap comments at column width (default: 100, 0 to disable)
    #[arg(short = 'w', long = "wrap", default_value = "100")]
    wrap: usize,

    /// Output in JSON format
    #[arg(long = "json", conflicts_with = "markdown")]
    json: bool,

    /// Output in Markdown format (suitable for documentation and LLM context)
    #[arg(long = "markdown", short = 'm', conflicts_with = "json")]
    markdown: bool,

    /// Prefix for metadata lines (e.g., "# " or "// ")
    #[arg(short = 'p', long = "prefix")]
    prefix: Option<String>,

    /// Number of parallel workers for metadata extraction
    /// (0 = auto-detect, 1 = sequential, N = use N workers)
    #[arg(short = 'j', long = "jobs", default_value = "0")]
    jobs: usize,

    /// Show codebase statistics (file counts, language breakdown, line counts)
    #[arg(long = "stats")]
    stats: bool,

    /// Skip line counting when showing stats (faster)
    #[arg(long = "no-lines", requires = "stats")]
    no_lines: bool,

    /// Show file sizes next to filenames
    #[arg(short = 's', long = "size")]
    size: bool,

    /// Only show files modified more recently than DURATION ago
    /// Duration format: 30s, 5m, 1h, 7d, 2w, 3M, 1y
    #[arg(long = "newer", value_name = "DURATION")]
    newer: Option<String>,

    /// Only show files modified longer than DURATION ago
    /// Duration format: 30s, 5m, 1h, 7d, 2w, 3M, 1y
    #[arg(long = "older", value_name = "DURATION")]
    older: Option<String>,

    /// Maximum file size for comment/type extraction (default: 1MB)
    /// Files larger than this are skipped. Use suffixes: K, M, G (e.g., 5M for 5MB)
    #[arg(long = "max-file-size", value_name = "SIZE")]
    max_file_size: Option<String>,
}

/// Parse a duration string like "1h", "7d", "2w" into a Duration.
/// Uses the humantime crate which supports:
/// - Seconds: s, sec, secs, second, seconds
/// - Minutes: m, min, mins, minute, minutes
/// - Hours: h, hr, hrs, hour, hours
/// - Days: d, day, days
/// - Weeks: w, wk, wks, week, weeks
/// - Months: M, month, months (30.44 days)
/// - Years: y, yr, yrs, year, years (365.25 days)
fn parse_duration_string(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s.trim()).map_err(|e| e.to_string())
}

/// Parse a file size string like "5M", "100K", "1G" into bytes.
/// Supports suffixes: K/KB (1024), M/MB (1024^2), G/GB (1024^3)
/// Without suffix, interprets as bytes.
fn parse_file_size(s: &str) -> Result<u64, String> {
    let s = s.trim().to_uppercase();
    let (num_str, multiplier) = if let Some(n) = s.strip_suffix("GB") {
        (n, 1024 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix('G') {
        (n, 1024 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("MB") {
        (n, 1024 * 1024)
    } else if let Some(n) = s.strip_suffix('M') {
        (n, 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("KB") {
        (n, 1024)
    } else if let Some(n) = s.strip_suffix('K') {
        (n, 1024)
    } else {
        (s.as_str(), 1)
    };

    let num: u64 = num_str
        .trim()
        .parse()
        .map_err(|_| format!("invalid number: {}", num_str))?;

    Ok(num * multiplier)
}

/// Determine metadata order based on which flag appeared first in argv
fn get_metadata_order(matches: &ArgMatches) -> MetadataOrder {
    let comments_index = matches.index_of("comments");
    let types_index = matches.index_of("types");

    match (comments_index, types_index) {
        (Some(c), Some(t)) if c < t => MetadataOrder::CommentsFirst,
        (Some(c), Some(t)) if t < c => MetadataOrder::TypesFirst,
        _ => MetadataOrder::CommentsFirst, // default
    }
}

fn main() {
    let matches = Args::command().get_matches();
    let args = Args::from_arg_matches(&matches).unwrap_or_else(|e| {
        eprintln!("fruit: argument parsing error: {}", e);
        process::exit(1);
    });

    // Configure max file size for extraction if specified
    if let Some(ref size_str) = args.max_file_size {
        match parse_file_size(size_str) {
            Ok(size) => {
                fruit::file_utils::set_max_file_size(size);
            }
            Err(e) => {
                eprintln!("fruit: invalid --max-file-size '{}': {}", size_str, e);
                process::exit(1);
            }
        }
    }

    // Determine what metadata to show:
    // - --no-comments: disable comments (for backwards compatibility)
    // - If neither -c nor -t nor --todos: show comments (default behavior)
    // - If only -t: show types only (full mode implied)
    // - If only -c: show comments
    // - If only --todos: show todos only
    // - If both -c and -t: show both
    // - Any combination with --todos: include todos
    let (show_comments, show_types) = if args.no_comments {
        (false, args.types)
    } else {
        match (args.comments, args.types, args.todos) {
            (false, false, false) => (true, false), // default: comments only
            (false, true, _) => (false, true),      // -t specified: types
            (true, false, _) => (true, false),      // -c specified: comments
            (true, true, _) => (true, true),        // both: show both
            (false, false, true) => (false, false), // --todos alone: no comments/types
        }
    };
    let show_todos = args.todos;

    // When -t or --todos or --imports is specified, default to full mode
    let full_mode = args.full_comment || args.types || args.todos || args.imports;

    // Parse time filters
    let newer_than = args.newer.as_ref().map(|s| {
        let duration = parse_duration_string(s).unwrap_or_else(|e| {
            eprintln!("fruit: invalid --newer duration '{}': {}", s, e);
            process::exit(1);
        });
        SystemTime::now() - duration
    });

    let older_than = args.older.as_ref().map(|s| {
        let duration = parse_duration_string(s).unwrap_or_else(|e| {
            eprintln!("fruit: invalid --older duration '{}': {}", s, e);
            process::exit(1);
        });
        SystemTime::now() - duration
    });

    let walker_config = WalkerConfig {
        show_all: args.all,
        max_depth: args.level,
        dirs_only: args.dirs_only,
        extract_comments: show_comments,
        extract_types: show_types,
        extract_todos: show_todos,
        todos_only: args.todos_only,
        extract_imports: args.imports,
        show_size: args.size,
        ignore_patterns: args.ignore.clone(),
        parallel_workers: args.jobs,
        newer_than,
        older_than,
        ..Default::default()
    };

    let root = if args.path.is_absolute() {
        args.path.clone()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&args.path)
    };

    // Handle different output modes
    let result = if args.stats {
        // Stats mode: collect and display codebase statistics
        let stats_config = StatsConfig {
            count_lines: !args.no_lines,
        };
        let stats = collect_stats(&root, &args, stats_config);

        if args.json {
            print_stats_json(&stats)
        } else {
            print_stats(&stats, should_use_color(args.color))
        }
    } else if args.json {
        // JSON output requires full tree in memory (for serialization)
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
        // Use streaming walker for console/markdown output - much lower memory usage
        let mut walker = StreamingWalker::new(walker_config);

        // Set up gitignore filter unless --all is specified
        if !args.all {
            if let Some(filter) = GitignoreFilter::new(&args.path) {
                walker = walker.with_gitignore_filter(filter);
            } else {
                eprintln!("fruit: warning: not a git repository, showing all files");
            }
        }

        let metadata_config = MetadataConfig {
            comments: show_comments,
            types: show_types,
            todos: show_todos,
            full: full_mode,
            prefix: args.prefix.clone(),
            order: get_metadata_order(&matches),
        };

        let output_config = OutputConfig {
            use_color: if args.markdown {
                false
            } else {
                should_use_color(args.color)
            },
            metadata: metadata_config,
            wrap_width: if args.wrap == 0 {
                None
            } else {
                Some(args.wrap)
            },
        };

        if args.markdown {
            let mut formatter = MarkdownFormatter::new(output_config);
            match walker.walk_streaming(&root, &mut formatter) {
                Ok(Some(_)) => print_markdown(&formatter),
                Ok(None) => {
                    eprintln!(
                        "fruit: cannot access '{}': No such file or directory",
                        args.path.display()
                    );
                    process::exit(1);
                }
                Err(e) => Err(e),
            }
        } else {
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
        }
    };

    if let Err(e) = result {
        eprintln!("fruit: error writing output: {}", e);
        process::exit(1);
    }
}

/// Collect codebase statistics by walking the directory tree.
fn collect_stats(root: &Path, args: &Args, stats_config: StatsConfig) -> CodebaseStats {
    use ignore::WalkBuilder;

    let mut collector = StatsCollector::new(stats_config);

    let walker = if args.all {
        WalkBuilder::new(root)
            .hidden(false)
            .ignore(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .build()
    } else {
        WalkBuilder::new(root)
            .hidden(true)
            .ignore(true)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build()
    };

    for entry in walker.flatten() {
        let path = entry.path();

        // Skip the root directory itself
        if path == root {
            continue;
        }

        if path.is_dir() {
            collector.record_directory();
        } else if path.is_file() {
            collector.record_file(path);
        }
    }

    collector.finalize()
}
