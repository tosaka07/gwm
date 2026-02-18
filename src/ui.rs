use crate::app::{App, AppMode, ConfirmAction};
use crate::theme::ThemeColors;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph},
    Frame,
};

/// Branch icon (NerdFont)
const BRANCH_ICON: &str = "\u{e725}";

/// Spinner animation frames (braille pattern)
const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Format branch display with optional icon
fn format_branch_with_icon(branch: &str, icons_enabled: bool) -> String {
    if icons_enabled {
        format!("{} {}", BRANCH_ICON, branch)
    } else {
        branch.to_string()
    }
}

/// Get the inner area with 1 character margin on all sides
fn inner_area(frame: &Frame) -> Rect {
    frame.area().inner(Margin {
        vertical: 1,
        horizontal: 1,
    })
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = inner_area(frame);
    let colors = app.theme.colors.clone();
    match app.mode {
        AppMode::Normal => draw_normal_mode(frame, app, area, &colors),
        AppMode::Create => draw_create_mode(frame, app, area, &colors),
        AppMode::Confirm => {
            draw_normal_mode(frame, app, area, &colors);
            draw_confirm_dialog(frame, app, &colors);
        }
        AppMode::Deleting => {
            draw_normal_mode(frame, app, area, &colors);
            draw_deleting_dialog(frame, app, &colors);
        }
        AppMode::Config => {
            draw_normal_mode(frame, app, area, &colors);
            draw_config_dialog(frame, app, &colors);
        }
    }
}

fn draw_normal_mode(frame: &mut Frame, app: &App, area: Rect, colors: &ThemeColors) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header with search
            Constraint::Length(1), // Spacer
            Constraint::Min(3),    // Main content (list + detail)
            Constraint::Length(1), // Footer/Status
        ])
        .split(area);

    // Header with search
    let header = if app.input.is_empty() {
        Paragraph::new(Line::from(vec![
            Span::styled(
                "gwm",
                Style::default()
                    .fg(colors.header)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(colors.separator)),
            Span::styled("Search", Style::default().fg(colors.text_muted)),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(
                "gwm",
                Style::default()
                    .fg(colors.header)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(colors.separator)),
            Span::styled(&app.input, Style::default().fg(colors.text)),
        ]))
    };
    frame.render_widget(header, chunks[0]);

    // Show cursor at search position
    let cursor_x = chunks[0].x + 6 + app.input.len() as u16; // "gwm │ " = 6 chars
    frame.set_cursor_position((cursor_x, chunks[0].y));

    // Split main content into left (list) and right (detail)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Worktree list
            Constraint::Percentage(60), // Detail pane
        ])
        .split(chunks[2]);

    // Worktree list (use filtered_worktrees)
    let icons_enabled = app.icons_enabled();
    let items: Vec<ListItem> = app
        .filtered_worktrees
        .iter()
        .enumerate()
        .map(|(i, wt)| {
            let is_selected = i == app.selected_worktree;
            let prefix = if is_selected { "▶ " } else { "  " };

            let name_style = if is_selected {
                Style::default()
                    .fg(colors.selected)
                    .add_modifier(Modifier::BOLD)
            } else if wt.is_main {
                Style::default().fg(colors.main_worktree)
            } else {
                Style::default()
            };

            // Hide branch name if it matches worktree name
            let branch_display = wt.branch.as_ref().filter(|b| *b != &wt.name);

            let mut spans = vec![
                Span::styled(
                    prefix,
                    if is_selected {
                        Style::default().fg(colors.selected)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(&wt.name, name_style),
            ];

            // Add separator and branch only if branch is different from worktree name
            if let Some(branch) = branch_display {
                spans.push(Span::styled(" | ", Style::default().fg(colors.separator)));
                spans.push(Span::styled(
                    format_branch_with_icon(branch, icons_enabled),
                    Style::default()
                        .fg(colors.branch)
                        .add_modifier(Modifier::DIM),
                ));
            }

            if wt.is_main {
                spans.push(Span::styled(
                    " [main]",
                    Style::default()
                        .fg(colors.main_worktree)
                        .add_modifier(Modifier::DIM),
                ));
            }

            let content = Line::from(spans);

            ListItem::new(content)
        })
        .collect();

    let title = if app.input.is_empty() {
        "Worktrees".to_string()
    } else {
        format!(
            "Worktrees ({}/{})",
            app.filtered_worktrees.len(),
            app.worktrees.len()
        )
    };
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .padding(Padding::horizontal(1)),
    );
    frame.render_widget(list, main_chunks[0]);

    // Detail pane
    draw_detail_pane(frame, app, main_chunks[1], colors);

    // Footer
    if let Some(msg) = &app.message {
        let footer = Paragraph::new(msg.as_str()).style(Style::default().fg(colors.success));
        frame.render_widget(footer, chunks[3]);
    } else {
        let footer = render_normal_footer(colors);
        frame.render_widget(footer, chunks[3]);
    }
}

