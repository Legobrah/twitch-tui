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
    Color::Rgb(0, 212, 255),
    Color::Rgb(166, 227, 161),
    Color::Rgb(243, 139, 168),
    Color::Rgb(249, 226, 175),
    Color::Rgb(203, 166, 247),
    Color::Rgb(137, 180, 250),
    Color::Rgb(245, 194, 231),
    Color::Rgb(148, 226, 213),
    Color::Rgb(250, 179, 135),
    Color::Rgb(166, 227, 161),
    Color::Rgb(180, 190, 254),
    Color::Rgb(245, 224, 220),
];

fn username_color(name: &str) -> Color {
    let hash = name.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    CHAT_COLORS[(hash as usize) % CHAT_COLORS.len()]
}

fn user_badge(name: &str) -> Option<&'static str> {
    match name {
        "nightbot" | "moobot" | "streamlabs" | "streamelements" | "wizebot" => Some("BOT"),
        _ => None,
    }
}

/// Estimate display width of a string (treating CJK as 2-wide).
fn display_width(s: &str) -> usize {
    s.chars().map(|c| if c.is_control() { 0 } else if c as u32 > 0x2E80 { 2 } else { 1 }).sum()
}

/// Build wrapped Lines for a chat message.
/// First line includes the prefix (badge + username + ": "),
/// continuation lines are indented to align with the message body.
fn wrap_message<'a>(
    badge: Option<&'a str>,
    sender: &'a str,
    color: Color,
    message: &'a str,
    max_width: usize,
) -> Vec<Line<'a>> {
    // First line: prefix + as many words as fit
    // Continuation lines: start at column 0, use full width
    let prefix_len = badge.map_or(0, |b| b.len() + 3) + sender.len() + 2; // "[BOT] " + "name: "
    let first_width = max_width.saturating_sub(prefix_len);

    if first_width == 0 || max_width < 10 {
        let mut spans: Vec<Span> = Vec::new();
        if let Some(b) = badge {
            spans.push(Span::styled(format!("[{}] ", b), Style::default().fg(theme::RED).add_modifier(Modifier::BOLD)));
        }
        spans.push(Span::styled(sender, Style::default().fg(color).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(": ", Style::default().fg(Color::Rgb(60, 66, 86))));
        spans.push(Span::styled(message, Style::default().fg(theme::TEXT)));
        return vec![Line::from(spans)];
    }

    let words: Vec<&str> = message.split(' ').collect();
    let mut wrapped_lines: Vec<Vec<&str>> = Vec::new();
    let mut current_line: Vec<&str> = Vec::new();
    let mut current_len = 0usize;
    let mut is_first = true;

    for word in &words {
        let w = display_width(word);
        let line_max = if is_first { first_width } else { max_width };
        if current_len > 0 && current_len + 1 + w > line_max {
            wrapped_lines.push(std::mem::take(&mut current_line));
            current_len = 0;
            is_first = false;
        }
        if current_len > 0 {
            current_len += 1;
        }
        current_len += w;
        current_line.push(word);
    }
    if !current_line.is_empty() {
        wrapped_lines.push(current_line);
    }

    if wrapped_lines.is_empty() {
        wrapped_lines.push(Vec::new());
    }

    let mut result = Vec::new();

    for (i, words) in wrapped_lines.into_iter().enumerate() {
        let text: String = words.join(" ");
        if i == 0 {
            let mut spans: Vec<Span> = Vec::new();
            if let Some(b) = badge {
                spans.push(Span::styled(format!("[{}] ", b), Style::default().fg(theme::RED).add_modifier(Modifier::BOLD)));
            }
            spans.push(Span::styled(sender, Style::default().fg(color).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled(": ", Style::default().fg(Color::Rgb(60, 66, 86))));
            spans.push(Span::styled(text, Style::default().fg(theme::TEXT)));
            result.push(Line::from(spans));
        } else {
            // Continuation: no indent, full width
            result.push(Line::from(Span::styled(text, Style::default().fg(theme::TEXT))));
        }
    }

    result
}

fn mentions_user(message: &str, username: Option<&str>) -> bool {
    let Some(user) = username else { return false };
    if user.is_empty() { return false; }
    let needle = user.to_lowercase();
    let haystack = message.to_lowercase();
    let mut rest = haystack.as_str();
    while let Some(idx) = rest.find(&needle) {
        let end = idx + needle.len();
        let before_ok = idx == 0 || !rest.as_bytes()[idx - 1].is_ascii_alphanumeric();
        let after_ok = end == rest.len() || !rest.as_bytes()[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        rest = &rest[end..];
    }
    false
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == FocusTarget::Chat;
    let border_color = if focused { theme::CYAN } else { theme::BORDER };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    let inner_width = chunks[0].width.saturating_sub(2) as usize; // borders
    let self_name = app.username.clone();

    let items: Vec<ListItem> = app
        .chat_messages
        .iter()
        .map(|msg| {
            if msg.system {
                ListItem::new(Line::from(vec![
                    Span::styled(" ◈ ", Style::default().fg(theme::DIM_TEXT)),
                    Span::styled(
                        msg.message.clone(),
                        Style::default().fg(theme::DIM_TEXT).add_modifier(Modifier::ITALIC),
                    ),
                ]))
            } else {
                let color = username_color(&msg.sender);
                let badge = user_badge(&msg.sender);
                let wrapped = wrap_message(badge, &msg.sender, color, &msg.message, inner_width);
                let is_mention = mentions_user(&msg.message, self_name.as_deref());
                if is_mention {
                    ListItem::new(wrapped).style(Style::default().bg(theme::MENTION_BG))
                } else {
                    ListItem::new(wrapped)
                }
            }
        })
        .collect();

    // Count total lines (not items) for scroll
    let total_lines: usize = items.iter().map(|i| i.height()).sum();
    let title = if total_lines > 50 {
        format!(" Chat · {} lines ", total_lines)
    } else if !items.is_empty() {
        " Chat ".to_string()
    } else {
        " Chat — press Enter on a stream to connect ".to_string()
    };

    let list_height = chunks[0].height.saturating_sub(2) as usize;
    let last_visible = if total_lines > list_height {
        items.len() - 1
    } else {
        items.len().saturating_sub(1)
    };

    let chat_list = List::new(items)
        .style(Style::default().bg(theme::SURFACE))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme::SURFACE)),
        );

    let mut state = ListState::default();
    state.select(Some(last_visible));
    f.render_stateful_widget(chat_list, chunks[0], &mut state);

    // Input bar
    let cursor = if focused { "▎" } else { "" };
    let prompt_color = if focused { theme::CYAN } else { theme::DIM_TEXT };

    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(prompt_color)),
        Span::styled(
            app.chat_input.clone(),
            Style::default().fg(if focused { theme::TEXT } else { theme::DIM_TEXT }),
        ),
        Span::styled(cursor, Style::default().fg(theme::CYAN)),
    ]);

    let input = Paragraph::new(input_line)
        .style(Style::default().bg(theme::SURFACE))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme::SURFACE)),
        );
    f.render_widget(input, chunks[1]);
}
