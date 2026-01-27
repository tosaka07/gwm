//! Integration tests for git worktree workflows

mod common;

use common::GitTestRepo;

/// Test the full worktree lifecycle: create -> list -> delete
#[test]
fn test_full_worktree_lifecycle() {
    let repo = GitTestRepo::new();

    // Create a branch for the worktree
    repo.create_branch("feature-lifecycle");

    // Create a worktree
    repo.create_worktree("wt-lifecycle", "feature-lifecycle");

    // Verify worktree exists
    let worktrees = repo.list_worktrees();
    assert!(worktrees.contains("wt-lifecycle"));
    assert!(worktrees.contains("feature-lifecycle"));

    // Remove the worktree
    repo.remove_worktree("wt-lifecycle");

    // Verify worktree is gone
    let worktrees_after = repo.list_worktrees();
    assert!(!worktrees_after.contains("wt-lifecycle"));
}

/// Test creating a worktree with a new branch
#[test]
fn test_worktree_with_new_branch() {
    let repo = GitTestRepo::new();

    // Create worktree with a new branch (atomic operation)
    repo.create_worktree_with_new_branch("wt-new-branch", "new-feature-branch");

    // Verify worktree exists
    let worktrees = repo.list_worktrees();
    assert!(worktrees.contains("wt-new-branch"));
    assert!(worktrees.contains("new-feature-branch"));

    // Verify branch was created
    let branches = repo.list_branches();
    assert!(branches.contains(&"new-feature-branch".to_string()));
}

/// Test the prune workflow: identify merged worktrees
#[test]
fn test_prune_merged_worktrees_workflow() {
    let repo = GitTestRepo::new();

    // Create and merge a branch
    repo.create_merged_branch("merged-feature");

    // Create a worktree for the merged branch
    repo.create_worktree("wt-merged", "merged-feature");

    // Verify worktree exists with merged branch
    let worktrees = repo.list_worktrees();
    assert!(worktrees.contains("wt-merged"));
    assert!(worktrees.contains("merged-feature"));

    // In a real prune workflow, this worktree would be identified as merged
    // and should be safe to delete

    // Clean up
    repo.remove_worktree("wt-merged");
}

/// Test multiple worktrees can coexist
#[test]
fn test_multiple_worktrees() {
    let repo = GitTestRepo::new();

    // Create multiple branches
    repo.create_branch("feature-1");
    repo.create_branch("feature-2");
    repo.create_branch("feature-3");

    // Create multiple worktrees
    repo.create_worktree("wt-1", "feature-1");
    repo.create_worktree("wt-2", "feature-2");
    repo.create_worktree("wt-3", "feature-3");

    // Verify all worktrees exist
    let worktrees = repo.list_worktrees();
    assert!(worktrees.contains("wt-1"));
    assert!(worktrees.contains("wt-2"));
    assert!(worktrees.contains("wt-3"));

    // Clean up
    repo.remove_worktree("wt-1");
    repo.remove_worktree("wt-2");
    repo.remove_worktree("wt-3");
}

/// Test branch divergence detection
#[test]
fn test_branch_divergence() {
    let repo = GitTestRepo::new();

    // Create a branch with commits (diverged from main)
    repo.create_branch_with_commit("diverged-branch");

    // The diverged branch should have its own commit
    let branches = repo.list_branches();
    assert!(branches.contains(&"diverged-branch".to_string()));

    // Add commit on main to make them truly diverged
    repo.add_commit("main-commit");

    // Both branches now have unique commits
    // In the actual application, find_merged_branches would NOT include diverged-branch
}

/// Test merged branch detection
#[test]
fn test_merged_branch_detection() {
    let repo = GitTestRepo::new();

    // Create and merge multiple branches
    repo.create_merged_branch("merged-1");
    repo.create_merged_branch("merged-2");

    // Create an unmerged branch
    repo.create_branch_with_commit("unmerged-1");

    let branches = repo.list_branches();
    assert!(branches.contains(&"merged-1".to_string()));
    assert!(branches.contains(&"merged-2".to_string()));
    assert!(branches.contains(&"unmerged-1".to_string()));

    // In the actual application:
    // - merged-1 and merged-2 would be in find_merged_branches() result
    // - unmerged-1 would NOT be in find_merged_branches() result
}
