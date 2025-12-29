//! Tests for Git operations (WorktreeManager)

mod common;

use common::TestRepo;
use gwm::git::WorktreeManager;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Generate unique worktree path to avoid conflicts in parallel tests
fn unique_worktree_path(base: &Path, prefix: &str) -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    base.join(format!("{}-{}-{}", prefix, std::process::id(), id))
}

// ===================
// WorktreeManager initialization tests
// ===================

#[test]
fn test_worktree_manager_new_in_valid_repo() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new();
    assert!(manager.is_ok());
}

#[test]
fn test_worktree_manager_repo_root() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    // Use canonicalize to handle macOS /var -> /private/var symlink
    let expected = repo.path().canonicalize().unwrap();
    let actual = manager.repo_root().canonicalize().unwrap();
    assert_eq!(actual, expected);
}

// ===================
// Worktree listing tests
// ===================

#[test]
fn test_list_worktrees_initial() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let worktrees = manager.list().unwrap();

    // Should have exactly one worktree (main)
    assert_eq!(worktrees.len(), 1);
    assert!(worktrees[0].is_main);
}

#[test]
fn test_list_worktrees_main_has_branch() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let worktrees = manager.list().unwrap();

    let main_wt = &worktrees[0];
    assert!(main_wt.branch.is_some());
}

#[test]
fn test_list_worktrees_main_has_commit_info() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let worktrees = manager.list().unwrap();

    let main_wt = &worktrees[0];
    assert!(!main_wt.commit_hash.is_empty());
    assert!(!main_wt.commit_message.is_empty());
    assert!(!main_wt.commit_author.is_empty());
    assert!(!main_wt.commit_date.is_empty());
}

// ===================
// Branch listing tests
// ===================

#[test]
fn test_list_branches_initial() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let branches = manager.list_branches().unwrap();

    // Should have at least one branch (main or master)
    assert!(!branches.is_empty());
}

#[test]
fn test_list_branches_with_created_branches() {
    let repo = TestRepo::new();
    repo.create_branch("feature-1");
    repo.create_branch("feature-2");
    repo.create_branch("develop");
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let branches = manager.list_branches().unwrap();

    // Should have main + 3 created branches
    assert!(branches.len() >= 4);
    assert!(branches.iter().any(|b| b == "feature-1"));
    assert!(branches.iter().any(|b| b == "feature-2"));
    assert!(branches.iter().any(|b| b == "develop"));
}

// ===================
// Worktree creation tests
// ===================

#[test]
fn test_create_worktree_new_branch() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let wt_path = unique_worktree_path(repo.path().parent().unwrap(), "new-worktree");

    let result = manager.create(&wt_path, "new-feature", None);
    assert!(result.is_ok());

    let wt = result.unwrap();
    // Use canonicalize to handle macOS /var -> /private/var symlink
    let expected_path = wt_path.canonicalize().unwrap();
    let actual_path = wt.path.canonicalize().unwrap();
    assert_eq!(actual_path, expected_path);
    assert_eq!(wt.branch, Some("new-feature".to_string()));
    assert!(!wt.is_main);

    // Verify worktree directory was created
    assert!(wt_path.exists());

    // Cleanup
    let _ = fs::remove_dir_all(&wt_path);
}

#[test]
fn test_create_worktree_existing_branch() {
    let repo = TestRepo::new();
    repo.create_branch("existing-branch");
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let wt_path = unique_worktree_path(repo.path().parent().unwrap(), "existing-wt");

    let result = manager.create(&wt_path, "existing-branch", None);
    assert!(result.is_ok());

    let wt = result.unwrap();
    assert_eq!(wt.branch, Some("existing-branch".to_string()));

    // Cleanup
    let _ = fs::remove_dir_all(&wt_path);
}

#[test]
fn test_create_worktree_based_on_branch() {
    let repo = TestRepo::new();
    repo.create_branch("develop");
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let wt_path = unique_worktree_path(repo.path().parent().unwrap(), "feature-from-develop");

    let result = manager.create(&wt_path, "feature-x", Some("develop"));
    assert!(result.is_ok());

    let wt = result.unwrap();
    assert_eq!(wt.branch, Some("feature-x".to_string()));

    // Cleanup
    let _ = fs::remove_dir_all(&wt_path);
}

#[test]
fn test_create_worktree_appears_in_list() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let wt_path = unique_worktree_path(repo.path().parent().unwrap(), "listed-wt");

    manager.create(&wt_path, "listed-branch", None).unwrap();

    let worktrees = manager.list().unwrap();
    assert_eq!(worktrees.len(), 2); // main + new worktree

    let new_wt = worktrees.iter().find(|w| !w.is_main).unwrap();
    assert_eq!(new_wt.branch, Some("listed-branch".to_string()));

    // Cleanup
    let _ = fs::remove_dir_all(&wt_path);
}

