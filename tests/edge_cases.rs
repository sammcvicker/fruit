//! Edge case and error handling tests for fruit

mod harness;

use harness::{TestRepo, run_fruit};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

// ============================================================================
// Symlink Edge Cases
// ============================================================================

#[test]
fn test_symlink_to_file() {
    let repo = TestRepo::with_git();
    repo.add_file("target.rs", "//! Target file\nfn target() {}");

    let link_path = repo.path().join("link.rs");
    symlink(repo.path().join("target.rs"), &link_path).expect("Failed to create symlink");

    // Add symlink to git
    std::process::Command::new("git")
        .args(["add", "link.rs"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to git add");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should succeed with symlink");
    // Symlinks should be skipped to prevent issues
    assert!(stdout.contains("target.rs"), "should show target file");
    // The symlink file is intentionally skipped
}

#[test]
fn test_symlink_to_directory() {
    let repo = TestRepo::with_git();
    repo.add_file("realdir/file.rs", "fn file() {}");

    let link_path = repo.path().join("linkdir");
    symlink(repo.path().join("realdir"), &link_path).expect("Failed to create dir symlink");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["-a"]);
    assert!(success, "fruit should succeed with directory symlink");
    assert!(stdout.contains("realdir"), "should show real directory");
    // Directory symlink should be skipped to prevent infinite loops
}

#[test]
fn test_symlink_to_parent_no_infinite_loop() {
    let repo = TestRepo::with_git();
    repo.add_file("subdir/file.rs", "fn file() {}");

    // Create symlink from subdir/parent -> .. (creates potential infinite loop)
    let link_path = repo.path().join("subdir").join("parent");
    symlink("..", &link_path).expect("Failed to create parent symlink");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["-a"]);
    assert!(success, "fruit should not hang on parent symlink");
    assert!(stdout.contains("subdir"), "should show subdir");
    assert!(stdout.contains("file.rs"), "should show file in subdir");
    // Should complete without infinite loop - symlinks are skipped
}

#[test]
fn test_broken_symlink() {
    let repo = TestRepo::with_git();
    repo.add_file("real.rs", "fn real() {}");

    // Create symlink to non-existent target
    let link_path = repo.path().join("broken_link.rs");
    symlink("nonexistent.rs", &link_path).expect("Failed to create broken symlink");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["-a"]);
    assert!(success, "fruit should handle broken symlinks");
    assert!(stdout.contains("real.rs"), "should show real file");
}

#[test]
fn test_self_referential_symlink() {
    let repo = TestRepo::with_git();
    repo.add_file("file.rs", "fn file() {}");

    // Create a symlink that points to itself
    let link_path = repo.path().join("selfref");
    symlink("selfref", &link_path).expect("Failed to create self-referential symlink");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["-a"]);
    assert!(success, "fruit should handle self-referential symlinks");
    assert!(stdout.contains("file.rs"), "should show regular file");
}

// ============================================================================
// Permission Error Handling
// ============================================================================

#[test]
#[cfg(unix)]
fn test_unreadable_directory() {
    let repo = TestRepo::with_git();
    repo.add_file("readable/file.rs", "fn readable() {}");

    // Create an unreadable directory
    let unreadable = repo.path().join("unreadable");
    fs::create_dir(&unreadable).expect("Failed to create dir");
    fs::write(unreadable.join("hidden.rs"), "fn hidden() {}").expect("Failed to write file");

    // Make directory unreadable (no read permission)
    let mut perms = fs::metadata(&unreadable).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&unreadable, perms).expect("Failed to set permissions");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["-a"]);

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&unreadable).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&unreadable, perms).expect("Failed to restore permissions");

    assert!(
        success,
        "fruit should handle unreadable directories gracefully"
    );
    assert!(
        stdout.contains("readable"),
        "should show readable directory"
    );
    assert!(stdout.contains("file.rs"), "should show readable file");
}

