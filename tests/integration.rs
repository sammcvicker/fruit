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
    assert_eq!(main_rs["comments"], "CLI entry point");
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

    // File with comment should have comments field
    let main_rs = children.iter().find(|c| c["name"] == "main.rs").unwrap();
    assert!(main_rs["comments"].is_string());

    // File without comment should not have comments field (skip_serializing_if)
    let empty = children.iter().find(|c| c["name"] == "empty.rs").unwrap();
    assert!(
        empty.get("comments").is_none(),
        "comments should be omitted when None"
    );
}

// ============================================================================
// --todos-only Flag Tests
// ============================================================================

#[test]
fn test_todos_only_filters_files() {
    let repo = TestRepo::with_git();
    repo.add_file("has_todo.rs", "// TODO: fix this\nfn foo() {}");
    repo.add_file("no_todo.rs", "// Regular comment\nfn bar() {}");
    repo.add_file("has_fixme.py", "# FIXME: broken\ndef baz(): pass");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--todos", "--todos-only"]);
    assert!(success);

    // Should show files with TODOs
    assert!(
        stdout.contains("has_todo.rs"),
        "should show file with TODO: {}",
        stdout
    );
    assert!(
        stdout.contains("has_fixme.py"),
        "should show file with FIXME: {}",
        stdout
    );

    // Should NOT show files without TODOs
    assert!(
        !stdout.contains("no_todo.rs"),
        "should not show file without TODOs: {}",
        stdout
    );
}

#[test]
fn test_todos_only_empty_when_no_todos() {
    let repo = TestRepo::with_git();
    repo.add_file("clean.rs", "// Just a comment\nfn foo() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--todos", "--todos-only"]);
    assert!(success);
    assert!(
        stdout.contains("0 files"),
        "should find no files: {}",
        stdout
    );
}

#[test]
fn test_todos_only_with_json_output() {
    let repo = TestRepo::with_git();
    repo.add_file("has_todo.rs", "// TODO: fix this\nfn foo() {}");
    repo.add_file("no_todo.rs", "// Regular comment\nfn bar() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--todos", "--todos-only", "--json"]);
    assert!(success);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let children = json["children"].as_array().unwrap();

    // Should only include file with TODO
    assert_eq!(
        children.len(),
        1,
        "should have exactly one file: {:?}",
        children
    );
    assert_eq!(children[0]["name"], "has_todo.rs");
    assert!(children[0]["todos"].is_array());
}

#[test]
fn test_todos_only_in_nested_directories() {
    let repo = TestRepo::with_git();
    repo.add_file("top.rs", "fn top() {}");
    repo.add_file("src/with_todo.rs", "// TODO: implement\nfn todo() {}");
    repo.add_file("src/no_todo.rs", "fn clean() {}");
    repo.add_file("tests/has_fixme.rs", "// FIXME: broken test\nfn test() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--todos", "--todos-only"]);
    assert!(success);

    // Should show files with TODOs
    assert!(
        stdout.contains("with_todo.rs"),
        "should show with_todo.rs: {}",
        stdout
    );
    assert!(
        stdout.contains("has_fixme.rs"),
        "should show has_fixme.rs: {}",
        stdout
    );

    // Should NOT show files without TODOs
    assert!(
        !stdout.contains("top.rs"),
        "should not show top.rs: {}",
        stdout
    );
    assert!(
        !stdout.contains("no_todo.rs"),
        "should not show no_todo.rs: {}",
        stdout
    );

    // Should show 2 files (only the ones with TODOs)
    assert!(
        stdout.contains("2 files"),
        "should show 2 files: {}",
        stdout
    );
}