fn draw_detail_pane(frame: &mut Frame, app: &App, area: Rect, colors: &ThemeColors) {
    let detail = app.get_selected_worktree_detail();
    let icons_enabled = app.icons_enabled();

    let mut lines: Vec<Line> = Vec::new();

    if let Some(detail) = detail {
        // Branch
        let branch_name = detail.branch.as_deref().unwrap_or("(detached)").to_string();
        let icon_span = if icons_enabled {
            Span::styled(
                format!("{} ", BRANCH_ICON),
                Style::default().fg(colors.branch),
            )
        } else {
            Span::raw("")
        };
        lines.push(Line::from(vec![
            Span::styled("Branch: ", Style::default().fg(colors.text_muted)),
            icon_span,
            Span::styled(
                branch_name,
                Style::default()
                    .fg(colors.branch)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Path
        let display_path = app.format_path(&detail.path);
        lines.push(Line::from(vec![
            Span::styled("Path:   ", Style::default().fg(colors.text_muted)),
            Span::styled(display_path, Style::default().fg(colors.text)),
        ]));

        lines.push(Line::from(""));

        // Changed files
        lines.push(Line::from(vec![Span::styled(
            "Changed Files",
            Style::default()
                .fg(colors.text_muted)
                .add_modifier(Modifier::UNDERLINED),
        )]));

        if detail.changed_files.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  (clean)",
                Style::default().fg(colors.text_muted),
            )]));
        } else {
            let summary = &detail.changed_files;
            lines.push(Line::from(vec![
                Span::styled("  +", Style::default().fg(colors.success)),
                Span::styled(
                    format!("{} ", summary.added),
                    Style::default().fg(colors.text),
                ),
                Span::styled("-", Style::default().fg(colors.error)),
                Span::styled(
                    format!("{} ", summary.deleted),
                    Style::default().fg(colors.text),
                ),
                Span::styled("~", Style::default().fg(colors.warning)),
                Span::styled(
                    format!("{}", summary.modified),
                    Style::default().fg(colors.text),
                ),
            ]));
        }

        lines.push(Line::from(""));

        // Recent commits
        lines.push(Line::from(vec![Span::styled(
            "Recent Commits",
            Style::default()
                .fg(colors.text_muted)
                .add_modifier(Modifier::UNDERLINED),
        )]));

        if detail.recent_commits.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  (no commits)",
                Style::default().fg(colors.text_muted),
            )]));
        } else {
            for commit in detail.recent_commits {
                let graph_char = if commit.is_merge { "●" } else { "○" };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", graph_char),
                        Style::default().fg(colors.header),
                    ),
                    Span::styled(commit.short_id, Style::default().fg(colors.warning)),
                    Span::styled(" ", Style::default()),
                    Span::styled(commit.message, Style::default().fg(colors.text)),
                ]));
            }
        }
    } else {
        lines.push(Line::from(vec![Span::styled(
            "No worktree selected",
            Style::default().fg(colors.text_muted),
        )]));
    }

    let detail_widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Details")
            .padding(Padding::horizontal(1)),
    );
    frame.render_widget(detail_widget, area);
}

