# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
# Build
cargo build
cargo build --release

# Test (requires git config for CI)
git config --global user.email "ci@example.com"
git config --global user.name "CI"
git config --global init.defaultBranch main
cargo test

# Run single test
cargo test test_name

# Lint & Format
cargo clippy -- -D warnings
cargo fmt --all --check

# Run locally
cargo run
cargo run -- --help
cargo run -- --config /path/to/config.toml
```

## Architecture Overview

gwm is a TUI application for managing git worktrees, built with ratatui/crossterm for the UI and git2 (libgit2) for git operations.

### Core Flow

```
main.rs (CLI + terminal setup)
    ↓
App (state machine: Normal → Create → Confirm → Help)
    ↓
┌───────────────┬──────────────┐
│ git/worktree  │     ui       │
│ (git2 ops)    │  (ratatui)   │
└───────────────┴──────────────┘
    ↓
hooks (post-creation automation)
```

### Module Responsibilities

- **main.rs** - Entry point, CLI parsing (clap), terminal setup, event loop
- **app.rs** - Application state, business logic, worktree/branch management
- **config/loader.rs** - Multi-level config loading (env > local > global), template parsing
- **git/worktree.rs** - GitManager wrapping git2, worktree CRUD, branch operations
- **ui.rs** - Ratatui rendering for all modes (list view, modals, help)
- **input.rs** - Keyboard event handling per AppMode
- **hooks.rs** - SetupRunner for file copying and post-creation commands

### State Machine (AppMode)

```
Normal ──Ctrl+O──→ Create ──Enter──→ (creates worktree) → Normal
   │                  │
   │ Ctrl+D           └──Esc──→ Normal
   ↓
Confirm ──y/Y/n──→ Normal
   │
   └──Esc──→ Normal

Normal ──?──→ Help ──Enter/Esc/q──→ Normal
```

### Configuration Priority

1. Environment variables (`GWM_WORKTREE_BASEDIR`, `GWM_UI_ICONS`, etc.)
2. Local config (`.gwm.toml` or `.gwm/config.toml` in repo/parent dirs)
3. Global config (`~/.config/gwm/config.toml`)

Config sections: `[worktree]`, `[naming]`, `[ui]`, `[[repository_settings]]`

### Template Variables for Naming

`{branch}`, `{host}`, `{owner}`, `{repository}` - extracted from git remote URL

## Testing

Integration tests use `GitTestRepo` helper in `tests/common/mod.rs` to create temporary git repos. Tests cover worktree lifecycle, branch merging detection, and multi-worktree scenarios.

## Release Process

Tags trigger CI: test → audit → build (13 targets) → release → homebrew tap update

Pre-built binaries for Linux/macOS/Windows (x86_64, ARM64, i686 variants).