#[test]
fn test_todos_only_without_todos_flag() {
    // Verify that --todos-only works without requiring --todos flag
    let repo = TestRepo::with_git();
    repo.add_file("has_todo.rs", "// TODO: implement\nfn foo() {}");
    repo.add_file("no_todo.rs", "// Regular comment\nfn bar() {}");

    // Should work with just --todos-only (no --todos required)
    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--todos-only"]);
    assert!(success, "fruit --todos-only should work without --todos");

    // Should show file with TODO
    assert!(
        stdout.contains("has_todo.rs"),
        "should show file with TODO: {}",
        stdout
    );

    // Should NOT show file without TODO
    assert!(
        !stdout.contains("no_todo.rs"),
        "should not show file without TODO: {}",
        stdout
    );
}

// ============================================================================
// Markdown Output Tests
// ============================================================================

#[test]
fn test_markdown_output() {
    let repo = TestRepo::with_git();
    repo.add_file("main.rs", "//! CLI entry point\nfn main() {}");
    repo.add_file("src/lib.rs", "//! Library module\npub mod foo;");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--markdown"]);
    assert!(success, "fruit --markdown should succeed");

    // Verify markdown structure - directories should be bold with trailing slash
    assert!(
        stdout.contains("**"),
        "should have bold formatting for directories: {}",
        stdout
    );

    // Files should be in backticks
    assert!(
        stdout.contains("`main.rs`"),
        "should have filename in backticks: {}",
        stdout
    );
    assert!(
        stdout.contains("`lib.rs`"),
        "should have filename in backticks: {}",
        stdout
    );

    // Comments should be extracted and shown
    assert!(
        stdout.contains("CLI entry point"),
        "should include comment for main.rs: {}",
        stdout
    );
    assert!(
        stdout.contains("Library module"),
        "should include comment for lib.rs: {}",
        stdout
    );

    // Should have summary line in italics
    assert!(
        stdout.contains("*1 directories, 2 files*"),
        "should have italicized summary: {}",
        stdout
    );
}

#[test]
fn test_markdown_with_types() {
    let repo = TestRepo::with_git();
    repo.add_file("lib.rs", "pub fn hello() {}\npub struct Config {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--markdown", "-t"]);
    assert!(success);

    // Should show type signatures
    assert!(
        stdout.contains("pub fn hello"),
        "should show function signature: {}",
        stdout
    );
    assert!(
        stdout.contains("pub struct Config"),
        "should show struct signature: {}",
        stdout
    );
}

#[test]
fn test_markdown_no_comments() {
    let repo = TestRepo::with_git();
    repo.add_file("main.rs", "//! Has comment\nfn main() {}");
    repo.add_file("empty.rs", "fn no_comment() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--markdown", "--no-comments"]);
    assert!(success);

    // Files should still be shown
    assert!(
        stdout.contains("`main.rs`"),
        "should show main.rs: {}",
        stdout
    );
    assert!(
        stdout.contains("`empty.rs`"),
        "should show empty.rs: {}",
        stdout
    );

    // Comments should NOT be extracted
    assert!(
        !stdout.contains("Has comment"),
        "should not extract comments with --no-comments: {}",
        stdout
    );
}

#[test]
fn test_markdown_nested_structure() {
    let repo = TestRepo::with_git();
    repo.add_file("src/main.rs", "//! Entry point\nfn main() {}");
    repo.add_file("src/lib.rs", "//! Library\npub mod foo;");
    repo.add_file("src/foo/mod.rs", "//! Foo module\npub fn foo() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--markdown"]);
    assert!(success);

    // Should show directory structure
    assert!(
        stdout.contains("**src/**"),
        "should show src directory: {}",
        stdout
    );
    assert!(
        stdout.contains("**foo/**"),
        "should show foo directory: {}",
        stdout
    );

    // Files should be present
    assert!(
        stdout.contains("`main.rs`"),
        "should show main.rs: {}",
        stdout
    );
    assert!(
        stdout.contains("`mod.rs`"),
        "should show mod.rs: {}",
        stdout
    );

    // Comments should be extracted
    assert!(
        stdout.contains("Entry point"),
        "should show main.rs comment: {}",
        stdout
    );
    assert!(
        stdout.contains("Foo module"),
        "should show mod.rs comment: {}",
        stdout
    );
}

