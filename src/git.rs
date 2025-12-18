//! Git repository integration and gitignore filtering

use git2::{Repository, Status};
use ignore::WalkBuilder;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Filter based on .gitignore patterns (respects nested .gitignore files).
/// This is the default behavior - shows files that aren't ignored by gitignore.
///
/// Uses the `ignore` crate (from ripgrep) which handles:
/// - Nested .gitignore files in subdirectories
/// - Global gitignore (~/.config/git/ignore)
/// - .git/info/exclude
/// - Parent directory .gitignore files
pub struct GitignoreFilter {
    included_files: HashSet<PathBuf>,
    included_dirs: HashSet<PathBuf>,
    repo_root: PathBuf,
}

impl GitignoreFilter {
    pub fn new(path: &Path) -> Option<Self> {
        // Find the repository root by looking for .git directory
        let repo_root = Self::find_repo_root(path)?;

        // Use ignore crate's WalkBuilder to collect all non-ignored paths
        // This handles nested .gitignore files automatically
        let mut included_files = HashSet::new();
        let mut included_dirs = HashSet::new();

        let walker = WalkBuilder::new(&repo_root)
            .hidden(false) // Don't skip hidden files (let .gitignore decide)
            .git_ignore(true) // Respect .gitignore
            .git_global(true) // Respect global gitignore
            .git_exclude(true) // Respect .git/info/exclude
            .build();

        for entry in walker.flatten() {
            let entry_path = entry.path().to_path_buf();
            if entry_path.is_file() {
                included_files.insert(entry_path);
            } else if entry_path.is_dir() {
                included_dirs.insert(entry_path);
            }
        }

        // Also add directories that contain included files
        for file_path in &included_files.clone() {
            let mut current = file_path.parent();
            while let Some(dir) = current {
                if !included_dirs.insert(dir.to_path_buf()) {
                    break; // Already added
                }
                current = dir.parent();
            }
        }

        Some(Self {
            included_files,
            included_dirs,
            repo_root,
        })
    }

    fn find_repo_root(path: &Path) -> Option<PathBuf> {
        let mut current = if path.is_file() {
            path.parent()?.to_path_buf()
        } else {
            path.to_path_buf()
        };

        // Canonicalize to handle relative paths
        current = current.canonicalize().ok()?;

        loop {
            if current.join(".git").exists() {
                return Some(current);
            }
            if !current.pop() {
                return None;
            }
        }
    }

    /// Check if a path should be included (not ignored by .gitignore).
    pub fn is_included(&self, path: &Path) -> bool {
        // Canonicalize the path for comparison
        let path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => path.to_path_buf(),
        };

        // Direct file check - O(1)
        if self.included_files.contains(&path) {
            return true;
        }

        // Directory check - O(1)
        if path.is_dir() {
            return self.included_dirs.contains(&path);
        }

        false
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }
}

/// Filter based on git tracking status (files in the git index).
/// Use this with --tracked flag to show only git-tracked files.
pub struct GitFilter {
    tracked_files: HashSet<PathBuf>,
    tracked_dirs: HashSet<PathBuf>,
    repo_root: PathBuf,
}

impl GitFilter {
    pub fn new(path: &Path) -> Option<Self> {
        let repo = Repository::discover(path).ok()?;
        let repo_root = repo.workdir()?.to_path_buf();
        let tracked_files = Self::collect_tracked_files(&repo, &repo_root)?;

        // Pre-compute all directories containing tracked files for O(1) lookup
        let mut tracked_dirs = HashSet::new();
        for file_path in &tracked_files {
            let mut current = file_path.parent();
            while let Some(dir) = current {
                if !tracked_dirs.insert(dir.to_path_buf()) {
                    // Already seen this directory, ancestors are also already added
                    break;
                }
                current = dir.parent();
            }
        }

        Some(Self {
            tracked_files,
            tracked_dirs,
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

        // Direct file check - O(1)
        if self.tracked_files.contains(&path) {
            return true;
        }

        // Directory check - O(1) using pre-computed set
        if path.is_dir() {
            return self.tracked_dirs.contains(&path);
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

    // Tests for GitignoreFilter
    #[test]
    fn test_gitignore_basic() {
        let dir = create_test_repo();

        // Create .gitignore
        fs::write(dir.path().join(".gitignore"), "*.log\ntarget/\n").unwrap();

        // Create files
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("debug.log"), "log content").unwrap();

        let filter = GitignoreFilter::new(dir.path()).unwrap();

        // main.rs should be included
        assert!(filter.is_included(&dir.path().join("main.rs")));
        // debug.log should be excluded (matches *.log)
        assert!(!filter.is_included(&dir.path().join("debug.log")));
    }

    #[test]
    fn test_gitignore_directory() {
        let dir = create_test_repo();

        // Create .gitignore
        fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();

        // Create directories and files
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::create_dir_all(dir.path().join("target")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("target/debug"), "binary").unwrap();

        let filter = GitignoreFilter::new(dir.path()).unwrap();

        // src directory should be included
        assert!(filter.is_included(&dir.path().join("src")));
        // src/main.rs should be included
        assert!(filter.is_included(&dir.path().join("src/main.rs")));
        // target directory should be excluded
        assert!(!filter.is_included(&dir.path().join("target")));
        // target/debug should be excluded
        assert!(!filter.is_included(&dir.path().join("target/debug")));
    }

    #[test]
    fn test_gitignore_nested() {
        let dir = create_test_repo();

        // Create root .gitignore
        fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();

        // Create subdir with its own .gitignore
        fs::create_dir_all(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir/.gitignore"), "*.tmp\n").unwrap();

        // Create files
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("subdir/code.rs"), "fn code() {}").unwrap();
        fs::write(dir.path().join("subdir/cache.tmp"), "temp").unwrap();
        fs::write(dir.path().join("root.log"), "log").unwrap();
        fs::write(dir.path().join("subdir/nested.log"), "log").unwrap();

        let filter = GitignoreFilter::new(dir.path()).unwrap();

        // main.rs should be included
        assert!(filter.is_included(&dir.path().join("main.rs")));
        // subdir/code.rs should be included
        assert!(filter.is_included(&dir.path().join("subdir/code.rs")));
        // subdir/cache.tmp should be excluded (nested .gitignore)
        assert!(!filter.is_included(&dir.path().join("subdir/cache.tmp")));
        // root.log should be excluded (root .gitignore)
        assert!(!filter.is_included(&dir.path().join("root.log")));
        // subdir/nested.log should be excluded (root .gitignore applies to subdirs)
        assert!(!filter.is_included(&dir.path().join("subdir/nested.log")));
    }

    #[test]
    fn test_gitignore_negation() {
        let dir = create_test_repo();

        // Create .gitignore with negation pattern
        fs::write(dir.path().join(".gitignore"), "*.log\n!important.log\n").unwrap();

        // Create files
        fs::write(dir.path().join("debug.log"), "debug").unwrap();
        fs::write(dir.path().join("important.log"), "important").unwrap();

        let filter = GitignoreFilter::new(dir.path()).unwrap();

        // debug.log should be excluded
        assert!(!filter.is_included(&dir.path().join("debug.log")));
        // important.log should be included (negation pattern)
        assert!(filter.is_included(&dir.path().join("important.log")));
    }
}
