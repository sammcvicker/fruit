//! Integration tests for fruit

mod harness;

use harness::{TestRepo, run_fruit};

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
        stdout.contains("CLI entry point"),
        "should extract comment: {}",
        stdout
    );
}

#[test]
fn test_gitignore_filtering() {
    let repo = TestRepo::with_git();
    repo.add_file("main.rs", "fn main() {}");
    repo.add_untracked("debug.log", "log content");
    // Add a .gitignore that ignores *.log files
    repo.add_file(".gitignore", "*.log\n");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    assert!(stdout.contains("main.rs"), "should show .rs file");
    assert!(
        !stdout.contains("debug.log"),
        "should not show .log file (ignored by .gitignore): {}",
        stdout
    );
}

#[test]
fn test_untracked_files_shown() {
    // With gitignore-based filtering, untracked files ARE shown (unlike old behavior)
    // unless they match a .gitignore pattern
    let repo = TestRepo::with_git();
    repo.add_file("tracked.rs", "fn tracked() {}");
    repo.add_untracked("untracked.rs", "fn untracked() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    assert!(stdout.contains("tracked.rs"), "should show tracked file");
    assert!(
        stdout.contains("untracked.rs"),
        "should show untracked file (gitignore-based filtering): {}",
        stdout
    );
}

#[test]
fn test_show_all_flag() {
    // -a should show files that would normally be hidden by .gitignore
    let repo = TestRepo::with_git();
    repo.add_file("main.rs", "fn main() {}");
    repo.add_file(".gitignore", "*.log\n");
    repo.add_untracked("debug.log", "log content");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["-a"]);
    assert!(success);
    assert!(stdout.contains("main.rs"), "should show main.rs");
    assert!(
        stdout.contains("debug.log"),
        "should show ignored file with -a: {}",
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
    assert!(
        !stdout.contains("file.rs"),
        "should not show files: {}",
        stdout
    );
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
        stdout.contains("Module docstring"),
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
        stdout.contains("Main application module"),
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

#[test]
fn test_json_output() {
    let repo = TestRepo::with_git();
    repo.add_file("main.rs", "//! CLI entry point\nfn main() {}");
    repo.add_file("src/lib.rs", "//! Library module\npub mod foo;");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--json"]);
    assert!(success, "fruit --json should succeed");

    // Parse as JSON to verify valid output
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    // Check structure
    assert_eq!(json["type"], "dir", "root should be a directory");
    assert!(json["children"].is_array(), "should have children array");

    // Verify files are included with correct structure
    let children = json["children"].as_array().unwrap();
    let main_rs = children.iter().find(|c| c["name"] == "main.rs");
    assert!(main_rs.is_some(), "should include main.rs");

    let main_rs = main_rs.unwrap();
    assert_eq!(main_rs["type"], "file");
    assert_eq!(main_rs["comment"], "CLI entry point");
}

#[test]
fn test_json_no_comments() {
    let repo = TestRepo::with_git();
    repo.add_file("main.rs", "//! Has comment\nfn main() {}");
    repo.add_file("empty.rs", "fn no_comment() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--json"]);
    assert!(success);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let children = json["children"].as_array().unwrap();

    // File with comment should have comment field
    let main_rs = children.iter().find(|c| c["name"] == "main.rs").unwrap();
    assert!(main_rs["comment"].is_string());

    // File without comment should not have comment field (skip_serializing_if)
    let empty = children.iter().find(|c| c["name"] == "empty.rs").unwrap();
    assert!(
        empty.get("comment").is_none(),
        "comment should be omitted when None"
    );
}
