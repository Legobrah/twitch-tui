use crate::app::{App, FocusTarget};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use super::theme;

const CHAT_COLORS: &[Color] = &[
    Color::Rgb(0, 212, 255),     // cyan
    Color::Rgb(166, 227, 161),    // green
    Color::Rgb(243, 139, 168),    // pink
    Color::Rgb(249, 226, 175),    // yellow
    Color::Rgb(203, 166, 247),    // mauve
    Color::Rgb(137, 180, 250),    // blue
    Color::Rgb(245, 194, 231),    // pink light
    Color::Rgb(148, 226, 213),    // teal
];

fn username_color(name: &str) -> Color {
    let hash = name.bytes().fold(0u8, |acc, b| acc.wrapping_add(b));
    CHAT_COLORS[hash as usize % CHAT_COLORS.len()]
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == FocusTarget::Chat;
    let border_color = if focused { theme::CYAN } else { theme::BORDER };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    // Build chat items with colored usernames and styled system messages
    let items: Vec<ListItem> = app
        .chat_messages
        .iter()
        .map(|msg| {
            if msg.system {
                ListItem::new(Line::from(Span::styled(
                    format!("  {}", msg.message),
                    Style::default().fg(theme::DIM_TEXT).add_modifier(Modifier::ITALIC),
                )))
            } else {
                let color = username_color(&msg.sender);
                let line = Line::from(vec![
                    Span::styled(msg.sender.clone(), Style::default().fg(color)),
                    Span::raw(": "),
                    Span::styled(msg.message.clone(), Style::default().fg(theme::TEXT)),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    // Title with scroll indicator when chat is active
    let title = if app.chat_messages.len() > 50 {
        format!(" Chat · {} messages ", app.chat_messages.len())
    } else {
        " Chat ".to_string()
    };

    let list_height = chunks[0].height.saturating_sub(2) as usize;
    let last_visible = if items.len() > list_height {
        items.len() - 1
    } else {
        items.len().saturating_sub(1)
    };

    let chat_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color)),
    );

    let mut state = ListState::default();
    state.select(Some(last_visible));
    f.render_stateful_widget(chat_list, chunks[0], &mut state);

    // Chat input with blinking cursor indicator
    let cursor = if focused { "▎" } else { "" };
    let input_text = format!("> {}{}", app.chat_input, cursor);
    let input_style = if focused {
        Style::default().fg(theme::CYAN)
    } else {
        Style::default().fg(theme::DIM_TEXT)
    };
    let input = Paragraph::new(input_text)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(input_style),
        );
    f.render_widget(input, chunks[1]);
}
