# gwm - Git Worktree Manager

A fast and intuitive TUI application for managing git worktrees.

<img src="https://raw.githubusercontent.com/tosaka07/gwm/main/docs/preview.gif" width=640>

## Features

- Browse and switch between worktrees
- Create new worktrees from existing or new branches
- Delete worktrees (with optional branch deletion)
- Prune merged worktrees
- Fuzzy search/filter worktrees
- NerdFont icons support
- Customizable color themes (256-color/True Color support)

## Installation

### Homebrew (macOS/Linux)

```bash
brew install tosaka07/tap/gwm
```

To upgrade:

```bash
brew upgrade gwm
```

### Cargo

```bash
cargo install --git https://github.com/tosaka07/gwm
```

### mise

Using the [Cargo backend](https://mise.jdx.dev/dev-tools/backends/cargo.html):

```bash
mise use -g cargo:gwm
```

### Nix

Using flakes:

```bash
# Run without installing
nix run github:tosaka07/gwm

# Install to profile
nix profile install github:tosaka07/gwm
```

Or add to your `flake.nix` inputs:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    gwm = {
      url = "github:tosaka07/gwm";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, gwm, ... }: {
    # Example: add to home-manager packages
    # gwm.packages.${system}.default
  };
}
```

## Usage

```bash
# Run in any git repository
# Opens a subshell in the selected worktree directory
gwm

# Print selected worktree path to stdout (for shell integration)
gwm -p
gwm --print-path
```

### Shell Integration

By default, `gwm` opens a subshell in the selected worktree directory. When you exit the subshell (`exit` or `Ctrl-D`), you return to the original directory.

If you prefer to change the current shell's directory instead, use the `-p` flag with a shell function:

```bash
# Add to your .bashrc or .zshrc
gwt() {
  local path=$(gwm --print-path)
  if [ -n "$path" ]; then
    cd "$path"
  fi
}
```

## Keybindings

### Normal Mode

| Key | Action |
|-----|--------|
| `↑` / `C-p` | Move up |
| `↓` / `C-n` | Move down |
| `Enter` | Open selected worktree |
| `C-o` | Create new worktree |
| `C-d` | Delete worktree |
| `C-D` | Prune merged worktrees |
| `?` | Show help |
| `C-q` / `Esc` | Quit |
| `a-z` | Filter worktrees |

### Create Mode

| Key | Action |
|-----|--------|
| `↑` / `C-p` | Move up |
| `↓` / `C-n` | Move down |
| `Enter` | Create worktree |
| `Esc` / `C-c` | Cancel |

### Delete Confirmation

| Key | Action |
|-----|--------|
| `y` | Delete worktree only |
| `Y` | Delete worktree and branch |
| `n` / `Esc` | Cancel |

## Configuration

gwm loads configuration from three sources (higher priority wins):

| Priority | Source | Path / Prefix |
|----------|--------|---------------|
| 1 (highest) | Environment | `GWM_*` |
| 2 | Local | `.gwm.toml` or `.gwm/config.toml` |
| 3 (lowest) | Global | `~/.config/gwm/config.toml` |

### Environment Variables

| Variable | Type | Description |
|----------|------|-------------|
| `GWM_WORKTREE_BASEDIR` | string | Base directory for new worktrees |
| `GWM_WORKTREE_AUTO_MKDIR` | bool | Auto-create base directory |
| `GWM_UI_ICONS` | bool | Show NerdFont icons |
| `GWM_UI_TILDE_HOME` | bool | Display `~` instead of home path |
| `GWM_UI_THEME` | string | Color theme (`default` or `classic`) |

Boolean values: `true`, `1`, `yes` or `false`, `0`, `no`

### Full Configuration Example

```toml
[worktree]
# Base directory for new worktrees
# Default: "~/worktrees"
# Supports:
#   - Absolute paths: /path/to/worktrees
#   - Home directory: ~/worktrees
#   - Relative paths (from repo root): .git/wt, ../worktrees
basedir = "~/worktrees"

# Automatically create base directory if it doesn't exist
# Default: true
auto_mkdir = true

[naming]
# Directory naming template
# Supports variables: {branch}, {host}, {owner}, {repository}
# These are extracted from the origin remote URL
# Examples:
#   "wt-{branch}"                          -> feature/login becomes wt-feature-login
#   "{branch}-dev"                         -> main becomes main-dev
#   "{host}/{owner}/{repository}/{branch}" -> ghq-style path (github.com/user/repo/main)
template = "wt-{branch}"

# Custom character replacements for branch names
# Default: { "/" = "-" }
# sanitize_chars = { "/" = "_", ":" = "-" }

[ui]
# Show icons in output (requires NerdFont)
# Default: true
icons = true

# Display ~ instead of full home path
# Default: true
tilde_home = true

# Color theme: "default" (256-color/True Color) or "classic" (8-bit 16-color)
# Default: "default"
theme = "default"