#[test]
fn test_markdown_with_todos() {
    let repo = TestRepo::with_git();
    repo.add_file("has_todo.rs", "// TODO: fix this\nfn foo() {}");
    repo.add_file("has_fixme.py", "# FIXME: broken\ndef baz(): pass");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--markdown", "--todos"]);
    assert!(success);

    // Should show TODO markers
    assert!(
        stdout.contains("TODO: fix this"),
        "should show TODO marker: {}",
        stdout
    );
    assert!(
        stdout.contains("FIXME: broken"),
        "should show FIXME marker: {}",
        stdout
    );
}

#[test]
fn test_markdown_todos_only() {
    let repo = TestRepo::with_git();
    repo.add_file("has_todo.rs", "// TODO: fix this\nfn foo() {}");
    repo.add_file("no_todo.rs", "// Regular comment\nfn bar() {}");

    let (stdout, _stderr, success) =
        run_fruit(repo.path(), &["--markdown", "--todos", "--todos-only"]);
    assert!(success);

    // Should show file with TODO
    assert!(
        stdout.contains("`has_todo.rs`"),
        "should show file with TODO: {}",
        stdout
    );
    assert!(
        stdout.contains("TODO: fix this"),
        "should show TODO marker: {}",
        stdout
    );

    // Should NOT show file without TODO
    assert!(
        !stdout.contains("`no_todo.rs`"),
        "should not show file without TODO: {}",
        stdout
    );

    // Should show correct file count
    assert!(
        stdout.contains("*0 directories, 1 files*"),
        "should show 1 file: {}",
        stdout
    );
}

#[test]
fn test_uppercase_extension_comment_extraction() {
    let repo = TestRepo::with_git();
    repo.add_file("MAIN.RS", "//! Uppercase extension\nfn main() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);
    assert!(
        stdout.contains("Uppercase extension"),
        "should extract comment from uppercase extension file: {}",
        stdout
    );
}

#[test]
fn test_mixed_case_extensions() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "Script.Py",
        r#""""Mixed case Python file."""
def main():
    pass
"#,
    );
    repo.add_file("Main.Rs", "//! Mixed case Rust file\nfn main() {}");
    repo.add_file(
        "app.JS",
        "/** Mixed case JavaScript file */\nfunction main() {}",
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &[]);
    assert!(success);

    // All files should be recognized and comments extracted
    assert!(
        stdout.contains("Mixed case Python file"),
        "should extract comment from .Py file: {}",
        stdout
    );
    assert!(
        stdout.contains("Mixed case Rust file"),
        "should extract comment from .Rs file: {}",
        stdout
    );
    assert!(
        stdout.contains("Mixed case JavaScript file"),
        "should extract comment from .JS file: {}",
        stdout
    );
}

// ============================================================================
// Imports Extraction Tests
// ============================================================================

#[test]
fn test_imports_rust_output() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "main.rs",
        r#"//! Main entry point
use std::path::Path;
use clap::Parser;
use serde::Deserialize;
use crate::config;
fn main() {}"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--imports"]);
    assert!(success);

    // Should show the file
    assert!(
        stdout.contains("main.rs"),
        "should show main.rs: {}",
        stdout
    );

    // Should show imports summary
    assert!(
        stdout.contains("imports:"),
        "should show imports label: {}",
        stdout
    );

    // Should show external imports
    assert!(
        stdout.contains("clap"),
        "should show clap import: {}",
        stdout
    );
    assert!(
        stdout.contains("serde"),
        "should show serde import: {}",
        stdout
    );

    // Should show std imports
    assert!(
        stdout.contains("std::"),
        "should show std imports: {}",
        stdout
    );

    // Should show internal imports
    assert!(
        stdout.contains("crate::"),
        "should show crate imports: {}",
        stdout
    );
}

