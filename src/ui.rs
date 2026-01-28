use crate::app::{App, AppMode, ConfirmAction};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph},
    Frame,
};

const HEADER_COLOR: Color = Color::Cyan;
const SELECTED_COLOR: Color = Color::Yellow;
const BRANCH_COLOR: Color = Color::Green;
const REMOTE_COLOR: Color = Color::Magenta;
const MAIN_WORKTREE_COLOR: Color = Color::Blue;
const KEY_COLOR: Color = Color::Yellow;
const DESC_COLOR: Color = Color::DarkGray;

/// Branch icon (NerdFont)
const BRANCH_ICON: &str = "\u{e725}";

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

pub fn draw(frame: &mut Frame, app: &App) {
    let area = inner_area(frame);
    match app.mode {
        AppMode::Normal => draw_normal_mode(frame, app, area),
        AppMode::Create => draw_create_mode(frame, app, area),
        AppMode::Confirm => {
            draw_normal_mode(frame, app, area);
            draw_confirm_dialog(frame, app);
        }
        AppMode::Help => {
            draw_normal_mode(frame, app, area);
            draw_help_dialog(frame);
        }
    }
}

fn draw_normal_mode(frame: &mut Frame, app: &App, area: Rect) {
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
                    .fg(HEADER_COLOR)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled("Search", Style::default().fg(Color::DarkGray)),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(
                "gwm",
                Style::default()
                    .fg(HEADER_COLOR)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.input, Style::default().fg(Color::White)),
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
                    .fg(SELECTED_COLOR)
                    .add_modifier(Modifier::BOLD)
            } else if wt.is_main {
                Style::default().fg(MAIN_WORKTREE_COLOR)
            } else {
                Style::default()
            };

            // Hide branch name if it matches worktree name
            let branch_display = wt.branch.as_ref().filter(|b| *b != &wt.name);

            let mut spans = vec![
                Span::styled(
                    prefix,
                    if is_selected {
                        Style::default().fg(SELECTED_COLOR)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(&wt.name, name_style),
            ];

            // Add separator and branch only if branch is different from worktree name
            if let Some(branch) = branch_display {
                spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled(
                    format_branch_with_icon(branch, icons_enabled),
                    Style::default()
                        .fg(BRANCH_COLOR)
                        .add_modifier(Modifier::DIM),
                ));
            }

            if wt.is_main {
                spans.push(Span::styled(
                    " [main]",
                    Style::default()
                        .fg(MAIN_WORKTREE_COLOR)
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
    draw_detail_pane(frame, app, main_chunks[1]);

    // Footer
    if let Some(msg) = &app.message {
        let footer = Paragraph::new(msg.as_str()).style(Style::default().fg(Color::Green));
        frame.render_widget(footer, chunks[3]);
    } else {
        let footer = render_normal_footer();
        frame.render_widget(footer, chunks[3]);
    }
}

fn draw_detail_pane(frame: &mut Frame, app: &App, area: Rect) {
    let detail = app.get_selected_worktree_detail();
    let icons_enabled = app.icons_enabled();

    let mut lines: Vec<Line> = Vec::new();

    if let Some(detail) = detail {
        // Branch
        let branch_name = detail.branch.as_deref().unwrap_or("(detached)").to_string();
        let icon_span = if icons_enabled {
            Span::styled(
                format!("{} ", BRANCH_ICON),
                Style::default().fg(BRANCH_COLOR),
            )
        } else {
            Span::raw("")
        };
        lines.push(Line::from(vec![
            Span::styled("Branch: ", Style::default().fg(Color::DarkGray)),
            icon_span,
            Span::styled(
                branch_name,
                Style::default()
                    .fg(BRANCH_COLOR)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Path
        let display_path = app.format_path(&detail.path);
        lines.push(Line::from(vec![
            Span::styled("Path:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(display_path, Style::default().fg(Color::White)),
        ]));

        lines.push(Line::from(""));

        // Changed files
        lines.push(Line::from(vec![Span::styled(
            "Changed Files",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::UNDERLINED),
        )]));

        if detail.changed_files.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  (clean)",
                Style::default().fg(Color::DarkGray),
            )]));
        } else {
            let summary = &detail.changed_files;
            lines.push(Line::from(vec![
                Span::styled("  +", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{} ", summary.added),
                    Style::default().fg(Color::White),
                ),
                Span::styled("-", Style::default().fg(Color::Red)),
                Span::styled(
                    format!("{} ", summary.deleted),
                    Style::default().fg(Color::White),
                ),
                Span::styled("~", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{}", summary.modified),
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        lines.push(Line::from(""));

        // Recent commits
        lines.push(Line::from(vec![Span::styled(
            "Recent Commits",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::UNDERLINED),
        )]));

        if detail.recent_commits.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  (no commits)",
                Style::default().fg(Color::DarkGray),
            )]));
        } else {
            for commit in detail.recent_commits {
                let graph_char = if commit.is_merge { "●" } else { "○" };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", graph_char),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(commit.short_id, Style::default().fg(Color::Yellow)),
                    Span::styled(" ", Style::default()),
                    Span::styled(commit.message, Style::default().fg(Color::White)),
                ]));
            }
        }
    } else {
        lines.push(Line::from(vec![Span::styled(
            "No worktree selected",
            Style::default().fg(Color::DarkGray),
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

fn draw_create_mode(frame: &mut Frame, app: &App, area: Rect) {
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
                .fg(HEADER_COLOR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" - Create Worktree", Style::default().fg(Color::DarkGray)),
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
            .fg(SELECTED_COLOR)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            create_new_prefix,
            if is_create_new_selected {
                Style::default().fg(SELECTED_COLOR)
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
                .fg(SELECTED_COLOR)
                .add_modifier(Modifier::BOLD)
        } else if branch.is_remote {
            Style::default().fg(REMOTE_COLOR)
        } else {
            Style::default().fg(BRANCH_COLOR)
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
                    Style::default().fg(SELECTED_COLOR)
                } else {
                    Style::default()
                },
            ),
            Span::styled(icon_prefix, name_style),
            Span::styled(&branch.name, name_style),
            if branch.is_head {
                Span::styled(" *", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            },
            if branch.is_remote {
                Span::styled(
                    " [remote]",
                    Style::default()
                        .fg(REMOTE_COLOR)
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
        let footer = Paragraph::new(msg.as_str()).style(Style::default().fg(Color::Green));
        frame.render_widget(footer, chunks[4]);
    } else {
        let footer = render_create_footer();
        frame.render_widget(footer, chunks[4]);
    }
}

fn render_normal_footer<'a>() -> Paragraph<'a> {
    Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(KEY_COLOR)),
        Span::styled(": move  ", Style::default().fg(DESC_COLOR)),
        Span::styled("Enter", Style::default().fg(KEY_COLOR)),
        Span::styled(": open  ", Style::default().fg(DESC_COLOR)),
        Span::styled("C-o", Style::default().fg(KEY_COLOR)),
        Span::styled(": create  ", Style::default().fg(DESC_COLOR)),
        Span::styled("C-d", Style::default().fg(KEY_COLOR)),
        Span::styled(": delete  ", Style::default().fg(DESC_COLOR)),
        Span::styled("D", Style::default().fg(KEY_COLOR)),
        Span::styled(": prune  ", Style::default().fg(DESC_COLOR)),
        Span::styled("?", Style::default().fg(KEY_COLOR)),
        Span::styled(": help  ", Style::default().fg(DESC_COLOR)),
        Span::styled("C-q", Style::default().fg(KEY_COLOR)),
        Span::styled(": quit", Style::default().fg(DESC_COLOR)),
    ]))
}

fn render_create_footer<'a>() -> Paragraph<'a> {
    Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(KEY_COLOR)),
        Span::styled(": move  ", Style::default().fg(DESC_COLOR)),
        Span::styled("Enter", Style::default().fg(KEY_COLOR)),
        Span::styled(": create  ", Style::default().fg(DESC_COLOR)),
        Span::styled("Esc", Style::default().fg(KEY_COLOR)),
        Span::styled("/", Style::default().fg(DESC_COLOR)),
        Span::styled("C-c", Style::default().fg(KEY_COLOR)),
        Span::styled(": cancel", Style::default().fg(DESC_COLOR)),
    ]))
}

