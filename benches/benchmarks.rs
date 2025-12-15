//! Performance benchmarks for fruit

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fruit::{GitFilter, GitignoreFilter, extract_first_comment};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

// Sample source code for benchmarking comment extraction
const RUST_SOURCE: &str = r#"//! Module documentation
//! with multiple lines

use std::path::Path;

/// Main function documentation
fn main() {
    println!("Hello, world!");
}
"#;

const PYTHON_SOURCE: &str = r#"#!/usr/bin/env python3
"""
Module docstring explaining the purpose of this module.

This is a longer description with more details.
"""

import os

def main():
    print("Hello, world!")
"#;

const JS_SOURCE: &str = r#"/**
 * Main application module
 * Provides core functionality
 */

function main() {
    console.log("Hello, world!");
}
"#;

const GO_SOURCE: &str = r#"// Package main provides the entry point for the application.
// This is additional documentation.
package main

import "fmt"

func main() {
    fmt.Println("Hello, world!")
}
"#;

const JAVA_SOURCE: &str = r#"/**
 * Main application class
 * @author Test
 */
public class Main {
    public static void main(String[] args) {
        System.out.println("Hello, world!");
    }
}
"#;

fn create_test_repo_with_files(file_count: usize) -> TempDir {
    let dir = TempDir::new().unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create files
    for i in 0..file_count {
        let file_path = dir.path().join(format!("file_{}.rs", i));
        fs::write(&file_path, format!("//! File {}\nfn main() {{}}", i)).unwrap();
    }

    // Add all files
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();

    dir
}

fn bench_comment_extraction(c: &mut Criterion) {
    let dir = TempDir::new().unwrap();

    // Create test files
    let rust_file = dir.path().join("test.rs");
    let python_file = dir.path().join("test.py");
    let js_file = dir.path().join("test.js");
    let go_file = dir.path().join("test.go");
    let java_file = dir.path().join("Test.java");

    fs::write(&rust_file, RUST_SOURCE).unwrap();
    fs::write(&python_file, PYTHON_SOURCE).unwrap();
    fs::write(&js_file, JS_SOURCE).unwrap();
    fs::write(&go_file, GO_SOURCE).unwrap();
    fs::write(&java_file, JAVA_SOURCE).unwrap();

    let mut group = c.benchmark_group("comment_extraction");

    group.bench_function("rust", |b| {
        b.iter(|| extract_first_comment(black_box(&rust_file)))
    });

    group.bench_function("python", |b| {
        b.iter(|| extract_first_comment(black_box(&python_file)))
    });

    group.bench_function("javascript", |b| {
        b.iter(|| extract_first_comment(black_box(&js_file)))
    });

    group.bench_function("go", |b| {
        b.iter(|| extract_first_comment(black_box(&go_file)))
    });

    group.bench_function("java", |b| {
        b.iter(|| extract_first_comment(black_box(&java_file)))
    });

    group.finish();
}

fn bench_git_filter_init(c: &mut Criterion) {
    let mut group = c.benchmark_group("git_filter_init");

    // Small repo (10 files)
    let small_repo = create_test_repo_with_files(10);
    group.bench_function("small_repo_10_files", |b| {
        b.iter(|| GitFilter::new(black_box(small_repo.path())))
    });

    // Medium repo (100 files)
    let medium_repo = create_test_repo_with_files(100);
    group.bench_function("medium_repo_100_files", |b| {
        b.iter(|| GitFilter::new(black_box(medium_repo.path())))
    });

    // Larger repo (500 files)
    let large_repo = create_test_repo_with_files(500);
    group.bench_function("large_repo_500_files", |b| {
        b.iter(|| GitFilter::new(black_box(large_repo.path())))
    });

    group.finish();
}

fn bench_gitignore_filter_init(c: &mut Criterion) {
    let mut group = c.benchmark_group("gitignore_filter_init");

    // Small repo (10 files)
    let small_repo = create_test_repo_with_files(10);
    group.bench_function("small_repo_10_files", |b| {
        b.iter(|| GitignoreFilter::new(black_box(small_repo.path())))
    });

    // Medium repo (100 files)
    let medium_repo = create_test_repo_with_files(100);
    group.bench_function("medium_repo_100_files", |b| {
        b.iter(|| GitignoreFilter::new(black_box(medium_repo.path())))
    });

    // Larger repo (500 files)
    let large_repo = create_test_repo_with_files(500);
    group.bench_function("large_repo_500_files", |b| {
        b.iter(|| GitignoreFilter::new(black_box(large_repo.path())))
    });

    group.finish();
}

fn bench_git_is_tracked(c: &mut Criterion) {
    let dir = create_test_repo_with_files(100);
    let filter = GitFilter::new(dir.path()).unwrap();

    let tracked_file = dir.path().join("file_50.rs");
    let untracked_file = dir.path().join("nonexistent.rs");

    let mut group = c.benchmark_group("git_is_tracked");

    group.bench_function("tracked_file", |b| {
        b.iter(|| filter.is_tracked(black_box(&tracked_file)))
    });

    group.bench_function("untracked_file", |b| {
        b.iter(|| filter.is_tracked(black_box(&untracked_file)))
    });

    group.bench_function("directory", |b| {
        b.iter(|| filter.is_tracked(black_box(dir.path())))
    });

    group.finish();
}

fn bench_gitignore_is_included(c: &mut Criterion) {
    let dir = create_test_repo_with_files(100);
    let filter = GitignoreFilter::new(dir.path()).unwrap();

    let included_file = dir.path().join("file_50.rs");
    let excluded_file = dir.path().join("nonexistent.rs");

    let mut group = c.benchmark_group("gitignore_is_included");

    group.bench_function("included_file", |b| {
        b.iter(|| filter.is_included(black_box(&included_file)))
    });

    group.bench_function("excluded_file", |b| {
        b.iter(|| filter.is_included(black_box(&excluded_file)))
    });

    group.bench_function("directory", |b| {
        b.iter(|| filter.is_included(black_box(dir.path())))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_comment_extraction,
    bench_git_filter_init,
    bench_gitignore_filter_init,
    bench_git_is_tracked,
    bench_gitignore_is_included,
);
criterion_main!(benches);