#[test]
fn test_imports_python_output() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "script.py",
        r#""""Python script module."""
import os
import sys
import numpy as np
from pathlib import Path
from . import utils
def main(): pass"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--imports"]);
    assert!(success);

    // Should show imports summary
    assert!(
        stdout.contains("imports:"),
        "should show imports label: {}",
        stdout
    );

    // Should show standard library imports
    assert!(
        stdout.contains("os") || stdout.contains("sys") || stdout.contains("pathlib"),
        "should show stdlib imports: {}",
        stdout
    );

    // Should show external imports
    assert!(
        stdout.contains("numpy"),
        "should show numpy import: {}",
        stdout
    );
}

#[test]
fn test_imports_javascript_output() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "app.js",
        r#"/** Main application module */
import React from 'react';
import path from 'path';
import { Component } from './components';
const fs = require('fs');
export function main() {}"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--imports"]);
    assert!(success);

    // Should show imports summary
    assert!(
        stdout.contains("imports:"),
        "should show imports label: {}",
        stdout
    );

    // Should show external imports
    assert!(
        stdout.contains("react"),
        "should show react import: {}",
        stdout
    );

    // Should show Node.js built-in
    assert!(
        stdout.contains("path") || stdout.contains("fs"),
        "should show Node.js built-ins: {}",
        stdout
    );
}

#[test]
fn test_imports_go_output() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "main.go",
        r#"// Package main is the entry point
package main

import "fmt"
import (
    "os"
    "github.com/spf13/cobra"
)

func main() {}"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--imports"]);
    assert!(success);

    // Should show imports summary
    assert!(
        stdout.contains("imports:"),
        "should show imports label: {}",
        stdout
    );

    // Should show standard library imports
    assert!(
        stdout.contains("fmt") || stdout.contains("os"),
        "should show Go stdlib imports: {}",
        stdout
    );

    // Should show external imports
    assert!(
        stdout.contains("github.com"),
        "should show external Go package: {}",
        stdout
    );
}

#[test]
fn test_imports_json_output() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "main.rs",
        r#"//! Main module
use std::path::Path;
use clap::Parser;
fn main() {}"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--imports", "--json"]);
    assert!(success);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let children = json["children"].as_array().unwrap();
    let main_rs = children.iter().find(|c| c["name"] == "main.rs").unwrap();

    // Should have imports field
    assert!(
        main_rs.get("imports").is_some(),
        "should have imports field in JSON: {:?}",
        main_rs
    );

    let imports = &main_rs["imports"];

    // Should have categorized imports
    assert!(
        imports.get("external").is_some(),
        "should have external imports: {:?}",
        imports
    );
    assert!(
        imports.get("std").is_some(),
        "should have std imports: {:?}",
        imports
    );

    // Verify specific imports
    let external = imports["external"].as_array().unwrap();
    assert!(
        external.iter().any(|i| i == "clap"),
        "should contain clap in external: {:?}",
        external
    );

    let std_imports = imports["std"].as_array().unwrap();
    assert!(
        std_imports
            .iter()
            .any(|i| i.as_str().unwrap().contains("path")),
        "should contain path in std: {:?}",
        std_imports
    );
}

#[test]
fn test_imports_no_imports_flag() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "main.rs",
        r#"//! Main module
use std::path::Path;
use clap::Parser;
fn main() {}"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--no-imports"]);
    assert!(success);

    // Should show the file
    assert!(
        stdout.contains("main.rs"),
        "should show main.rs: {}",
        stdout
    );

    // Should NOT show imports
    assert!(
        !stdout.contains("imports:"),
        "should not show imports with --no-imports: {}",
        stdout
    );
}

