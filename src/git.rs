//! Git repository integration

use git2::{Repository, Status};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct GitFilter {
    #[allow(dead_code)]
    repo: Repository,
    tracked_files: HashSet<PathBuf>,
    repo_root: PathBuf,
}

impl GitFilter {
    pub fn new(path: &Path) -> Option<Self> {
        let repo = Repository::discover(path).ok()?;
        let repo_root = repo.workdir()?.to_path_buf();
        let tracked_files = Self::collect_tracked_files(&repo, &repo_root)?;

        Some(Self {
            repo,
            tracked_files,
            repo_root,
        })
    }

    fn collect_tracked_files(repo: &Repository, repo_root: &Path) -> Option<HashSet<PathBuf>> {
        let mut tracked = HashSet::new();

        // Get all files from the index (staged/tracked files)
        let index = repo.index().ok()?;
        for entry in index.iter() {
            let path_str = String::from_utf8_lossy(&entry.path);
            let full_path = repo_root.join(path_str.as_ref());
            tracked.insert(full_path);
        }

        // Also check status for any tracked files with modifications
        let statuses = repo.statuses(None).ok()?;
        for entry in statuses.iter() {
            let status = entry.status();
            // Include files that are tracked (not new/untracked)
            if !status.contains(Status::WT_NEW) && !status.contains(Status::IGNORED) {
                if let Some(path) = entry.path() {
                    let full_path = repo_root.join(path);
                    tracked.insert(full_path);
                }
            }
        }

        Some(tracked)
    }

    pub fn is_tracked(&self, path: &Path) -> bool {
        // Canonicalize the path for comparison
        let path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => path.to_path_buf(),
        };

        // Direct file check
        if self.tracked_files.contains(&path) {
            return true;
        }

        // Check if it's a directory containing tracked files
        if path.is_dir() {
            return self.tracked_files.iter().any(|tracked| {
                tracked.starts_with(&path)
            });
        }

        false
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_repo() -> TempDir {
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

        dir
    }

    #[test]
    fn test_tracked_file() {
        let dir = create_test_repo();
        let file_path = dir.path().join("tracked.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        Command::new("git")
            .args(["add", "tracked.rs"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let filter = GitFilter::new(dir.path()).unwrap();
        assert!(filter.is_tracked(&file_path));
    }

    #[test]
    fn test_untracked_file() {
        let dir = create_test_repo();
        let tracked = dir.path().join("tracked.rs");
        let untracked = dir.path().join("untracked.rs");

        fs::write(&tracked, "fn main() {}").unwrap();
        fs::write(&untracked, "fn other() {}").unwrap();

        Command::new("git")
            .args(["add", "tracked.rs"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let filter = GitFilter::new(dir.path()).unwrap();
        assert!(filter.is_tracked(&tracked));
        assert!(!filter.is_tracked(&untracked));
    }
}
