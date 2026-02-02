# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-02-02

### Added
- Add glob pattern and directory copy support for copy_files hook
- Add top-level copy_files and setup_commands configuration support
- Add color theme system with 256-color support

### Changed
- Change prune keybind to D and add footer display

### Chore
- Add color theme configuration to README
- Add mise and nix installation instructions

## [0.3.0] - 2025-01-28

### Added
- Recognize main repo root when running from worktree

### Chore
- Add Nix flake for package distribution
- Add MIT license file
- Add preview GIF to README

## [0.2.0] - 2025-01-27

### Added
- Launch subshell in selected worktree directory on Enter (default behavior)
- Add `-p`/`--print-path` option for shell integration

### Chore
- Add CLAUDE.md for Claude Code guidance
- Improve release skill changelog classification rules
- Extract changelog for GitHub release body in CI

## [0.1.3] - 2025-01-27

### Chore
- Add release skill for version management
- Add automatic Homebrew tap update on release
