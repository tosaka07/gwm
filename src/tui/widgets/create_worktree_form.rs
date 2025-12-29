use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, CreateWorktreeForm};
use crate::tui::colors;
use crate::tui::ui::centered_rect;

/// Render the create worktree form dialog (Telescope/fzf style)
pub fn render_create_worktree_form(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    form: &CreateWorktreeForm,
) {
    let dialog_area = centered_rect(50, 60, area);

    // Clear background
    frame.render_widget(Clear, dialog_area);

    // Block with title on border
    let block = Block::default()
        .title(" Create Worktree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::BORDER_FOCUS));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Layout: Input line, Separator, Branch list
    let [input_area, separator_area, list_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(inner);

    // Render input line (Telescope style: "> {name}│")
    render_input_line(frame, input_area, form);

    // Render separator line using Block with top border only
    let separator = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(colors::BORDER_FOCUS));
    frame.render_widget(separator, separator_area);

    // Render branch list
    render_branch_list(frame, app, list_area, form);
}

/// Render the input line with prompt and cursor
fn render_input_line(frame: &mut Frame, area: Rect, form: &CreateWorktreeForm) {
    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(colors::PRIMARY)),
        Span::raw(&form.name),
        Span::styled("│", Style::default().fg(colors::PRIMARY)),
    ]);

    let input = Paragraph::new(input_line);
    frame.render_widget(input, area);
}

/// Render the branch list with selection marker
fn render_branch_list(frame: &mut Frame, app: &App, area: Rect, form: &CreateWorktreeForm) {
    let items: Vec<ListItem> = app
        .branches
        .iter()
        .enumerate()
        .map(|(i, branch)| {
            let is_selected = i == form.branch_idx;
            let marker = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(colors::PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors::MUTED)
            };
            ListItem::new(format!("{}{}", marker, branch)).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, area);
}