#[test]
fn test_imports_file_without_imports() {
    let repo = TestRepo::with_git();
    repo.add_file("empty.rs", "fn main() {}");

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--imports", "--json"]);
    assert!(success);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let children = json["children"].as_array().unwrap();
    let empty_rs = children.iter().find(|c| c["name"] == "empty.rs").unwrap();

    // File without imports should not have imports field (skip_serializing_if)
    assert!(
        empty_rs.get("imports").is_none(),
        "imports field should be omitted when empty: {:?}",
        empty_rs
    );
}

#[test]
fn test_imports_multiple_files() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "server.rs",
        r#"//! Server module
use std::net::TcpListener;
use actix_web::HttpServer;
fn main() {}"#,
    );
    repo.add_file(
        "client.py",
        r#""""Client module."""
import requests
import json
def main(): pass"#,
    );
    repo.add_file(
        "config.js",
        r#"/** Config module */
const path = require('path');
import dotenv from 'dotenv';
export const config = {};"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--imports"]);
    assert!(success);

    // All files should be shown with imports
    assert!(stdout.contains("server.rs"), "should show server.rs");
    assert!(stdout.contains("client.py"), "should show client.py");
    assert!(stdout.contains("config.js"), "should show config.js");

    // Should have imports for Rust file
    assert!(
        stdout.contains("actix_web"),
        "should show actix_web import: {}",
        stdout
    );

    // Should have imports for Python file
    assert!(
        stdout.contains("requests"),
        "should show requests import: {}",
        stdout
    );

    // Should have imports for JavaScript file
    assert!(
        stdout.contains("dotenv"),
        "should show dotenv import: {}",
        stdout
    );
}

#[test]
fn test_imports_with_nested_directories() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "src/main.rs",
        r#"//! Main entry point
use clap::Parser;
fn main() {}"#,
    );
    repo.add_file(
        "src/lib.rs",
        r#"//! Library module
use serde::Serialize;
pub fn run() {}"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--imports"]);
    assert!(success);

    // Should show nested structure
    assert!(stdout.contains("src"), "should show src directory");
    assert!(stdout.contains("main.rs"), "should show main.rs");
    assert!(stdout.contains("lib.rs"), "should show lib.rs");

    // Should show imports for both files
    assert!(
        stdout.contains("clap"),
        "should show clap import: {}",
        stdout
    );
    assert!(
        stdout.contains("serde"),
        "should show serde import: {}",
        stdout
    );
}

#[test]
fn test_imports_typescript_output() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "app.ts",
        r#"/** Main application module */
import React from 'react';
import type { Config } from '@types/node';
import { Component } from './components';
import * as fs from 'fs';
export function main() {}"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--imports"]);
    assert!(success);

    // Should show imports summary
    assert!(
        stdout.contains("imports:"),
        "should show imports label: {}",
        stdout
    );

    // Should show external imports
    assert!(
        stdout.contains("react"),
        "should show react import: {}",
        stdout
    );
    assert!(
        stdout.contains("@types/node"),
        "should show @types/node import: {}",
        stdout
    );

    // Should show internal imports
    assert!(
        stdout.contains("./components"),
        "should show relative import: {}",
        stdout
    );
}

#[test]
fn test_markdown_multiline_comments_full_mode() {
    let repo = TestRepo::with_git();
    repo.add_file(
        "lib.rs",
        r#"//! First line of comment
//! Second line of comment
//! Third line of comment
pub fn main() {}"#,
    );

    let (stdout, _stderr, success) = run_fruit(repo.path(), &["--markdown", "--full-comment"]);
    assert!(success);

    // Should show all comment lines
    assert!(
        stdout.contains("First line of comment"),
        "should show first line: {}",
        stdout
    );

    // In full mode, additional lines should be in blockquote format
    assert!(
        stdout.contains("> "),
        "should use blockquote for additional lines: {}",
        stdout
    );
    assert!(
        stdout.contains("Second line of comment"),
        "should show second line: {}",
        stdout
    );
    assert!(
        stdout.contains("Third line of comment"),
        "should show third line: {}",
        stdout
    );
}
