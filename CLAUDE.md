# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run

# Test
cargo test                      # Run all tests
cargo test --test git_test      # Run specific test file
cargo test test_move_down       # Run tests matching name

# Check without building
cargo check

# Lint (if clippy installed)
cargo clippy
```

## Architecture Overview

**gwm** is a TUI application for managing Git worktrees, built with Rust + ratatui.

### Core Data Flow

```
KeyEvent → ActionDispatcher → Action → ActionHandler → App State → UI Render
```

### Module Structure

- **`src/app.rs`** - Central application state (`App` struct). Manages:
  - Mode system: `Normal`, `Insert`, `Search`, `Dialog`
  - Worktree/branch lists with selection state
  - Dialog state: `ConfirmDelete`, `CreateWorktree`, `BranchSelect`
  - Notification queue with animations

- **`src/action/`** - Command pattern implementation:
  - `types.rs` - `Action` enum (MoveUp, CreateWorktree, OpenShell, etc.)
  - `dispatcher.rs` - Maps `KeyEvent` to `Action` based on config bindings
  - `handler.rs` - Executes actions, modifies `App` state

- **`src/config/`** - Configuration system:
  - Loads from `~/.config/gwm/config.toml` (global) and `.gwm/config.toml` (local)
  - Local config overrides global; `default.rs` provides base Vim+Emacs bindings
  - Supports hooks (`pre_create`, `post_create`, `pre_delete`, `post_delete`)

- **`src/git/worktree.rs`** - Git operations via git2:
  - `WorktreeManager` wraps git2 for worktree CRUD
  - `Worktree` struct contains path, branch, commit info

- **`src/tui/`** - Terminal rendering:
  - `ui.rs` - Main render orchestration
  - `event.rs` - Async event polling (~60fps tick)
  - `widgets/` - Composable UI components

### Key Patterns

1. **Mode-based keybindings**: Same key can have different actions per mode (see `config/default.rs`)
2. **Dialog state machine**: `DialogKind` enum variants carry their own state
3. **Config layering**: Default → Global → Local with override semantics
4. **Hook system**: Shell commands triggered on worktree lifecycle events

### Test Structure

Tests are in `tests/` directory using `tempfile` for isolated git repos:
- `action_test.rs` - ActionDispatcher behavior
- `navigation_test.rs` - App navigation logic
- `form_test.rs` - CreateWorktree form logic
- `git_test.rs` - WorktreeManager operations
- `common/mod.rs` - `TestRepo` helper for creating temporary git repositories
