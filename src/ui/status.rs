use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use super::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let bg = theme::STATUS_BG;
    let sep = Span::styled(
        " \u{2502} ".to_string(),
        Style::default().fg(theme::BORDER).bg(bg),
    );

    let (auth_dot, auth_text, auth_color) = if app.has_oauth {
        let u = app.username.as_deref().unwrap_or("authed");
        (theme::LIVE_DOT, u.to_string(), theme::GREEN)
    } else {
        (theme::OFFLINE_DOT, "anon".to_string(), theme::DIM_TEXT)
    };

    let loading_span = if app.is_loading {
        let frame = theme::SPINNER_FRAMES[app.spinner_frame % theme::SPINNER_FRAMES.len()];
        Some(Span::styled(
            format!(" {} ", frame),
            Style::default().fg(theme::CYAN).bg(bg),
        ))
    } else {
        None
    };

    let left = vec![
        Span::styled(
            format!(" {} ", app.mode_label()),
            Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD).bg(bg),
        ),
        sep.clone(),
        Span::styled(
            format!(" {} ", app.key_hints()),
            Style::default().fg(theme::DIM_TEXT).bg(bg),
        ),
    ];

    let right = vec![
        Span::styled(format!("{} ", auth_dot), Style::default().fg(auth_color).bg(bg)),
        Span::styled(format!("{} ", auth_text), Style::default().fg(auth_color).bg(bg)),
    ];

    let left_width: usize = left.iter().map(|s| s.width()).sum::<usize>()
        + loading_span.as_ref().map(|s| s.width()).unwrap_or(0);
    let right_width: usize = right.iter().map(|s| s.width()).sum();
    let total_width = area.width as usize;
    let pad = total_width.saturating_sub(left_width + right_width);

    let mut spans: Vec<Span> = Vec::new();
    spans.extend(left);
    if let Some(sp) = loading_span {
        spans.push(sp);
    }
    spans.push(Span::styled(
        " ".repeat(pad),
        Style::default().bg(bg),
    ));
    spans.extend(right);

    let line = Line::from(spans);
    let para = Paragraph::new(line).style(Style::default().bg(bg));
    f.render_widget(para, area);
}