fn draw_create_mode(frame: &mut Frame, app: &App, area: Rect, colors: &ThemeColors) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Input field
            Constraint::Min(3),    // Branch list
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "gwm",
            Style::default()
                .fg(colors.header)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" - Create Worktree", Style::default().fg(colors.text_muted)),
    ]));
    frame.render_widget(header, chunks[0]);

    // Input field - title changes based on selection
    let input_title = if app.selected_branch == 0 {
        // "Create new branch" is selected
        "New branch name"
    } else if app.input.is_empty() {
        "Worktree name (empty = branch name)"
    } else {
        "Worktree name"
    };
    let input = Paragraph::new(app.input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(input_title)
            .padding(Padding::horizontal(1)),
    );
    frame.render_widget(input, chunks[2]);

    // Show cursor in input field (border + padding = 2)
    frame.set_cursor_position((chunks[2].x + app.input.len() as u16 + 2, chunks[2].y + 1));

    // Branch list - start with "Create new branch" option
    let icons_enabled = app.icons_enabled();
    let mut items: Vec<ListItem> = Vec::new();

    // Add "Create new branch" option at index 0
    let is_create_new_selected = app.selected_branch == 0;
    let create_new_prefix = if is_create_new_selected { "▶ " } else { "  " };
    let create_new_style = if is_create_new_selected {
        Style::default()
            .fg(colors.selected)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(colors.header)
    };
    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            create_new_prefix,
            if is_create_new_selected {
                Style::default().fg(colors.selected)
            } else {
                Style::default()
            },
        ),
        Span::styled("(Create new branch)", create_new_style),
    ])));

    // Add existing branches (index 1+)
    for (i, branch) in app.filtered_branches.iter().enumerate() {
        let is_selected = (i + 1) == app.selected_branch;
        let prefix = if is_selected { "▶ " } else { "  " };

        let name_style = if is_selected {
            Style::default()
                .fg(colors.selected)
                .add_modifier(Modifier::BOLD)
        } else if branch.is_remote {
            Style::default().fg(colors.remote)
        } else {
            Style::default().fg(colors.branch)
        };

        let icon_prefix = if icons_enabled {
            format!("{} ", BRANCH_ICON)
        } else {
            String::new()
        };

        let content = Line::from(vec![
            Span::styled(
                prefix,
                if is_selected {
                    Style::default().fg(colors.selected)
                } else {
                    Style::default()
                },
            ),
            Span::styled(icon_prefix, name_style),
            Span::styled(&branch.name, name_style),
            if branch.is_head {
                Span::styled(" *", Style::default().fg(colors.warning))
            } else {
                Span::raw("")
            },
            if branch.is_remote {
                Span::styled(
                    " [remote]",
                    Style::default()
                        .fg(colors.remote)
                        .add_modifier(Modifier::DIM),
                )
            } else {
                Span::raw("")
            },
        ]);

        items.push(ListItem::new(content));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Branches")
            .padding(Padding::horizontal(1)),
    );
    frame.render_widget(list, chunks[3]);

    // Footer
    if let Some(msg) = &app.message {
        let footer = Paragraph::new(msg.as_str()).style(Style::default().fg(colors.success));
        frame.render_widget(footer, chunks[4]);
    } else {
        let footer = render_create_footer(colors);
        frame.render_widget(footer, chunks[4]);
    }
}

fn render_normal_footer(colors: &ThemeColors) -> Paragraph<'static> {
    Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(colors.key)),
        Span::styled(": move  ", Style::default().fg(colors.description)),
        Span::styled("Enter", Style::default().fg(colors.key)),
        Span::styled(": open  ", Style::default().fg(colors.description)),
        Span::styled("C-o", Style::default().fg(colors.key)),
        Span::styled(": create  ", Style::default().fg(colors.description)),
        Span::styled("C-d", Style::default().fg(colors.key)),
        Span::styled(": delete  ", Style::default().fg(colors.description)),
        Span::styled("D", Style::default().fg(colors.key)),
        Span::styled(": prune  ", Style::default().fg(colors.description)),
        Span::styled("?", Style::default().fg(colors.key)),
        Span::styled(": config  ", Style::default().fg(colors.description)),
        Span::styled("C-q", Style::default().fg(colors.key)),
        Span::styled(": quit", Style::default().fg(colors.description)),
    ]))
}

fn render_create_footer(colors: &ThemeColors) -> Paragraph<'static> {
    Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(colors.key)),
        Span::styled(": move  ", Style::default().fg(colors.description)),
        Span::styled("Enter", Style::default().fg(colors.key)),
        Span::styled(": create  ", Style::default().fg(colors.description)),
        Span::styled("Esc", Style::default().fg(colors.key)),
        Span::styled("/", Style::default().fg(colors.description)),
        Span::styled("C-c", Style::default().fg(colors.key)),
        Span::styled(": cancel", Style::default().fg(colors.description)),
    ]))
}

