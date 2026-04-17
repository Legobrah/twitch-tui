use crate::app::App;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use super::theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let channel = match app.selected_channel() {
        Some(ch) => ch,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Stream Info ")
                .style(Style::default().fg(theme::BORDER));
            let hint = Paragraph::new("Select a channel to see details")
                .block(block)
                .style(Style::default().fg(theme::DIM_TEXT));
            f.render_widget(hint, area);
            return;
        }
    };

    let mut lines = Vec::new();

    // Header with live status
    let status = if channel.is_live {
        format!("{} {} (LIVE)", theme::LIVE_DOT, channel.display_name)
    } else {
        format!("{} {} (OFFLINE)", theme::OFFLINE_DOT, channel.display_name)
    };
    lines.push(status);
    lines.push(String::new());

    if let Some(title) = &channel.title {
        lines.push(format!("Title: {}", title));
    }
    if let Some(game) = &channel.game_name {
        lines.push(format!("Game: {}", game));
    }
    if let Some(viewers) = channel.viewer_count {
        lines.push(format!("Viewers: {}", viewers));
    }
    if let Some(started) = &channel.started_at {
        lines.push(format!("Started: {}", started));
    }

    let text = lines.join("\n");
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Stream Info ")
        .style(Style::default().fg(theme::CYAN));
    let para = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(theme::TEXT))
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}
