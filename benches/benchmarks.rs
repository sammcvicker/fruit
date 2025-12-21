//! Performance benchmarks for fruit

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fruit::{
    GitignoreFilter, OutputConfig, StreamingFormatter, StreamingWalker, WalkerConfig,
    extract_first_comment, test_utils::TestRepo,
};
use std::fs;
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

/// Create a test repository with the specified number of files.
fn create_test_repo_with_files(file_count: usize) -> TestRepo {
    let repo = TestRepo::with_git();

    // Create files
    for i in 0..file_count {
        repo.add_file(
            &format!("file_{}.rs", i),
            &format!("//! File {}\nfn main() {{}}", i),
        );
    }

    repo
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

fn bench_parallel_extraction(c: &mut Criterion) {
    // Create a repo with many files for benchmarking
    let repo = create_test_repo_with_files(200);

    let mut group = c.benchmark_group("parallel_extraction");
    group.sample_size(20); // Reduce sample size for slower benchmarks

    // Sequential extraction (-j1)
    group.bench_function("sequential_j1", |b| {
        b.iter(|| {
            let config = WalkerConfig {
                extract_comments: true,
                parallel_workers: 1, // Sequential
                ..Default::default()
            };
            let walker = StreamingWalker::new(config);
            let mut formatter = StreamingFormatter::new(OutputConfig::default());
            let _ = walker.walk_streaming(black_box(repo.path()), &mut formatter);
        })
    });

    // Parallel extraction with auto-detect workers (-j0)
    group.bench_function("parallel_j0_auto", |b| {
        b.iter(|| {
            let config = WalkerConfig {
                extract_comments: true,
                parallel_workers: 0, // Auto-detect
                ..Default::default()
            };
            let walker = StreamingWalker::new(config);
            let mut formatter = StreamingFormatter::new(OutputConfig::default());
            let _ = walker.walk_streaming(black_box(repo.path()), &mut formatter);
        })
    });

    // Parallel extraction with 4 workers
    group.bench_function("parallel_j4", |b| {
        b.iter(|| {
            let config = WalkerConfig {
                extract_comments: true,
                parallel_workers: 4,
                ..Default::default()
            };
            let walker = StreamingWalker::new(config);
            let mut formatter = StreamingFormatter::new(OutputConfig::default());
            let _ = walker.walk_streaming(black_box(repo.path()), &mut formatter);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_comment_extraction,
    bench_gitignore_filter_init,
    bench_gitignore_is_included,
    bench_parallel_extraction,
);
criterion_main!(benches);