fn draw_confirm_dialog(frame: &mut Frame, app: &App, colors: &ThemeColors) {
    let area = centered_rect(60, 30, frame.area());
    let clear_area = expand_area(area, frame.area());

    let message = match app.confirm_action {
        Some(ConfirmAction::DeleteSingle) => {
            let wt = &app.filtered_worktrees[app.selected_worktree];
            format!("Delete worktree '{}'?", wt.name)
        }
        Some(ConfirmAction::Prune) => {
            let names: Vec<_> = app
                .merged_worktrees
                .iter()
                .map(|w| w.name.as_str())
                .collect();
            format!(
                "Prune {} merged worktree(s)?\n\n{}",
                names.len(),
                names.join(", ")
            )
        }
        None => String::new(),
    };

    let shortcut_line = Line::from(vec![
        Span::styled(" y", Style::default().fg(colors.key)),
        Span::styled(": worktree ", Style::default().fg(colors.description)),
        Span::styled("Y", Style::default().fg(colors.key)),
        Span::styled(
            ": worktree & branch ",
            Style::default().fg(colors.description),
        ),
        Span::styled("n", Style::default().fg(colors.key)),
        Span::styled("/", Style::default().fg(colors.description)),
        Span::styled("Esc", Style::default().fg(colors.key)),
        Span::styled(": cancel ", Style::default().fg(colors.description)),
    ]);

    let lines: Vec<Line> = message
        .lines()
        .map(|l| {
            Line::from(Span::styled(
                l.to_string(),
                Style::default().fg(colors.text),
            ))
        })
        .collect();

    let dialog = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Confirm")
            .title_bottom(shortcut_line)
            .style(Style::default().fg(colors.warning))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(Clear, clear_area);
    frame.render_widget(dialog, area);
}