#[test]
#[cfg(unix)]
fn test_unreadable_file_comment_extraction() {
    let repo = TestRepo::with_git();
    let file_path = repo.add_file("unreadable.rs", "//! Secret comment\nfn secret() {}");

    // Make file unreadable
    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&file_path, perms).expect("Failed to set permissions");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&file_path, perms).expect("Failed to restore permissions");

    assert!(success, "fruit should handle unreadable files");
    // File should appear but without comment (can't read content)
    assert!(stdout.contains("unreadable.rs"), "should list the file");
    assert!(
        !stdout.contains("Secret comment"),
        "should not show comment from unreadable file"
    );
}

// ============================================================================
// Special Filenames
// ============================================================================

#[test]
fn test_filename_with_spaces() {
    let repo = TestRepo::with_git();
    repo.add_file("file with spaces.rs", "//! Spaced file\nfn spaced() {}");
    repo.add_file("dir with spaces/nested.rs", "fn nested() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should handle spaces in filenames");
    assert!(
        stdout.contains("file with spaces.rs"),
        "should show file with spaces: {}",
        stdout
    );
    assert!(
        stdout.contains("dir with spaces"),
        "should show dir with spaces"
    );
}

#[test]
fn test_filename_with_unicode() {
    let repo = TestRepo::with_git();
    repo.add_file("日本語.rs", "//! Japanese filename\nfn japanese() {}");
    repo.add_file("émoji_🎉.rs", "//! Emoji in name\nfn emoji() {}");
    repo.add_file("中文目录/文件.rs", "fn chinese() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should handle unicode filenames");
    assert!(
        stdout.contains("日本語.rs"),
        "should show Japanese filename"
    );
    assert!(stdout.contains("émoji_🎉.rs"), "should show emoji filename");
    assert!(stdout.contains("中文目录"), "should show Chinese directory");
}

#[test]
fn test_filename_with_special_chars() {
    let repo = TestRepo::with_git();
    // Note: Some characters like / and null are not valid in filenames
    repo.add_file("file-with-dashes.rs", "fn dashes() {}");
    repo.add_file("file_with_underscores.rs", "fn underscores() {}");
    repo.add_file("file.multiple.dots.rs", "fn dots() {}");
    repo.add_file("UPPERCASE.RS", "fn upper() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should handle special characters");
    assert!(stdout.contains("file-with-dashes.rs"));
    assert!(stdout.contains("file_with_underscores.rs"));
    assert!(stdout.contains("file.multiple.dots.rs"));
    assert!(stdout.contains("UPPERCASE.RS"));
}

// ============================================================================
// Comment Extraction Edge Cases
// ============================================================================

#[test]
fn test_empty_file() {
    let repo = TestRepo::with_git();
    repo.add_file("empty.rs", "");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should handle empty files");
    assert!(stdout.contains("empty.rs"), "should show empty file");
    // No comment should be extracted from empty file
}

#[test]
fn test_whitespace_only_file() {
    let repo = TestRepo::with_git();
    repo.add_file("whitespace.rs", "   \n\n\t\t\n   ");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should handle whitespace-only files");
    assert!(stdout.contains("whitespace.rs"), "should show file");
}

