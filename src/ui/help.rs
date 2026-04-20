use crate::app::App;
use ratatui::{
    layout::{Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use super::theme;

pub fn total_lines() -> u16 {
    help_lines().len() as u16
}

const HELP_SECTIONS: &[(&str, &[(&str, &str)])] = &[
    ("Navigation", &[
        ("Tab / Shift+Tab", "Switch pane"),
        ("j/k, ↑/↓", "Move by one"),
        ("PgDn / PgUp", "Move by ten"),
        ("g / G", "Jump top / bottom"),
        ("n", "Load next page"),
        ("Esc", "Back"),
    ]),
    ("Actions", &[
        ("Enter", "Watch stream"),
        ("s", "Save / unsave"),
        ("/", "Search"),
        ("c", "Categories"),
        ("v", "VODs for selected"),
        ("f", "Following (OAuth)"),
        ("r", "Refresh current view"),
        ("q", "Quit"),
    ]),
    ("Chat", &[
        ("Tab", "Focus chat"),
        ("type + Enter", "Send message"),
        ("↑ / ↓", "Recall prior message"),
        ("Esc", "Back to browse"),
    ]),
    ("Quality", &[
        ("j/k, ↑/↓", "Select quality"),
        ("Enter", "Confirm"),
        ("Esc", "Default quality"),
    ]),
    ("Help overlay", &[
        ("j/k, ↑/↓", "Scroll"),
        ("g / G", "Top / bottom"),
        ("? / Esc / q", "Close"),
    ]),
];

fn help_lines() -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    for (i, (section, bindings)) in HELP_SECTIONS.iter().enumerate() {
        if i > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            section.to_string(),
            Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in *bindings {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<16}", key), Style::default().fg(theme::DIM_TEXT)),
                Span::styled(*desc, Style::default().fg(theme::TEXT)),
            ]));
        }
    }

    lines
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let modal_w = 56u16.min(area.width.saturating_sub(4));
    let modal_h = (total_lines() + 4).min(area.height.saturating_sub(2));
    let modal_area = Rect::new(
        area.x + (area.width.saturating_sub(modal_w)) / 2,
        area.y + (area.height.saturating_sub(modal_h)) / 2,
        modal_w,
        modal_h,
    );

    let dim_bg = Block::default().style(Style::default().bg(ratatui::style::Color::Rgb(4, 5, 16)));
    f.render_widget(dim_bg, area.inner(Margin::new(0, 0)));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .title_bottom(Span::styled(" j/k scroll · ? or Esc close ", Style::default().fg(theme::DIM_TEXT)))
        .style(Style::default().fg(theme::CYAN).bg(theme::SURFACE));
    let para = Paragraph::new(help_lines())
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.help_scroll, 0))
        .style(Style::default().fg(theme::TEXT).bg(theme::SURFACE));
    f.render_widget(Clear, modal_area);
    f.render_widget(para, modal_area);
}