fn draw_deleting_dialog(frame: &mut Frame, app: &App, colors: &ThemeColors) {
    let area = centered_rect(50, 20, frame.area());
    let clear_area = expand_area(area, frame.area());

    let spinner = SPINNER_FRAMES[(app.tick as usize) % SPINNER_FRAMES.len()];
    let message = format!(
        "{} {}",
        spinner,
        app.deleting_message.as_deref().unwrap_or("Deleting...")
    );

    let wait_hint = Line::from(vec![Span::styled(
        " Please wait... ",
        Style::default().fg(colors.text_muted),
    )]);

    let dialog = Paragraph::new(Line::from(vec![Span::styled(
        message,
        Style::default().fg(colors.warning),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Processing")
            .title_bottom(wait_hint)
            .style(Style::default().fg(colors.warning))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(Clear, clear_area);
    frame.render_widget(dialog, area);
}

fn draw_config_dialog(frame: &mut Frame, app: &mut App, colors: &ThemeColors) {
    let area = centered_rect(70, 80, frame.area());
    let clear_area = expand_area(area, frame.area());

    let sources = &app.config_sources;
    let mut lines: Vec<Line> = Vec::new();

    // --- Global section ---
    lines.push(Line::from(vec![Span::styled(
        "Global",
        Style::default()
            .fg(colors.header)
            .add_modifier(Modifier::BOLD),
    )]));
    match &sources.global.path {
        Some(path) => {
            lines.push(Line::from(vec![Span::styled(
                format!("  {}", app.format_path(&path.to_string_lossy())),
                Style::default().fg(colors.text_muted),
            )]));
            let entries = config_entries(&sources.global.config);
            if entries.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "  (no settings)",
                    Style::default().fg(colors.text_muted),
                )]));
            } else {
                for (key, value) in &entries {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} = ", key), Style::default().fg(colors.text)),
                        Span::styled(value.clone(), Style::default().fg(colors.branch)),
                    ]));
                }
            }
        }
        None => {
            lines.push(Line::from(vec![Span::styled(
                "  (not found)",
                Style::default().fg(colors.text_muted),
            )]));
        }
    }

    lines.push(Line::from(""));

    // --- Local section ---
    lines.push(Line::from(vec![Span::styled(
        "Local",
        Style::default()
            .fg(colors.header)
            .add_modifier(Modifier::BOLD),
    )]));
    match &sources.local.path {
        Some(path) => {
            lines.push(Line::from(vec![Span::styled(
                format!("  {}", app.format_path(&path.to_string_lossy())),
                Style::default().fg(colors.text_muted),
            )]));
            let entries = config_entries(&sources.local.config);
            if entries.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "  (no settings)",
                    Style::default().fg(colors.text_muted),
                )]));
            } else {
                for (key, value) in &entries {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} = ", key), Style::default().fg(colors.text)),
                        Span::styled(value.clone(), Style::default().fg(colors.branch)),
                    ]));
                }
            }
        }
        None => {
            lines.push(Line::from(vec![Span::styled(
                "  (not found)",
                Style::default().fg(colors.text_muted),
            )]));
        }
    }

    lines.push(Line::from(""));

    // --- Effective section ---
    lines.push(Line::from(vec![Span::styled(
        "Effective",
        Style::default()
            .fg(colors.header)
            .add_modifier(Modifier::BOLD),
    )]));

    let effective_entries = effective_config_entries(sources);
    for (key, value, source) in &effective_entries {
        let source_style = match source.as_str() {
            "global" | "local" | "env" => Style::default().fg(colors.warning),
            _ => Style::default().fg(colors.text_muted),
        };
        let source_label = match source.as_str() {
            "global" => " ← global",
            "local" => " ← local",
            "env" => " ← env",
            _ => " (default)",
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<22}= ", key), Style::default().fg(colors.text)),
            Span::styled(format!("{:<20}", value), Style::default().fg(colors.branch)),
            Span::styled(source_label, source_style),
        ]));
    }

    // Calculate scroll max: content lines - visible lines inside the dialog
    // area.height minus 2 (top/bottom border) is the visible content height
    let visible_height = area.height.saturating_sub(2);
    let content_height = lines.len() as u16;
    app.config_scroll_max = content_height.saturating_sub(visible_height);
    // Clamp current scroll position
    if app.config_scroll > app.config_scroll_max {
        app.config_scroll = app.config_scroll_max;
    }

    let close_hint = Line::from(vec![
        Span::styled(" ↑↓", Style::default().fg(colors.key)),
        Span::styled(": scroll  ", Style::default().fg(colors.description)),
        Span::styled("Esc", Style::default().fg(colors.key)),
        Span::styled("/", Style::default().fg(colors.description)),
        Span::styled("Enter", Style::default().fg(colors.key)),
        Span::styled("/", Style::default().fg(colors.description)),
        Span::styled("q", Style::default().fg(colors.key)),
        Span::styled(": close ", Style::default().fg(colors.description)),
    ]);

    let dialog = Paragraph::new(lines).scroll((app.config_scroll, 0)).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Configuration")
            .title_bottom(close_hint)
            .style(Style::default().fg(colors.header))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(Clear, clear_area);
    frame.render_widget(dialog, area);
}

/// Extract explicitly set config entries as (key, value) pairs
fn config_entries(config: &crate::config::Config) -> Vec<(&'static str, String)> {
    let mut entries = Vec::new();
    if let Some(ref v) = config.worktree.basedir {
        entries.push(("worktree.basedir", format!("\"{}\"", v)));
    }
    if let Some(v) = config.worktree.auto_mkdir {
        entries.push(("worktree.auto_mkdir", v.to_string()));
    }
    if let Some(ref v) = config.naming.template {
        entries.push(("naming.template", format!("\"{}\"", v)));
    }
    if config.naming.sanitize_chars.is_some() {
        entries.push(("naming.sanitize_chars", "(custom)".to_string()));
    }
    if let Some(v) = config.ui.icons {
        entries.push(("ui.icons", v.to_string()));
    }
    if let Some(v) = config.ui.tilde_home {
        entries.push(("ui.tilde_home", v.to_string()));
    }
    if let Some(ref v) = config.ui.theme {
        entries.push(("ui.theme", format!("\"{}\"", v)));
    }
    if config.ui.colors.is_some() {
        entries.push(("ui.colors", "(custom)".to_string()));
    }
    if let Some(ref v) = config.copy_files {
        entries.push(("copy_files", format!("{:?}", v)));
    }
    if let Some(ref v) = config.setup_commands {
        entries.push(("setup_commands", format!("{:?}", v)));
    }
    entries
}

