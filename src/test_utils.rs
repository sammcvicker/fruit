//! Test utilities for creating temporary git repositories.
//!
//! This module is only compiled for tests and benchmarks.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// A temporary git repository for testing.
///
/// Provides methods for creating files, git initialization, and staging files.
/// The repository is automatically cleaned up when dropped.
pub struct TestRepo {
    dir: TempDir,
    git_initialized: bool,
}

impl TestRepo {
    /// Create a new empty temporary directory.
    pub fn new() -> Self {
        let dir = TempDir::new().expect("TestRepo::new: failed to create temporary directory");
        Self {
            dir,
            git_initialized: false,
        }
    }

    /// Create a new temporary directory with git initialized.
    pub fn with_git() -> Self {
        let mut repo = Self::new();
        repo.init_git();
        repo
    }

    /// Get the path to the temporary directory.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Initialize a git repository in the temporary directory.
    ///
    /// Also configures user.email and user.name for commits.
    pub fn init_git(&mut self) {
        let repo_path = self.dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap_or_else(|e| panic!("TestRepo::init_git: 'git init' failed at {:?}: {}", repo_path, e));

        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(repo_path)
            .output()
            .unwrap_or_else(|e| panic!("TestRepo::init_git: 'git config user.email' failed at {:?}: {}", repo_path, e));

        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(repo_path)
            .output()
            .unwrap_or_else(|e| panic!("TestRepo::init_git: 'git config user.name' failed at {:?}: {}", repo_path, e));

        self.git_initialized = true;
    }

    /// Add a file and stage it if git is initialized.
    ///
    /// Creates parent directories as needed.
    pub fn add_file(&self, path: &str, content: &str) -> PathBuf {
        let full_path = self.dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("TestRepo::add_file: failed to create parent directories for {:?}: {}", full_path, e));
        }
        fs::write(&full_path, content)
            .unwrap_or_else(|e| panic!("TestRepo::add_file: failed to write {:?}: {}", full_path, e));

        if self.git_initialized {
            Command::new("git")
                .args(["add", path])
                .current_dir(self.dir.path())
                .output()
                .unwrap_or_else(|e| panic!("TestRepo::add_file: 'git add {}' failed: {}", path, e));
        }

        full_path
    }

    /// Add a file without staging it.
    ///
    /// Creates parent directories as needed.
    pub fn add_untracked(&self, path: &str, content: &str) -> PathBuf {
        let full_path = self.dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("TestRepo::add_untracked: failed to create parent directories for {:?}: {}", full_path, e));
        }
        fs::write(&full_path, content)
            .unwrap_or_else(|e| panic!("TestRepo::add_untracked: failed to write {:?}: {}", full_path, e));
        full_path
    }

    /// Stage all files in the repository.
    pub fn stage_all(&self) {
        if self.git_initialized {
            let repo_path = self.dir.path();
            Command::new("git")
                .args(["add", "."])
                .current_dir(repo_path)
                .output()
                .unwrap_or_else(|e| panic!("TestRepo::stage_all: 'git add .' failed at {:?}: {}", repo_path, e));
        }
    }

    /// Create a commit with the given message.
    pub fn commit(&self, message: &str) {
        assert!(self.git_initialized, "TestRepo::commit: git not initialized - call init_git() first");
        let repo_path = self.dir.path();
        Command::new("git")
            .args(["commit", "-m", message, "--allow-empty"])
            .current_dir(repo_path)
            .output()
            .unwrap_or_else(|e| panic!("TestRepo::commit: 'git commit -m {:?}' failed at {:?}: {}", message, repo_path, e));
    }
}

impl Default for TestRepo {
    fn default() -> Self {
        Self::new()
    }
}
