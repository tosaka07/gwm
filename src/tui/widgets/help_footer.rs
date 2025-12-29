use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, Mode};
use crate::tui::colors;

/// Render the help footer widget
pub fn render_help_footer(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.mode {
        Mode::Normal => get_normal_mode_help(),
        Mode::Insert => get_insert_mode_help(),
        Mode::Search => get_search_mode_help(),
        Mode::Dialog => get_dialog_mode_help(),
    };

    let spans: Vec<Span> = help_text
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(*key, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(": "),
                Span::styled(*desc, Style::default().fg(colors::MUTED)),
                Span::raw("  "),
            ]
        })
        .collect();

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(colors::BAR_BG));

    frame.render_widget(paragraph, area);
}

fn get_normal_mode_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("j/k", "Move"),
        ("Enter", "Open Shell"),
        ("c", "Create"),
        ("d", "Delete"),
        ("D", "Del Merged"),
        ("r", "Rebase"),
        ("/", "Search"),
        ("?", "Help"),
        ("q", "Quit"),
    ]
}

fn get_insert_mode_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Esc", "Cancel"),
        ("Enter", "Confirm"),
        ("C-a/e", "Start/End"),
    ]
}

fn get_search_mode_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Esc", "Cancel"),
        ("Enter", "Confirm"),
        ("C-n/p", "Next/Prev"),
    ]
}

fn get_dialog_mode_help() -> Vec<(&'static str, &'static str)> {
    vec![("y", "Yes"), ("n", "No"), ("Esc", "Cancel")]
}
