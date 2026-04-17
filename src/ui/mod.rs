pub mod browse;
pub mod chat;
pub mod detail;
pub mod help;
pub mod theme;

use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(size);

    browse::render(f, app, chunks[0]);
    detail::render(f, app, chunks[1]);
    chat::render(f, app, chunks[2]);

    if app.show_help {
        help::render(f, size);
    }
}
