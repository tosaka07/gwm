use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, DialogKind};
use crate::tui::colors;
use crate::tui::ui::centered_rect;

/// Render the confirmation dialog
pub fn render_confirm_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let dialog_area = centered_rect(50, 30, area);

    // Clear background
    frame.render_widget(Clear, dialog_area);

    let DialogKind::ConfirmDelete { worktree_path } = &app.dialog else {
        return;
    };

    let block = Block::default()
        .title(" Delete Worktree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::ERROR));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let [message_area, _, button_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Message
    let lines = vec![
        Line::from("Are you sure you want to delete:"),
        Line::from(""),
        Line::from(Span::styled(
            worktree_path,
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This action cannot be undone.",
            Style::default().fg(colors::ERROR),
        )),
    ];

    let message = Paragraph::new(lines);
    frame.render_widget(message, message_area);

    // Buttons
    let buttons = Line::from(vec![
        Span::styled(
            " [y] Yes ",
            Style::default()
                .fg(colors::ERROR)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            " [n] No ",
            Style::default()
                .fg(colors::SUCCESS)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let button_paragraph = Paragraph::new(buttons);
    frame.render_widget(button_paragraph, button_area);
}
