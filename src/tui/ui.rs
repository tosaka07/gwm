use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;

use super::colors;
use super::widgets::{render_detail_panel, render_help_footer, render_worktree_list};

/// Main render function
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Main layout: header, body, footer
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Render header
    render_header(frame, app, header_area);

    // Body layout: worktree list, detail panel
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(body_area);

    // Render worktree list
    render_worktree_list(frame, app, list_area);

    // Render detail panel
    render_detail_panel(frame, app, detail_area);

    // Render footer
    render_help_footer(frame, app, footer_area);

    // Render dialogs if active
    if app.has_active_dialog() {
        render_dialog(frame, app, area);
    }

    // Render notifications (stacked popups in bottom-right)
    if !app.notifications.is_empty() {
        render_notifications(frame, app, area);
    }
}

/// Render header bar
fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let mode_str = format!("[{}]", app.mode);
    let title = "gwm - Git Worktree Manager";

    let header_text = Line::from(vec![
        Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(
            " ".repeat(
                area.width
                    .saturating_sub(title.len() as u16 + mode_str.len() as u16)
                    as usize,
            ),
        ),
        Span::styled(mode_str, Style::default().fg(colors::PRIMARY)),
    ]);

    let header = Paragraph::new(header_text).style(Style::default().bg(colors::BAR_BG));

    frame.render_widget(header, area);
}

/// Render dialog overlay
fn render_dialog(frame: &mut Frame, app: &mut App, area: Rect) {
    use super::widgets::{
        render_branch_selector, render_confirm_dialog, render_create_worktree_form,
    };

    // Clone dialog kind to avoid borrow conflict
    let dialog_kind = app.dialog.clone();

    match dialog_kind {
        crate::app::DialogKind::None => {}
        crate::app::DialogKind::ConfirmDelete { .. } => {
            render_confirm_dialog(frame, app, area);
        }
        crate::app::DialogKind::CreateWorktree(ref form) => {
            render_create_worktree_form(frame, app, area, form);
        }
        crate::app::DialogKind::BranchSelect { title } => {
            render_branch_selector(frame, app, area, &title);
        }
    }
}

/// Render notifications as stacked popups in bottom-right
/// Oldest at top, newest at bottom
fn render_notifications(frame: &mut Frame, app: &App, area: Rect) {
    const POPUP_WIDTH: u16 = 40;
    const POPUP_HEIGHT: u16 = 3;
    const MAX_VISIBLE: usize = 5;

    // Get indices of visible notifications (last MAX_VISIBLE)
    let start_idx = app.notifications.len().saturating_sub(MAX_VISIBLE);
    let count = app.notifications.len() - start_idx;

    // Render each notification
    for (i, idx) in (start_idx..app.notifications.len()).enumerate() {
        let notification = &app.notifications[idx];

        let (label, border_color) = match notification.level {
            crate::app::NotificationLevel::Info => ("INFO", colors::PRIMARY),
            crate::app::NotificationLevel::Error => ("ERROR", colors::ERROR),
        };

        // Calculate slide offset for animation
        let slide_offset = notification.slide_offset(POPUP_WIDTH + 2);

        // Position: bottom-right, stacking upward, with slide offset
        // i=0 is oldest (top), i=count-1 is newest (bottom)
        let y_offset = (count - 1 - i) as u16 * POPUP_HEIGHT;
        let base_x = area.width.saturating_sub(POPUP_WIDTH + 1);
        let popup_x = base_x + slide_offset;
        let popup_y = area.height.saturating_sub(POPUP_HEIGHT + 1 + y_offset);

        // Skip if fully slid out of view
        if popup_x >= area.width {
            continue;
        }

        // Clip width to visible portion (prevent rendering outside terminal)
        let visible_width = area.width.saturating_sub(popup_x).min(POPUP_WIDTH);
        if visible_width == 0 {
            continue;
        }

        let popup_area = Rect {
            x: popup_x,
            y: popup_y,
            width: visible_width,
            height: POPUP_HEIGHT,
        };

        // Clear background
        frame.render_widget(Clear, popup_area);

        // Build popup
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(format!(" {} ", label))
            .title_style(
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            );

        // Truncate message based on visible width
        let max_msg_len = visible_width.saturating_sub(4) as usize;
        let msg = if max_msg_len == 0 {
            String::new()
        } else if notification.message.len() > max_msg_len {
            format!(
                "{}...",
                &notification.message[..max_msg_len.saturating_sub(3)]
            )
        } else {
            notification.message.clone()
        };

        let paragraph = Paragraph::new(msg).block(block);
        frame.render_widget(paragraph, popup_area);
    }
}

/// Create a centered rectangle
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [_, center, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(area);

    let [_, center, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(center);

    center
}