// ===================
// Worktree removal tests
// ===================

#[test]
fn test_remove_worktree() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let wt_path = unique_worktree_path(repo.path().parent().unwrap(), "to-remove");

    // Create worktree
    let wt = manager.create(&wt_path, "to-remove-branch", None).unwrap();
    assert!(wt_path.exists());

    // Remove worktree - use the path returned by git2 (handles symlinks correctly)
    let result = manager.remove(&wt.path, true);
    assert!(result.is_ok());
    assert!(!wt_path.exists());
}

#[test]
fn test_remove_worktree_updates_list() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let wt_path = unique_worktree_path(repo.path().parent().unwrap(), "to-remove-listed");

    // Create worktree
    let wt = manager
        .create(&wt_path, "to-remove-listed-branch", None)
        .unwrap();

    let worktrees_before = manager.list().unwrap();
    assert_eq!(worktrees_before.len(), 2);

    // Remove worktree - use the path returned by git2 (handles symlinks correctly)
    manager.remove(&wt.path, true).unwrap();

    let worktrees_after = manager.list().unwrap();
    assert_eq!(worktrees_after.len(), 1);
}

#[test]
fn test_remove_nonexistent_worktree_fails() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let fake_path = Path::new("/nonexistent/path");

    let result = manager.remove(fake_path, true);
    assert!(result.is_err());
}

// ===================
// Merged worktree tests
// ===================

#[test]
fn test_find_merged_worktrees_empty_when_no_worktrees() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();

    // Get the main branch name (could be main or master)
    let branches = manager.list_branches().unwrap();
    let main_branch = branches.first().unwrap();

    let merged = manager.find_merged_worktrees(main_branch).unwrap();
    assert!(merged.is_empty());
}

#[test]
fn test_find_merged_worktrees_excludes_main() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let branches = manager.list_branches().unwrap();
    let main_branch = branches.first().unwrap();

    let merged = manager.find_merged_worktrees(main_branch).unwrap();

    // Main worktree should never be in the merged list
    assert!(!merged.iter().any(|wt| wt.is_main));
}

// ===================
// Worktree struct tests
// ===================

#[test]
fn test_worktree_path_is_absolute() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let worktrees = manager.list().unwrap();

    for wt in worktrees {
        assert!(wt.path.is_absolute());
    }
}

#[test]
fn test_worktree_commit_hash_is_valid() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let worktrees = manager.list().unwrap();

    for wt in worktrees {
        // Git commit hashes are 40 hex characters
        assert_eq!(wt.commit_hash.len(), 40);
        assert!(wt.commit_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

// ===================
// Multiple worktree tests
// ===================

#[test]
fn test_multiple_worktrees() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let base_path = repo.path().parent().unwrap();

    // Create multiple worktrees
    let wt1_path = unique_worktree_path(base_path, "wt-1");
    let wt2_path = unique_worktree_path(base_path, "wt-2");
    let wt3_path = unique_worktree_path(base_path, "wt-3");

    manager.create(&wt1_path, "branch-1", None).unwrap();
    manager.create(&wt2_path, "branch-2", None).unwrap();
    manager.create(&wt3_path, "branch-3", None).unwrap();

    let worktrees = manager.list().unwrap();
    assert_eq!(worktrees.len(), 4); // main + 3

    // Cleanup
    let _ = fs::remove_dir_all(&wt1_path);
    let _ = fs::remove_dir_all(&wt2_path);
    let _ = fs::remove_dir_all(&wt3_path);
}

#[test]
fn test_worktrees_have_unique_branches() {
    let repo = TestRepo::new();
    std::env::set_current_dir(repo.path()).unwrap();

    let manager = WorktreeManager::new().unwrap();
    let base_path = repo.path().parent().unwrap();

    let wt1_path = unique_worktree_path(base_path, "unique-1");
    let wt2_path = unique_worktree_path(base_path, "unique-2");

    manager.create(&wt1_path, "unique-branch-1", None).unwrap();
    manager.create(&wt2_path, "unique-branch-2", None).unwrap();

    let worktrees = manager.list().unwrap();
    let branches: Vec<_> = worktrees
        .iter()
        .filter_map(|wt| wt.branch.as_ref())
        .collect();

    // All branches should be unique
    let unique_count = {
        let mut seen = std::collections::HashSet::new();
        branches.iter().filter(|b| seen.insert(*b)).count()
    };
    assert_eq!(unique_count, branches.len());

    // Cleanup
    let _ = fs::remove_dir_all(&wt1_path);
    let _ = fs::remove_dir_all(&wt2_path);
}
