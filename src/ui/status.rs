use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::Paragraph,
    Frame,
};
use super::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let mode_text = format!(" Mode: {} ", app.mode_label());
    let hints_text = app.key_hints().to_string();

    let content = format!("{:<width$}{}", mode_text, hints_text, width = (area.width as usize) / 3);
    let para = Paragraph::new(content)
        .style(
            Style::default()
                .fg(theme::DIM_TEXT)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(para, area);
}