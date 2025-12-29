use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::tui::colors;

/// Render the detail panel widget
pub fn render_detail_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL);

    let Some(selected_idx) = app.list_state.selected() else {
        let empty = Paragraph::new("No worktree selected")
            .block(block)
            .style(Style::default().fg(colors::MUTED));
        frame.render_widget(empty, area);
        return;
    };

    let Some(worktree) = app.worktrees.get(selected_idx) else {
        let empty = Paragraph::new("No worktree selected")
            .block(block)
            .style(Style::default().fg(colors::MUTED));
        frame.render_widget(empty, area);
        return;
    };

    let branch_name = worktree.branch.as_deref().unwrap_or("(detached)");

    let lines = vec![
        Line::from(vec![
            Span::styled("Branch: ", Style::default().fg(colors::MUTED)),
            Span::styled(branch_name, Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Path:   ", Style::default().fg(colors::MUTED)),
            Span::raw(worktree.path.display().to_string()),
        ]),
        Line::from(vec![
            Span::styled("Commit: ", Style::default().fg(colors::MUTED)),
            Span::styled(
                &worktree.commit_hash[..7.min(worktree.commit_hash.len())],
                Style::default().fg(colors::HIGHLIGHT),
            ),
        ]),
        Line::from(vec![
            Span::styled("Author: ", Style::default().fg(colors::MUTED)),
            Span::raw(&worktree.commit_author),
        ]),
        Line::from(vec![
            Span::styled("Date:   ", Style::default().fg(colors::MUTED)),
            Span::raw(&worktree.commit_date),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Message:", Style::default().fg(colors::MUTED)),
        ]),
        Line::from(Span::raw(&worktree.commit_message)),
    ];

    let paragraph = Paragraph::new(lines).block(block);

    frame.render_widget(paragraph, area);
}
