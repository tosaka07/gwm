# gwm - Git Worktree Manager

A terminal user interface (TUI) for managing Git worktrees.

## Features

- List and navigate worktrees with vim-style keybindings
- Create new worktrees from any branch
- Delete worktrees with confirmation
- Open shell in selected worktree directory
- Customizable keybindings (Vim + Emacs style)
- Lifecycle hooks (pre/post create/delete)
- Configuration layering (global + local)

## Installation

```bash
cargo install --path .
```

## Usage

Run `gwm` in a git repository:

```bash
gwm
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `Ctrl+n` / `↓` | Move down |
| `k` / `Ctrl+p` / `↑` | Move up |
| `g` / `Home` | Move to top |
| `G` / `End` | Move to bottom |
| `Ctrl+d` / `PageDown` | Page down |
| `Ctrl+u` / `PageUp` | Page up |

### Actions

| Key | Action |
|-----|--------|
| `Enter` | Open shell in worktree |
| `c` | Create new worktree |
| `d` | Delete worktree |
| `D` | Delete merged worktrees |
| `r` | Rebase worktree |
| `R` | Refresh list |
| `/` | Search |
| `?` | Toggle help |
| `q` | Quit |

## Configuration

Configuration files are loaded from:
- Global: `~/.config/gwm/config.toml`
- Local: `.gwm/config.toml` (in repository root)

Local config overrides global config.

### Example Configuration

```toml
[worktree]
# Directory template for new worktrees
# {name} = branch name, {repo} = repository name
base_dir = "../worktrees/{name}"

# Files to copy when creating a new worktree
copy_files = [".env", "node_modules"]

# Custom keybindings
[[bindings]]
key = "i"
command = "pnpm install"

# Lifecycle hooks
[[hooks]]
event = "post_create"
command = "pnpm install"
```

## License

MIT
