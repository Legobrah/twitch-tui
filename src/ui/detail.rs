use crate::app::{App, AppMode};
use crate::ui::format::{format_uptime, format_viewers_full};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use super::theme;

const TAG_COLORS: &[ratatui::style::Color] = &[
    ratatui::style::Color::Rgb(0, 212, 255),
    ratatui::style::Color::Rgb(166, 227, 161),
    ratatui::style::Color::Rgb(243, 139, 168),
    ratatui::style::Color::Rgb(249, 226, 175),
];

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if let AppMode::Vods { .. } = &app.mode {
        render_vod_detail(f, app, area);
        return;
    }

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

    let mut lines: Vec<Line> = Vec::new();

    // Header line
    if channel.is_live {
        let uptime = channel.started_at.as_deref().map(format_uptime).unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(theme::LIVE_DOT.to_string(), Style::default().fg(theme::RED)),
            Span::raw(" "),
            Span::styled(
                channel.display_name.clone(),
                Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("LIVE for {}", uptime),
                Style::default().fg(theme::GREEN),
            ),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(theme::OFFLINE_DOT.to_string(), Style::default().fg(theme::DIM_TEXT)),
            Span::raw(" "),
            Span::styled(
                channel.display_name.clone(),
                Style::default().fg(theme::DIM_TEXT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Offline", Style::default().fg(theme::DIM_TEXT)),
        ]));
    }

    // Separator
    let sep_width = area.width.saturating_sub(2) as usize;
    let sep: String = "\u{2501}".repeat(sep_width);
    lines.push(Line::from(Span::styled(sep, Style::default().fg(theme::BORDER))));

    // Title
    if let Some(title) = &channel.title {
        lines.push(Line::from(Span::styled(
            title.clone(),
            Style::default().fg(theme::TEXT),
        )));
    }

    // Game
    if let Some(game) = &channel.game_name {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            game.clone(),
            Style::default().fg(theme::DIM_TEXT),
        )));
    }

    // Viewers
    if let Some(viewers) = channel.viewer_count {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("\u{25b3} ", Style::default().fg(theme::GREEN)),
            Span::styled(
                format!("{} viewers", format_viewers_full(viewers)),
                Style::default().fg(theme::GREEN),
            ),
        ]));
    }

    // Tags
    if !channel.tags.is_empty() {
        lines.push(Line::from(""));
        let tag_spans: Vec<Span> = channel
            .tags
            .iter()
            .enumerate()
            .flat_map(|(i, tag)| {
                let color = TAG_COLORS[i % TAG_COLORS.len()];
                let mut spans = vec![
                    Span::styled(format!("[{}]", tag), Style::default().fg(color)),
                ];
                if i < channel.tags.len() - 1 {
                    spans.push(Span::raw(" "));
                }
                spans
            })
            .collect();
        lines.push(Line::from(tag_spans));
    }

    // Context hints
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[s] save · [Enter] watch · [v] vods",
        Style::default().fg(theme::DIM_TEXT),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Stream Info ")
        .style(Style::default().fg(theme::CYAN));
    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

fn render_vod_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let vod = match app.vods.get(app.selected_index) {
        Some(v) => v,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" VOD Info ")
                .style(Style::default().fg(theme::BORDER));
            let hint = Paragraph::new("Select a VOD to see details")
                .block(block)
                .style(Style::default().fg(theme::DIM_TEXT));
            f.render_widget(hint, area);
            return;
        }
    };

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(
            vod.title.clone(),
            Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
        ),
    ]));

    let sep_width = area.width.saturating_sub(2) as usize;
    let sep: String = "\u{2501}".repeat(sep_width);
    lines.push(Line::from(Span::styled(sep, Style::default().fg(theme::BORDER))));

    lines.push(Line::from(Span::styled(
        format!("Duration: {}", vod.duration),
        Style::default().fg(theme::GREEN),
    )));

    lines.push(Line::from(Span::styled(
        format!("Created: {}", vod.created_at),
        Style::default().fg(theme::DIM_TEXT),
    )));

    lines.push(Line::from(Span::styled(
        format!("Channel: {}", vod.user_name),
        Style::default().fg(theme::DIM_TEXT),
    )));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[Enter] play · [Esc] back",
        Style::default().fg(theme::DIM_TEXT),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" VOD Info ")
        .style(Style::default().fg(theme::CYAN));
    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}
