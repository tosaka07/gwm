use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

use crate::app::App;
use crate::tui::colors;
use crate::tui::ui::centered_rect;

/// Render the branch selector dialog
pub fn render_branch_selector(frame: &mut Frame, app: &mut App, area: Rect, title: &str) {
    let dialog_area = centered_rect(60, 60, area);

    // Clear background
    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::BORDER_FOCUS));

    let items: Vec<ListItem> = app
        .branches
        .iter()
        .map(|branch| ListItem::new(branch.as_str()))
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(colors::SELECTION_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, dialog_area, &mut app.branch_list_state);
}
