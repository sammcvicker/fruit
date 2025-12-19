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
        for file_path in &included_files {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestRepo;

    #[test]
    fn test_tracked_file() {
        let repo = TestRepo::with_git();
        let file_path = repo.add_file("tracked.rs", "fn main() {}");

        let filter = GitFilter::new(repo.path()).unwrap();
        assert!(filter.is_tracked(&file_path));
    }

    #[test]
    fn test_untracked_file() {
        let repo = TestRepo::with_git();
        let tracked = repo.add_file("tracked.rs", "fn main() {}");
        let untracked = repo.add_untracked("untracked.rs", "fn other() {}");

        let filter = GitFilter::new(repo.path()).unwrap();
        assert!(filter.is_tracked(&tracked));
        assert!(!filter.is_tracked(&untracked));
    }

    // Tests for GitignoreFilter
    #[test]
    fn test_gitignore_basic() {
        let repo = TestRepo::with_git();

        // Create .gitignore
        repo.add_untracked(".gitignore", "*.log\ntarget/\n");

        // Create files
        repo.add_untracked("main.rs", "fn main() {}");
        repo.add_untracked("debug.log", "log content");

        let filter = GitignoreFilter::new(repo.path()).unwrap();

        // main.rs should be included
        assert!(filter.is_included(&repo.path().join("main.rs")));
        // debug.log should be excluded (matches *.log)
        assert!(!filter.is_included(&repo.path().join("debug.log")));
    }

    #[test]
    fn test_gitignore_directory() {
        let repo = TestRepo::with_git();

        // Create .gitignore
        repo.add_untracked(".gitignore", "target/\n");

        // Create directories and files
        repo.add_untracked("src/main.rs", "fn main() {}");
        repo.add_untracked("target/debug", "binary");

        let filter = GitignoreFilter::new(repo.path()).unwrap();

        // src directory should be included
        assert!(filter.is_included(&repo.path().join("src")));
        // src/main.rs should be included
        assert!(filter.is_included(&repo.path().join("src/main.rs")));
        // target directory should be excluded
        assert!(!filter.is_included(&repo.path().join("target")));
        // target/debug should be excluded
        assert!(!filter.is_included(&repo.path().join("target/debug")));
    }

    #[test]
    fn test_gitignore_nested() {
        let repo = TestRepo::with_git();

        // Create root .gitignore
        repo.add_untracked(".gitignore", "*.log\n");

        // Create subdir with its own .gitignore
        repo.add_untracked("subdir/.gitignore", "*.tmp\n");

        // Create files
        repo.add_untracked("main.rs", "fn main() {}");
        repo.add_untracked("subdir/code.rs", "fn code() {}");
        repo.add_untracked("subdir/cache.tmp", "temp");
        repo.add_untracked("root.log", "log");
        repo.add_untracked("subdir/nested.log", "log");

        let filter = GitignoreFilter::new(repo.path()).unwrap();

        // main.rs should be included
        assert!(filter.is_included(&repo.path().join("main.rs")));
        // subdir/code.rs should be included
        assert!(filter.is_included(&repo.path().join("subdir/code.rs")));
        // subdir/cache.tmp should be excluded (nested .gitignore)
        assert!(!filter.is_included(&repo.path().join("subdir/cache.tmp")));
        // root.log should be excluded (root .gitignore)
        assert!(!filter.is_included(&repo.path().join("root.log")));
        // subdir/nested.log should be excluded (root .gitignore applies to subdirs)
        assert!(!filter.is_included(&repo.path().join("subdir/nested.log")));
    }

    #[test]
    fn test_gitignore_negation() {
        let repo = TestRepo::with_git();

        // Create .gitignore with negation pattern
        repo.add_untracked(".gitignore", "*.log\n!important.log\n");

        // Create files
        repo.add_untracked("debug.log", "debug");
        repo.add_untracked("important.log", "important");

        let filter = GitignoreFilter::new(repo.path()).unwrap();

        // debug.log should be excluded
        assert!(!filter.is_included(&repo.path().join("debug.log")));
        // important.log should be included (negation pattern)
        assert!(filter.is_included(&repo.path().join("important.log")));
    }
}
