//! Common test utilities

use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary git repository for testing
pub struct TestRepo {
    pub dir: TempDir,
    pub path: PathBuf,
}

impl TestRepo {
    /// Create a new test repository with initial commit
    pub fn new() -> Self {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let path = dir.path().to_path_buf();

        // Initialize git repository
        let repo = git2::Repository::init(&path).expect("Failed to init repo");

        // Create initial commit
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("Failed to create initial commit");

        Self { dir, path }
    }

    /// Create additional branches
    pub fn create_branch(&self, name: &str) {
        let repo = git2::Repository::open(&self.path).unwrap();
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        repo.branch(name, &commit, false).unwrap();
    }

    /// Get the repository path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Default for TestRepo {
    fn default() -> Self {
        Self::new()
    }
}