/// Build effective config entries with source information
fn effective_config_entries(
    sources: &crate::config::ConfigSources,
) -> Vec<(&'static str, String, String)> {
    let global = &sources.global.config;
    let local = &sources.local.config;
    let env = &sources.env;

    let mut entries = Vec::new();

    // worktree.basedir
    let (val, src) = resolve_source_str(
        &env.worktree.basedir,
        &local.worktree.basedir,
        &global.worktree.basedir,
        "~/worktrees",
    );
    entries.push(("worktree.basedir", val, src));

    // worktree.auto_mkdir
    let (val, src) = resolve_source_bool(
        &env.worktree.auto_mkdir,
        &local.worktree.auto_mkdir,
        &global.worktree.auto_mkdir,
        true,
    );
    entries.push(("worktree.auto_mkdir", val, src));

    // naming.template
    let (val, src) = resolve_source_opt_str(
        &env.naming.template,
        &local.naming.template,
        &global.naming.template,
    );
    entries.push(("naming.template", val, src));

    // ui.icons
    let (val, src) = resolve_source_bool(&env.ui.icons, &local.ui.icons, &global.ui.icons, true);
    entries.push(("ui.icons", val, src));

    // ui.tilde_home
    let (val, src) = resolve_source_bool(
        &env.ui.tilde_home,
        &local.ui.tilde_home,
        &global.ui.tilde_home,
        true,
    );
    entries.push(("ui.tilde_home", val, src));

    // ui.theme
    let (val, src) =
        resolve_source_str(&env.ui.theme, &local.ui.theme, &global.ui.theme, "default");
    entries.push(("ui.theme", val, src));

    entries
}

fn resolve_source_str(
    env: &Option<String>,
    local: &Option<String>,
    global: &Option<String>,
    default: &str,
) -> (String, String) {
    if let Some(v) = env {
        (v.clone(), "env".to_string())
    } else if let Some(v) = local {
        (v.clone(), "local".to_string())
    } else if let Some(v) = global {
        (v.clone(), "global".to_string())
    } else {
        (default.to_string(), "default".to_string())
    }
}

fn resolve_source_opt_str(
    env: &Option<String>,
    local: &Option<String>,
    global: &Option<String>,
) -> (String, String) {
    if let Some(v) = env {
        (v.clone(), "env".to_string())
    } else if let Some(v) = local {
        (v.clone(), "local".to_string())
    } else if let Some(v) = global {
        (v.clone(), "global".to_string())
    } else {
        ("(none)".to_string(), "default".to_string())
    }
}

fn resolve_source_bool(
    env: &Option<bool>,
    local: &Option<bool>,
    global: &Option<bool>,
    default: bool,
) -> (String, String) {
    if let Some(v) = env {
        (v.to_string(), "env".to_string())
    } else if let Some(v) = local {
        (v.to_string(), "local".to_string())
    } else if let Some(v) = global {
        (v.to_string(), "global".to_string())
    } else {
        (default.to_string(), "default".to_string())
    }
}

/// Expand a Rect by 1 cell on each side, clamped to the given bounds
fn expand_area(area: Rect, bounds: Rect) -> Rect {
    let x = area.x.saturating_sub(1).max(bounds.x);
    let y = area.y.saturating_sub(1).max(bounds.y);
    let right = (area.x + area.width + 1).min(bounds.x + bounds.width);
    let bottom = (area.y + area.height + 1).min(bounds.y + bounds.height);
    Rect {
        x,
        y,
        width: right.saturating_sub(x),
        height: bottom.saturating_sub(y),
    }
}

