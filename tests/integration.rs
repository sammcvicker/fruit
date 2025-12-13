//! Integration tests for fruit

mod harness;

use harness::{run_fruit, TestRepo};

#[test]
fn test_basic_tree_output() {
    let repo = TestRepo::with_git();
    repo.add_file("main.rs", "//! Main module\nfn main() {}");
    repo.add_file("lib.rs", "//! Library module\npub mod foo;");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should succeed");
    assert!(stdout.contains("main.rs"), "should show main.rs");
    assert!(stdout.contains("lib.rs"), "should show lib.rs");
}

#[test]
fn test_comment_extraction() {
    let repo = TestRepo::with_git();
    repo.add_file("main.rs", "//! CLI entry point\nfn main() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    assert!(
        stdout.contains("# CLI entry point"),
        "should extract comment: {}",
        stdout
    );
}

#[test]
fn test_git_filtering() {
    let repo = TestRepo::with_git();
    repo.add_file("tracked.rs", "fn tracked() {}");
    repo.add_untracked("untracked.rs", "fn untracked() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    assert!(stdout.contains("tracked.rs"), "should show tracked file");
    assert!(
        !stdout.contains("untracked.rs"),
        "should not show untracked file: {}",
        stdout
    );
}

#[test]
fn test_show_all_flag() {
    let repo = TestRepo::with_git();
    repo.add_file("tracked.rs", "fn tracked() {}");
    repo.add_untracked("untracked.rs", "fn untracked() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["-a"]);
    assert!(success);
    assert!(stdout.contains("tracked.rs"), "should show tracked file");
    assert!(
        stdout.contains("untracked.rs"),
        "should show untracked file with -a: {}",
        stdout
    );
}

#[test]
fn test_depth_limit() {
    let repo = TestRepo::with_git();
    repo.add_file("top.rs", "fn top() {}");
    repo.add_file("level1/mid.rs", "fn mid() {}");
    repo.add_file("level1/level2/deep.rs", "fn deep() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["-L", "1"]);
    assert!(success);
    assert!(stdout.contains("top.rs"), "should show top level");
    assert!(stdout.contains("level1"), "should show first level dir");
    // Should not descend into level2
    assert!(
        !stdout.contains("deep.rs"),
        "should not show deep files: {}",
        stdout
    );
}

#[test]
fn test_dirs_only() {
    let repo = TestRepo::with_git();
    repo.add_file("file.rs", "fn file() {}");
    repo.add_file("subdir/nested.rs", "fn nested() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["-d"]);
    assert!(success);
    assert!(!stdout.contains("file.rs"), "should not show files: {}", stdout);
    assert!(stdout.contains("subdir"), "should show directories");
}

#[test]
fn test_no_comments_flag() {
    let repo = TestRepo::with_git();
    repo.add_file("main.rs", "//! Module comment\nfn main() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--no-comments"]);
    assert!(success);
    assert!(stdout.contains("main.rs"), "should show file");
    assert!(
        !stdout.contains("Module comment"),
        "should not extract comments: {}",
        stdout
    );
}

#[test]
fn test_nested_directories() {
    let repo = TestRepo::with_git();
    repo.add_file("src/main.rs", "//! Entry point\nfn main() {}");
    repo.add_file("src/lib.rs", "//! Library\npub mod foo;");
    repo.add_file("src/foo/mod.rs", "//! Foo module\npub fn foo() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    assert!(stdout.contains("src"));
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("foo"));
    assert!(stdout.contains("mod.rs"));
}

#[test]
fn test_ignore_pattern() {
    let repo = TestRepo::with_git();
    repo.add_file("keep.rs", "fn keep() {}");
    repo.add_file("ignore_me.rs", "fn ignore() {}");
    repo.add_file("also_ignore.rs", "fn also() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["-I", "*ignore*"]);
    assert!(success);
    assert!(stdout.contains("keep.rs"), "should show non-ignored files");
    assert!(
        !stdout.contains("ignore_me.rs"),
        "should ignore matching pattern: {}",
        stdout
    );
}

#[test]
fn test_python_comments() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "script.py",
        r#""""Module docstring for the script."""

def main():
    pass
"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    assert!(
        stdout.contains("# Module docstring"),
        "should extract Python docstring: {}",
        stdout
    );
}

#[test]
fn test_javascript_comments() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "app.js",
        r#"/**
 * Main application module
 */
export function main() {}
"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    assert!(
        stdout.contains("# Main application module"),
        "should extract JSDoc comment: {}",
        stdout
    );
}

#[test]
fn test_directory_file_counts() {
    let repo = TestRepo::with_git();
    repo.add_file("a.rs", "fn a() {}");
    repo.add_file("b.rs", "fn b() {}");
    repo.add_file("sub/c.rs", "fn c() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    // Should have 1 directory (sub) and 3 files
    assert!(
        stdout.contains("1 directories, 3 files"),
        "should count correctly: {}",
        stdout
    );
}