#[test]
fn test_file_with_only_code_no_comment() {
    let repo = TestRepo::with_git();
    repo.add_file("no_comment.rs", "fn main() {\n    println!(\"hello\");\n}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    assert!(stdout.contains("no_comment.rs"));
    // Should not crash or show garbage
}

#[test]
fn test_very_long_first_line() {
    let repo = TestRepo::with_git();
    let long_comment = format!("//! {}\nfn main() {{}}", "x".repeat(10000));
    repo.add_file("long.rs", &long_comment);

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should handle very long comments");
    assert!(stdout.contains("long.rs"), "should show file");
}

#[test]
fn test_binary_file_with_code_extension() {
    let repo = TestRepo::with_git();
    // Create a file that looks like source but contains binary data
    let binary_content: Vec<u8> = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0x89, 0x50, 0x4E, 0x47];
    let file_path = repo.path().join("binary.rs");
    fs::write(&file_path, &binary_content).expect("Failed to write binary file");

    std::process::Command::new("git")
        .args(["add", "binary.rs"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to git add");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should handle binary files gracefully");
    assert!(stdout.contains("binary.rs"), "should list binary file");
    // Should not crash on binary content
}

#[test]
fn test_file_no_extension() {
    let repo = TestRepo::with_git();
    repo.add_file("Makefile", "# Build script\nall:\n\techo hello");
    repo.add_file("README", "This is a readme");
    repo.add_file("LICENSE", "MIT License");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    assert!(stdout.contains("Makefile"));
    assert!(stdout.contains("README"));
    assert!(stdout.contains("LICENSE"));
    // Files without extension should not crash comment extraction
}

#[test]
fn test_file_unknown_extension() {
    let repo = TestRepo::with_git();
    repo.add_file("data.xyz", "Some random data");
    repo.add_file("config.toml", "# TOML config\n[section]\nkey = \"value\"");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    assert!(stdout.contains("data.xyz"));
    assert!(stdout.contains("config.toml"));
}

// ============================================================================
// Git Edge Cases
// ============================================================================

#[test]
fn test_non_git_directory() {
    let repo = TestRepo::new(); // No git init
    repo.add_untracked("file.rs", "fn file() {}");

    let (_stdout, stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should work outside git repos");
    assert!(
        stderr.contains("not a git repository"),
        "should warn about no git: {}",
        stderr
    );
    // With no git, files should still show when using -a or the warning is shown
}

#[test]
fn test_empty_git_repo() {
    let repo = TestRepo::with_git();
    // No files added

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should handle empty repos");
    assert!(stdout.contains("0 directories, 0 files"));
}

#[test]
fn test_git_repo_with_only_gitignore() {
    let repo = TestRepo::with_git();
    repo.add_file(".gitignore", "*.log\ntarget/");

    let (_stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    // .gitignore should be tracked and visible
}

// ============================================================================
// Output Edge Cases
// ============================================================================

#[test]
fn test_very_deep_nesting() {
    let repo = TestRepo::with_git();
    // Create deeply nested structure
    repo.add_file("a/b/c/d/e/f/g/h/deep.rs", "fn deep() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should handle deep nesting");
    assert!(stdout.contains("deep.rs"), "should show deeply nested file");
}

#[test]
fn test_many_files_in_directory() {
    let repo = TestRepo::with_git();
    // Create many files
    for i in 0..100 {
        repo.add_file(
            &format!("file_{:03}.rs", i),
            &format!("fn file_{}() {{}}", i),
        );
    }

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success, "fruit should handle many files");
    assert!(
        stdout.contains("100 files"),
        "should count all files: {}",
        stdout
    );
}

#[test]
fn test_sorting_order() {
    let repo = TestRepo::with_git();
    repo.add_file("zebra.rs", "fn z() {}");
    repo.add_file("apple.rs", "fn a() {}");
    repo.add_file("middle.rs", "fn m() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);

    // Verify alphabetical order
    let apple_pos = stdout.find("apple.rs").expect("should have apple");
    let middle_pos = stdout.find("middle.rs").expect("should have middle");
    let zebra_pos = stdout.find("zebra.rs").expect("should have zebra");

    assert!(apple_pos < middle_pos, "apple should come before middle");
    assert!(middle_pos < zebra_pos, "middle should come before zebra");
}

#[test]
fn test_wrap_width_zero() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "long_comment.rs",
        "//! This is a very long comment that would normally be wrapped but we disabled wrapping\nfn main() {}",
    );

    let (_stdout, _stderr, success) = run_fruit(repo.path(), &["-w", "0"]);
    assert!(success, "fruit should handle wrap width 0");
    // Comment should not be wrapped
}

#[test]
fn test_very_narrow_wrap_width() {
    let repo = TestRepo::with_git();
    repo.add_file("comment.rs", "//! Short comment\nfn main() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["-w", "5"]);
    assert!(success, "fruit should handle narrow wrap width");
    assert!(stdout.contains("comment.rs"));
}
