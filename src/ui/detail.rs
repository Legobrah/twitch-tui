use crate::app::{App, AppMode};
use crate::thumb;
use crate::ui::format::{format_uptime, format_viewers_full};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use ratatui_image::{FilterType, Resize, StatefulImage};
use super::theme;

/// Compute an aspect-correct centered rect for a 16:9 thumbnail inside `inner`.
/// Returns (thumb_rect, remaining_text_rect) or None if the pane is too small.
fn thumb_and_text_areas(inner: Rect, cell: (u16, u16)) -> Option<(Rect, Rect)> {
    // Reserve at least 6 rows below the image for the info block.
    if inner.height < 12 || inner.width < 24 {
        return None;
    }
    let max_rows = (inner.height.saturating_sub(6)).min(22);
    // Target ~45% of pane height for the image block.
    let mut rows = (inner.height as u32 * 45 / 100) as u16;
    rows = rows.clamp(8, max_rows);

    let (cw, ch) = (cell.0.max(1) as u32, cell.1.max(1) as u32);
    // width_cells so that (width_cells * cw) / (rows * ch) ≈ 16/9
    let mut width_cells = ((rows as u32 * ch * 16) / (9 * cw)) as u16;

    let max_width = inner.width.saturating_sub(2); // 1-col side padding
    if width_cells > max_width {
        width_cells = max_width;
        // Recompute rows so aspect stays 16:9.
        rows = ((width_cells as u32 * cw * 9) / (16 * ch)) as u16;
        rows = rows.max(6);
    }
    if rows == 0 || width_cells == 0 {
        return None;
    }

    let pad_x = inner.width.saturating_sub(width_cells) / 2;
    let thumb = Rect::new(inner.x + pad_x, inner.y + 1, width_cells, rows);

    let text_y = thumb.y + rows + 1;
    if text_y >= inner.y + inner.height {
        return None;
    }
    let text = Rect::new(
        inner.x,
        text_y,
        inner.width,
        (inner.y + inner.height) - text_y,
    );
    Some((thumb, text))
}

const TAG_COLORS: &[ratatui::style::Color] = &[
    ratatui::style::Color::Rgb(0, 212, 255),
    ratatui::style::Color::Rgb(166, 227, 161),
    ratatui::style::Color::Rgb(243, 139, 168),
    ratatui::style::Color::Rgb(249, 226, 175),
];

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if let AppMode::QualitySelect { .. } = &app.mode {
        if let Some(ch) = app.watching_channel.clone() {
            let key = thumb::cache_key(&ch);
            render_channel_detail(f, app, &ch, area, true, key.as_deref());
            return;
        }
    }

    if let AppMode::Vods { .. } = &app.mode {
        render_vod_detail(f, app, area);
        return;
    }

    let channel = match app.selected_channel().cloned() {
        Some(ch) => ch,
        None => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled("  ℹ", Style::default().fg(theme::CYAN))),
                Line::from(Span::styled("  Select a channel", Style::default().fg(theme::DIM_TEXT))),
                Line::from(Span::styled("  to see details", Style::default().fg(theme::DIM_TEXT))),
            ];
            let para = Paragraph::new(lines)
                .style(Style::default().bg(theme::SURFACE))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Stream Info ")
                        .border_style(Style::default().fg(theme::BORDER))
                        .style(Style::default().bg(theme::SURFACE)),
                );
            f.render_widget(para, area);
            return;
        }
    };

    let key = thumb::cache_key(&channel);
    render_channel_detail(f, app, &channel, area, false, key.as_deref());
}

fn render_channel_detail(
    f: &mut Frame,
    app: &mut App,
    channel: &crate::twitch::Channel,
    area: Rect,
    watching: bool,
    thumb_key: Option<&str>,
) {
    // Outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Stream Info ")
        .border_style(Style::default().fg(theme::CYAN))
        .style(Style::default().bg(theme::SURFACE));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Decide whether to show a thumbnail. Needs: live + cached + picker +
    // enough room in the pane for an aspect-correct image.
    let cell = app.picker.as_ref().map(|p| p.font_size()).unwrap_or((10, 20));
    let want_thumb = channel.is_live && thumb_key.is_some();
    let areas = if want_thumb {
        thumb_and_text_areas(inner, cell)
    } else {
        None
    };

    let (thumb_area, text_area) = match areas {
        Some((t, rest)) => (Some(t), rest),
        None => (None, inner),
    };

    if let Some(a) = thumb_area {
        if let Some(p) = thumb_key.and_then(|k| app.thumb_cache.get_mut(k)) {
            let img = StatefulImage::default().resize(Resize::Fit(Some(FilterType::Triangle)));
            f.render_stateful_widget(img, a, p);
        }
    }

    let mut lines: Vec<Line> = Vec::new();

    // Header
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

    let sep_width = text_area.width.saturating_sub(0) as usize;
    let sep: String = "\u{2501}".repeat(sep_width);
    lines.push(Line::from(Span::styled(sep, Style::default().fg(theme::BORDER))));

    // Watching indicator
    if watching {
        lines.push(Line::from(vec![
            Span::styled(" ▶ ", Style::default().fg(theme::GREEN)),
            Span::styled("Now playing", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
    }

    if let Some(title) = &channel.title {
        lines.push(Line::from(Span::styled(
            title.clone(),
            Style::default().fg(theme::TEXT),
        )));
    }

    if let Some(game) = &channel.game_name {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Game ", Style::default().fg(theme::DIM_TEXT)),
            Span::styled(game.clone(), Style::default().fg(theme::YELLOW)),
        ]));
    }

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

    // Hints
    lines.push(Line::from(""));
    if watching {
        lines.push(Line::from(Span::styled(
            "j/k quality · Enter confirm · Esc default",
            Style::default().fg(theme::DIM_TEXT),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "[s] save · [Enter] watch · [v] vods",
            Style::default().fg(theme::DIM_TEXT),
        )));
    }

    let para = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(para, text_area);
}

fn render_vod_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let vod = match app.vods.get(app.selected_index) {
        Some(v) => v,
        None => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled("  📼", Style::default().fg(theme::CYAN))),
                Line::from(Span::styled("  Select a VOD", Style::default().fg(theme::DIM_TEXT))),
            ];
            let para = Paragraph::new(lines)
                .style(Style::default().bg(theme::SURFACE))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" VOD Info ")
                        .border_style(Style::default().fg(theme::BORDER))
                        .style(Style::default().bg(theme::SURFACE)),
                );
            f.render_widget(para, area);
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

    lines.push(Line::from(vec![
        Span::styled("Duration ", Style::default().fg(theme::DIM_TEXT)),
        Span::styled(vod.duration.clone(), Style::default().fg(theme::GREEN)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Channel  ", Style::default().fg(theme::DIM_TEXT)),
        Span::styled(vod.user_name.clone(), Style::default().fg(theme::TEXT)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Created  ", Style::default().fg(theme::DIM_TEXT)),
        Span::styled(vod.created_at.clone(), Style::default().fg(theme::DIM_TEXT)),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[Enter] play · [Esc] back",
        Style::default().fg(theme::DIM_TEXT),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" VOD Info ")
        .border_style(Style::default().fg(theme::CYAN))
        .style(Style::default().bg(theme::SURFACE));
    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}
