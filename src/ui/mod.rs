pub mod browse;
pub mod chat;
pub mod detail;
pub mod format;
pub mod help;
pub mod quality;
pub mod status;
pub mod theme;

use crate::app::{App, AppMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Set terminal background
    let bg = Block::default().style(Style::default().bg(theme::BG));
    f.render_widget(bg, size);

    let error_height = if app.error_message.is_some() { 1 } else { 0 };
    let top_height = 1 + error_height;
    let main_area = Rect::new(size.x, size.y + top_height, size.width, size.height.saturating_sub(top_height));
    let status_area = Rect::new(size.x, size.y, size.width, 1);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(main_area);

    status::render(f, app, status_area);
    browse::render(f, app, chunks[0]);
    detail::render(f, app, chunks[1]);
    chat::render(f, app, chunks[2]);

    if let Some(err) = &app.error_message {
        let error_bar = Paragraph::new(format!(" {} ", err))
            .style(Style::default().fg(theme::RED).bg(Color::Rgb(40, 15, 20)));
        let bar_area = Rect::new(size.x, size.y + 1, size.width, 1);
        f.render_widget(error_bar, bar_area);
    }

    if app.show_help {
        help::render(f, main_area);
    }

    if let AppMode::QualitySelect { .. } = &app.mode {
        quality::render(f, app, main_area);
    }
}