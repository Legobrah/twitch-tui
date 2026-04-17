use crate::app::{App, FocusTarget};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use super::theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == FocusTarget::Chat;
    let border_color = if focused { theme::CYAN } else { theme::BORDER };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    // Chat messages (newest at bottom)
    let items: Vec<ListItem> = app
        .chat_messages
        .iter()
        .map(|msg| {
            if msg.system {
                ListItem::new(format!("-- {}", msg.message))
                    .style(Style::default().fg(theme::DIM_TEXT))
            } else {
                ListItem::new(format!("{}: {}", msg.sender, msg.message))
                    .style(Style::default().fg(theme::TEXT))
            }
        })
        .collect();

    let chat_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Chat ")
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(chat_list, chunks[0]);

    // Chat input
    let input_style = if focused {
        Style::default().fg(theme::CYAN)
    } else {
        Style::default().fg(theme::DIM_TEXT)
    };
    let input = Paragraph::new(format!("> {}", app.chat_input))
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(input_style),
        );
    f.render_widget(input, chunks[1]);
}
