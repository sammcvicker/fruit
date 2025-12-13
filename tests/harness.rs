//! Test harness for fruit integration tests

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

pub struct TestRepo {
    dir: TempDir,
    git_initialized: bool,
}

impl TestRepo {
    pub fn new() -> Self {
        let dir = TempDir::new().expect("Failed to create temp dir");
        Self {
            dir,
            git_initialized: false,
        }
    }

    pub fn with_git() -> Self {
        let mut repo = Self::new();
        repo.init_git();
        repo
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn init_git(&mut self) {
        Command::new("git")
            .args(["init"])
            .current_dir(self.dir.path())
            .output()
            .expect("Failed to init git");

        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(self.dir.path())
            .output()
            .expect("Failed to set git email");

        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(self.dir.path())
            .output()
            .expect("Failed to set git name");

        self.git_initialized = true;
    }

    pub fn add_file(&self, path: &str, content: &str) -> PathBuf {
        let full_path = self.dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dirs");
        }
        fs::write(&full_path, content).expect("Failed to write file");

        if self.git_initialized {
            Command::new("git")
                .args(["add", path])
                .current_dir(self.dir.path())
                .output()
                .expect("Failed to git add");
        }

        full_path
    }

    pub fn add_untracked(&self, path: &str, content: &str) -> PathBuf {
        let full_path = self.dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dirs");
        }
        fs::write(&full_path, content).expect("Failed to write file");
        full_path
    }

    pub fn commit(&self, message: &str) {
        assert!(self.git_initialized, "Git not initialized");
        Command::new("git")
            .args(["commit", "-m", message, "--allow-empty"])
            .current_dir(self.dir.path())
            .output()
            .expect("Failed to commit");
    }
}

pub fn run_fruit(dir: &Path, args: &[&str]) -> (String, String, bool) {
    let binary = env!("CARGO_BIN_EXE_fruit");
    let output = Command::new(binary)
        .args(args)
        .current_dir(dir)
        .output()
        .expect("Failed to run fruit");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creates_temp_dir() {
        let repo = TestRepo::new();
        assert!(repo.path().exists());
    }

    #[test]
    fn test_harness_git_init() {
        let repo = TestRepo::with_git();
        assert!(repo.path().join(".git").exists());
    }

    #[test]
    fn test_harness_add_file() {
        let repo = TestRepo::with_git();
        let file_path = repo.add_file("test.rs", "fn main() {}");
        assert!(file_path.exists());
    }
}
