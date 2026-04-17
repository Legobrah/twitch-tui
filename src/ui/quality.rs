use crate::app::{App, AppMode, QUALITY_OPTIONS};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use super::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let (quality_index, channel_display_name) = match &app.mode {
        AppMode::QualitySelect { quality_index, channel_display_name, .. } => (*quality_index, channel_display_name.clone()),
        _ => return,
    };

    let overlay_width = 40u16;
    let overlay_height = (QUALITY_OPTIONS.len() as u16) + 4;
    let x = area.width.saturating_sub(overlay_width) / 2;
    let y = area.height.saturating_sub(overlay_height) / 2;
    let overlay_area = Rect::new(x, y, overlay_width, overlay_height);

    f.render_widget(Clear, overlay_area);

    let items: Vec<ListItem> = QUALITY_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, q)| {
            let prefix = if i == quality_index { "> " } else { "  " };
            let style = if i == quality_index {
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            ListItem::new(format!("{}{}", prefix, q)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Select Quality — {} ", channel_display_name))
            .border_style(Style::default().fg(theme::CYAN)),
    );

    let mut state = ListState::default();
    state.select(Some(quality_index));

    f.render_stateful_widget(list, overlay_area, &mut state);

    let hint_area = Rect::new(x, y + overlay_height, overlay_width, 1);
    let hint = Paragraph::new(" j/k navigate · Enter select · Esc default")
        .style(Style::default().fg(theme::DIM_TEXT));
    f.render_widget(hint, hint_area);
}