fn draw_confirm_dialog(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 30, frame.area());

    let message = match app.confirm_action {
        Some(ConfirmAction::DeleteSingle) => {
            let wt = &app.worktrees[app.selected_worktree];
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

    let dialog = Paragraph::new(format!(
        "{}\n\n[y] worktree / [Y] worktree & branch / [n] cancel",
        message
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Confirm")
            .style(Style::default().fg(Color::Yellow))
            .padding(Padding::horizontal(1)),
    )
    .style(Style::default());

    frame.render_widget(Clear, area);
    frame.render_widget(dialog, area);
}

fn draw_help_dialog(frame: &mut Frame) {
    let area = centered_rect(70, 80, frame.area());

    let help_text = vec![
        Line::from(vec![Span::styled(
            "Keybindings",
            Style::default()
                .fg(HEADER_COLOR)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from(vec![
            Span::styled("  ↑ / C-p", Style::default().fg(KEY_COLOR)),
            Span::styled("     Move up", Style::default().fg(DESC_COLOR)),
        ]),
        Line::from(vec![
            Span::styled("  ↓ / C-n", Style::default().fg(KEY_COLOR)),
            Span::styled("     Move down", Style::default().fg(DESC_COLOR)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(KEY_COLOR)),
            Span::styled(
                "       Open worktree / Create",
                Style::default().fg(DESC_COLOR),
            ),
        ]),
        Line::from(vec![
            Span::styled("  C-o", Style::default().fg(KEY_COLOR)),
            Span::styled(
                "         Enter create mode",
                Style::default().fg(DESC_COLOR),
            ),
        ]),
        Line::from(vec![
            Span::styled("  C-d", Style::default().fg(KEY_COLOR)),
            Span::styled("         Delete worktree", Style::default().fg(DESC_COLOR)),
        ]),
        Line::from(vec![
            Span::styled("  D", Style::default().fg(KEY_COLOR)),
            Span::styled(
                "           Prune merged worktrees",
                Style::default().fg(DESC_COLOR),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "General",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from(vec![
            Span::styled("  ?", Style::default().fg(KEY_COLOR)),
            Span::styled("           Show this help", Style::default().fg(DESC_COLOR)),
        ]),
        Line::from(vec![
            Span::styled("  Esc / C-q", Style::default().fg(KEY_COLOR)),
            Span::styled("   Quit / Cancel", Style::default().fg(DESC_COLOR)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    let dialog = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Help")
            .style(Style::default().fg(HEADER_COLOR))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(dialog, area);
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
}