/// Create a centered rectangle with given percentage width and height
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_branch_with_icon_enabled() {
        let result = format_branch_with_icon("main", true);
        assert!(result.contains(BRANCH_ICON));
        assert!(result.contains("main"));
        assert_eq!(result, format!("{} main", BRANCH_ICON));
    }

    #[test]
    fn test_format_branch_with_icon_disabled() {
        let result = format_branch_with_icon("main", false);
        assert!(!result.contains(BRANCH_ICON));
        assert_eq!(result, "main");
    }

    #[test]
    fn test_format_branch_with_icon_special_chars() {
        let result = format_branch_with_icon("feature/test-123", true);
        assert_eq!(result, format!("{} feature/test-123", BRANCH_ICON));

        let result = format_branch_with_icon("feature/test-123", false);
        assert_eq!(result, "feature/test-123");
    }

    // ========== config_entries tests ==========

    #[test]
    fn test_config_entries_default_returns_empty() {
        let config = crate::config::Config::default();
        let entries = config_entries(&config);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_config_entries_with_values() {
        let mut config = crate::config::Config::default();
        config.worktree.basedir = Some("~/wt".to_string());
        config.ui.icons = Some(false);
        config.ui.theme = Some("classic".to_string());

        let entries = config_entries(&config);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0], ("worktree.basedir", "\"~/wt\"".to_string()));
        assert_eq!(entries[1], ("ui.icons", "false".to_string()));
        assert_eq!(entries[2], ("ui.theme", "\"classic\"".to_string()));
    }

    // ========== resolve_source tests ==========

    #[test]
    fn test_resolve_source_str_env_wins() {
        let (val, src) = resolve_source_str(
            &Some("env_val".to_string()),
            &Some("local_val".to_string()),
            &Some("global_val".to_string()),
            "default",
        );
        assert_eq!(val, "env_val");
        assert_eq!(src, "env");
    }

    #[test]
    fn test_resolve_source_str_local_wins() {
        let (val, src) = resolve_source_str(
            &None,
            &Some("local_val".to_string()),
            &Some("global_val".to_string()),
            "default",
        );
        assert_eq!(val, "local_val");
        assert_eq!(src, "local");
    }

    #[test]
    fn test_resolve_source_str_global_wins() {
        let (val, src) =
            resolve_source_str(&None, &None, &Some("global_val".to_string()), "default");
        assert_eq!(val, "global_val");
        assert_eq!(src, "global");
    }

    #[test]
    fn test_resolve_source_str_default() {
        let (val, src) = resolve_source_str(&None, &None, &None, "fallback");
        assert_eq!(val, "fallback");
        assert_eq!(src, "default");
    }

    #[test]
    fn test_resolve_source_bool_priority() {
        let (val, src) = resolve_source_bool(&Some(true), &Some(false), &None, false);
        assert_eq!(val, "true");
        assert_eq!(src, "env");

        let (val, src) = resolve_source_bool(&None, &Some(false), &Some(true), true);
        assert_eq!(val, "false");
        assert_eq!(src, "local");

        let (val, src) = resolve_source_bool(&None, &None, &Some(true), false);
        assert_eq!(val, "true");
        assert_eq!(src, "global");

        let (val, src) = resolve_source_bool(&None, &None, &None, true);
        assert_eq!(val, "true");
        assert_eq!(src, "default");
    }

    #[test]
    fn test_resolve_source_opt_str_none_default() {
        let (val, src) = resolve_source_opt_str(&None, &None, &None);
        assert_eq!(val, "(none)");
        assert_eq!(src, "default");
    }

    #[test]
    fn test_resolve_source_opt_str_with_value() {
        let (val, src) = resolve_source_opt_str(&None, &Some("tmpl".to_string()), &None);
        assert_eq!(val, "tmpl");
        assert_eq!(src, "local");
    }

    // ========== effective_config_entries tests ==========

    #[test]
    fn test_effective_config_entries_all_defaults() {
        let sources = crate::config::ConfigSources::default();
        let entries = effective_config_entries(&sources);

        assert_eq!(entries.len(), 6);
        // All should be "default" source
        for (_key, _val, src) in &entries {
            assert_eq!(src, "default");
        }
    }

    #[test]
    fn test_effective_config_entries_mixed_sources() {
        let mut sources = crate::config::ConfigSources::default();
        sources.global.config.worktree.basedir = Some("~/global-wt".to_string());
        sources.local.config.worktree.basedir = Some("~/local-wt".to_string());
        sources.local.config.ui.icons = Some(false);

        let entries = effective_config_entries(&sources);

        // worktree.basedir: local wins over global
        let basedir = entries
            .iter()
            .find(|(k, _, _)| *k == "worktree.basedir")
            .unwrap();
        assert_eq!(basedir.1, "~/local-wt");
        assert_eq!(basedir.2, "local");

        // ui.icons: local
        let icons = entries.iter().find(|(k, _, _)| *k == "ui.icons").unwrap();
        assert_eq!(icons.1, "false");
        assert_eq!(icons.2, "local");

        // ui.theme: default
        let theme = entries.iter().find(|(k, _, _)| *k == "ui.theme").unwrap();
        assert_eq!(theme.2, "default");
    }
}
