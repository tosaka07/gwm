use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::App;
use crate::tui::colors;

/// Render the worktree list widget
pub fn render_worktree_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default().title(" Worktrees ").borders(Borders::ALL);

    let items: Vec<ListItem> = app
        .worktrees
        .iter()
        .map(|wt| {
            // Branch name (main line, bold)
            let branch_name = wt.branch.as_deref().unwrap_or("(detached)");
            let branch_line = Line::from(vec![
                if wt.is_main {
                    Span::raw("")
                } else {
                    Span::styled(" ", Style::default().fg(colors::PRIMARY))
                },
                Span::styled(branch_name, Style::default().add_modifier(Modifier::BOLD)),
                if wt.is_main {
                    Span::styled(" (main)", Style::default().fg(colors::MAIN_TAG))
                } else {
                    Span::raw("")
                },
            ]);

            // Path (secondary line, dimmed)
            let path_str = wt.path.display().to_string();
            let path_line = Line::from(Span::styled(
                format!("  {}", shorten_path(&path_str)),
                Style::default().fg(colors::MUTED),
            ));

            ListItem::new(vec![branch_line, path_line])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(colors::SELECTION_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

/// Shorten path by replacing home directory with ~
fn shorten_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        let home_str = home.display().to_string();
        if path.starts_with(&home_str) {
            return path.replacen(&home_str, "~", 1);
        }
    }
    path.to_string()
}
