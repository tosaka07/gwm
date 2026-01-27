# gwm - Git Worktree Manager

A fast and intuitive TUI application for managing git worktrees.

## Screenshot

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌─ Worktrees ────────────────────────┐ ┌─ Details ───────────────────────┐ │
│  │                                    │ │                                 │ │
│  │  ▶ gwm                    [main]   │ │ Branch:  main                 │ │
│  │    feature-auth   feature/auth   │ │ Path:   /Users/dev/gwm         │ │
│  │    bugfix-123                      │ │                                 │ │
│  │                                    │ │ Changed Files                   │ │
│  │                                    │ │   (clean)                       │ │
│  │                                    │ │                                 │ │
│  │                                    │ │ Recent Commits                  │ │
│  │                                    │ │   abc1234 Initial commit        │ │
│  │                                    │ │   def5678 Add feature           │ │
│  │                                    │ │                                 │ │
│  └────────────────────────────────────┘ └─────────────────────────────────┘ │
│                                                                             │
│  Enter: open  C-o: create  C-d: delete  ?: help  C-q: quit                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Features

- Browse and switch between worktrees
- Create new worktrees from existing or new branches
- Delete worktrees (with optional branch deletion)
- Prune merged worktrees
- Fuzzy search/filter worktrees
- NerdFont icons support

## Installation

### Homebrew (macOS/Linux)

```bash
brew install tosaka07/tap/gwm
```

### Cargo

```bash
cargo install --git https://github.com/tosaka07/gwm
```

## Usage

```bash
# Run in any git repository
gwm
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

# Per-repository settings
[[repository_settings]]
repository = "~/src/my-project"

# Files to copy from main worktree after creating
copy_files = [".env", ".env.local"]

# Commands to run after creating a worktree
# Available variables: $WORKTREE_NAME, $WORKTREE_PATH, $WORKTREE_BRANCH
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

#### [[repository_settings]]

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `repository` | string | - | Repository path (used as key for matching) |
| `copy_files` | string[] | - | Files to copy from main worktree to new worktree |
| `setup_commands` | string[] | - | Commands to run after creating a worktree |

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
