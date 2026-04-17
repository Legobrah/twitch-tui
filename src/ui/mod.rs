pub mod browse;
pub mod chat;
pub mod detail;
pub mod help;
pub mod theme;

use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::Paragraph,
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let main_area = if app.error_message.is_some() {
        Rect::new(size.x, size.y, size.width, size.height.saturating_sub(1))
    } else {
        size
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(main_area);

    browse::render(f, app, chunks[0]);
    detail::render(f, app, chunks[1]);
    chat::render(f, app, chunks[2]);

    if let Some(err) = &app.error_message {
        let error_bar = Paragraph::new(err.clone())
            .style(Style::default().fg(theme::RED));
        let bar_area = Rect::new(size.x, size.height.saturating_sub(1), size.width, 1);
        f.render_widget(error_bar, bar_area);
    }

    if app.show_help {
        help::render(f, size);
    }
}
