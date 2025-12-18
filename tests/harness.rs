//! Test harness for fruit integration tests
//!
//! Re-exports TestRepo from fruit::test_utils and provides run_fruit helper.

use std::path::Path;
use std::process::Command;

// Re-export TestRepo from the shared test utilities
pub use fruit::test_utils::TestRepo;

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
