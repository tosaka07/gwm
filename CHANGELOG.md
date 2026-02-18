# Changelog

All notable changes to this project will be documented in this file.

## [0.5.0] - 2026-02-18

### Added
- Replace help overlay with config viewer dialog
- Add ConfigSources to expose per-level config info
- Display mode-specific key bindings in footer for overlay dialogs

### Changed
- Move key hints into dialog borders using title_bottom

### Fixed
- Clear area around overlay dialogs for readable borders

## [0.4.1] - 2026-02-13

### Added
- Run worktree deletion in background with spinner animation to prevent UI freezing

### Fixed
- Fix confirm dialog using wrong worktree list when search filter is active
- Detect and report unreplaced template variables in naming configuration
- Support ssh:// and git:// protocol URLs in remote URL parsing

### Changed
- Unify confirm dialog shortcut styling with color-coded key/description spans

### Chore
- Update dependencies to fix security advisories

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