# Custom color overrides (optional)
# Supports: hex (#RRGGBB, #RGB), named colors (red, green, etc.), 256-color index (0-255)
# [ui.colors]
# header = "#06B6D4"
# selected = "yellow"
# branch = "34"

# Global copy/setup settings (applies to all repositories)
# Files to copy from main worktree after creating
copy_files = [".env", ".claude"]

# Commands to run after creating a worktree
# Available variables: $WORKTREE_NAME, $WORKTREE_PATH, $WORKTREE_BRANCH
setup_commands = ["npm install"]

# Per-repository settings (overrides global settings when matched)
[[repository_settings]]
repository = "~/src/my-project"
copy_files = [".env", ".env.local", "secrets.json"]
setup_commands = [
    "npm install",
    "cp ../.env .env"
]
```

### Parameters

#### [worktree]

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `basedir` | string | `"~/worktrees"` | Base directory for new worktrees. Supports absolute, `~`, and relative paths |
| `auto_mkdir` | bool | `true` | Automatically create base directory if it doesn't exist |

**Path Examples:**

| basedir | Result (repo at `/home/user/myrepo`) |
|---------|--------------------------------------|
| `~/worktrees` | `/home/user/worktrees` |
| `/opt/worktrees` | `/opt/worktrees` |
| `.git/wt` | `/home/user/myrepo/.git/wt` |
| `../worktrees` | `/home/user/myrepo/../worktrees` |

#### [naming]

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `template` | string | - | Directory naming template with variables |
| `sanitize_chars` | map | `{ "/" = "-" }` | Character replacements for branch names |

**Template Variables:**

| Variable | Description | Example |
|----------|-------------|---------|
| `{branch}` | Branch name (sanitized) | `feature-login` |
| `{host}` | Git host from origin URL | `github.com` |
| `{owner}` | Repository owner | `username` |
| `{repository}` | Repository name | `myproject` |

**Template Examples:**

| Template | Result |
|----------|--------|
| `wt-{branch}` | `wt-feature-login` |
| `{branch}-dev` | `main-dev` |
| `{host}/{owner}/{repository}/{branch}` | `github.com/user/repo/main` |
| `{owner}-{repository}-{branch}` | `user-repo-feature-login` |

#### [ui]

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `icons` | bool | `true` | Show NerdFont icons in output |
| `tilde_home` | bool | `true` | Display ~ instead of full home path |
| `theme` | string | `"default"` | Color theme: `default` (256-color) or `classic` (8-bit) |

#### [ui.colors]

Custom color overrides. All fields are optional.

| Parameter | Description |
|-----------|-------------|
| `header` | Header text color |
| `selected` | Selected item color |
| `branch` | Branch name color |
| `remote` | Remote branch color |
| `main_worktree` | Main worktree indicator color |
| `key` | Keybinding color |
| `description` | Description text color |

**Color formats:**
- Hex: `#RRGGBB` or `#RGB` (e.g., `"#06B6D4"`, `"#F00"`)
- Named: `red`, `green`, `blue`, `cyan`, `magenta`, `yellow`, `white`, `black`, `gray`, `darkgray`
- 256-color index: `0` to `255` (e.g., `"34"`)

#### Global copy_files / setup_commands

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `copy_files` | string[] | - | Files to copy from main worktree (applies to all repositories) |
| `setup_commands` | string[] | - | Commands to run after creating (applies to all repositories) |

**Priority (high to low):**
1. Local `.gwm.toml` top-level settings
2. `[[repository_settings]]` (if repository path matches)
3. Global config (`~/.config/gwm/config.toml`) top-level settings

Local settings completely replace global settings (not merged).

#### [[repository_settings]]

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `repository` | string | - | Repository path (used as key for matching) |
| `copy_files` | string[] | - | Files to copy (overrides top-level setting) |
| `setup_commands` | string[] | - | Commands to run (overrides top-level setting) |

### copy_files Patterns

`copy_files` supports single files, directories, and glob patterns:

```toml
copy_files = [
    ".env",                # Single file
    ".env.local",          # Single file
    ".claude",             # Directory (copied recursively)
    "config/database.yml", # Nested file
    ".env*",               # Glob pattern (matches .env, .env.local, .env.test, etc.)
    "secrets/*.json",      # Glob in subdirectory
]
```

| Pattern | Description |
|---------|-------------|
| `.env` | Copy single file |
| `.claude` | Copy directory recursively |
| `.env*` | Glob: all files starting with `.env` |
| `config/*.yml` | Glob: all `.yml` files in `config/` |
| `**/*.json` | Glob: all `.json` files recursively |

Files that don't exist in the main worktree are silently skipped.

### Setup Command Variables

| Variable | Description |
|----------|-------------|
| `$WORKTREE_NAME` | Name of the worktree |
| `$WORKTREE_PATH` | Absolute path to the worktree |
| `$WORKTREE_BRANCH` | Branch name of the worktree |

## Requirements

- Rust 1.70+
- Git
- Terminal with NerdFont (optional, for icons)

## License

MIT
