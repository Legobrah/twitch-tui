use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use super::theme;

const STATUS_BG: Color = Color::Rgb(10, 14, 30);

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", app.mode_label()),
            Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD).bg(STATUS_BG),
        ),
        Span::styled(
            "│".to_string(),
            Style::default().fg(Color::Rgb(30, 40, 60)).bg(STATUS_BG),
        ),
        Span::styled(
            format!(" {} ", app.key_hints()),
            Style::default().fg(theme::DIM_TEXT).bg(STATUS_BG),
        ),
    ]);

    let para = Paragraph::new(line).style(Style::default().bg(STATUS_BG));
    f.render_widget(para, area);
}
