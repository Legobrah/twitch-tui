use crate::app::{App, AppMode, FocusTarget};
use crate::ui::format::{format_uptime, format_viewers};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use super::theme;

use theme::SELECTION_BG;

fn fg_span<'a>(content: impl Into<std::borrow::Cow<'a, str>>, color: Color, selected: bool) -> Span<'a> {
    let mut style = Style::default().fg(color);
    if selected {
        style = style.bg(SELECTION_BG);
    }
    Span::styled(content, style)
}

fn sep<'a>() -> Span<'a> {
    Span::raw("  ")
}

fn styled_item<'a>(line: Line<'a>, selected: bool) -> ListItem<'a> {
    let bg = if selected { SELECTION_BG } else { Color::Reset };
    ListItem::new(line).style(Style::default().bg(bg))
}

fn more_prompt<'a>() -> ListItem<'a> {
    ListItem::new(Line::from(vec![
        Span::styled("── press 'n' for more ──", Style::default().fg(theme::DIM_TEXT)),
    ]))
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == FocusTarget::Browse;
    let border_color = if focused { theme::CYAN } else { theme::BORDER };
    let mode = app.mode.clone();

    if app.is_loading {
        let loading = Paragraph::new(Line::from(vec![
            Span::styled(" ⟳ ", Style::default().fg(theme::CYAN)),
            Span::styled("Loading...", Style::default().fg(theme::DIM_TEXT)),
        ]))
        .style(Style::default().bg(theme::SURFACE))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", header_title(&mode)))
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme::SURFACE)),
        );
        f.render_widget(loading, area);
        return;
    }

    match &mode {
        AppMode::SavedChannels | AppMode::Followed => {
            render_channels(f, app, area, border_color, focused, &mode);
        }
        AppMode::Categories => {
            render_categories(f, app, area, border_color, focused);
        }
        AppMode::CategoryStreams { game_name, .. } => {
            render_category_streams(f, app, area, border_color, focused, game_name);
        }
        AppMode::Search { query } => {
            render_search(f, app, area, border_color, focused, query);
        }
        AppMode::Vods { channel_name } => {
            render_vods(f, app, area, border_color, focused, channel_name);
        }
        _ => {}
    }
}

fn header_title(mode: &AppMode) -> String {
    match mode {
        AppMode::SavedChannels => "Saved Channels".to_string(),
        AppMode::Followed => "Following".to_string(),
        AppMode::Categories => "Categories".to_string(),
        AppMode::CategoryStreams { game_name, .. } => game_name.clone(),
        AppMode::Search { query } => format!("Search: {}", query),
        AppMode::Vods { channel_name } => format!("VODs: {}", channel_name),
        _ => String::new(),
    }
}

/// Render an empty state with a centered icon and short hints.
fn render_empty(f: &mut Frame, area: Rect, border_color: Color, title: &str, icon: &str, lines: Vec<Line>) {
    let mut all_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", icon),
            Style::default().fg(theme::CYAN),
        )),
    ];
    for l in lines {
        all_lines.push(l);
    }

    let para = Paragraph::new(all_lines)
        .style(Style::default().bg(theme::SURFACE))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme::SURFACE)),
        );
    f.render_widget(para, area);
}

fn render_channels(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_color: Color,
    focused: bool,
    mode: &AppMode,
) {
    if app.channels.is_empty() {
        let (title, icon, lines) = match mode {
            AppMode::SavedChannels => (
                "Saved Channels",
                "📡",
                vec![
                    Line::from(Span::styled("  No saved channels", Style::default().fg(theme::DIM_TEXT))),
                    Line::from(""),
                    Line::from(Span::styled("  / search  ·  f following", Style::default().fg(theme::DIM_TEXT))),
                ],
            ),
            AppMode::Followed => (
                "Following",
                "👤",
                vec![
                    Line::from(Span::styled("  No followed channels", Style::default().fg(theme::DIM_TEXT))),
                ],
            ),
            _ => (
                "Channels",
                "📺",
                vec![
                    Line::from(Span::styled("  No channels", Style::default().fg(theme::DIM_TEXT))),
                ],
            ),
        };
        render_empty(f, area, border_color, title, icon, lines);
        return;
    }

    let live_count = app.channels.iter().filter(|c| c.is_live).count();
    let total = app.channels.len();
    let header = match mode {
        AppMode::Followed => format!(" Following · {} · {} live ", total, live_count),
        _ => format!(" Saved · {} · {} live ", total, live_count),
    };

    let mut items: Vec<ListItem> = app
        .channels
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let sel = i == app.selected_index && focused;
            if ch.is_live {
                let uptime = ch.started_at.as_deref().map(format_uptime).unwrap_or_default();
                let game = ch.game_name.as_deref().unwrap_or("");
                let viewers = ch.viewer_count.map(|v| format_viewers(v)).unwrap_or_default();

                let name_style = if sel {
                    Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD).bg(SELECTION_BG)
                } else {
                    Style::default().fg(theme::TEXT)
                };

                let line = Line::from(vec![
                    Span::styled(theme::LIVE_DOT, Style::default().fg(theme::RED).bg(if sel { SELECTION_BG } else { Color::Reset })),
                    Span::raw(" "),
                    Span::styled(ch.display_name.clone(), name_style),
                    sep(),
                    fg_span(uptime, theme::DIM_TEXT, sel),
                    sep(),
                    fg_span(game, theme::DIM_TEXT, sel),
                    sep(),
                    fg_span(viewers, theme::GREEN, sel),
                ]);
                styled_item(line, sel)
            } else {
                let name_style = if sel {
                    Style::default().fg(theme::DIM_TEXT).add_modifier(Modifier::BOLD).bg(SELECTION_BG)
                } else {
                    Style::default().fg(theme::DIM_TEXT)
                };
                let offline_style = Style::default()
                    .fg(theme::DIM_TEXT)
                    .add_modifier(Modifier::ITALIC)
                    .bg(if sel { SELECTION_BG } else { Color::Reset });

                let line = Line::from(vec![
                    Span::styled(theme::OFFLINE_DOT, Style::default().fg(theme::DIM_TEXT).bg(if sel { SELECTION_BG } else { Color::Reset })),
                    Span::raw(" "),
                    Span::styled(ch.display_name.clone(), name_style),
                    Span::styled("  offline", offline_style),
                ]);
                styled_item(line, sel)
            }
        })
        .collect();

    if app.pagination_cursor.is_some() {
        items.push(more_prompt());
    }

    let list = List::new(items)
        .style(Style::default().bg(theme::SURFACE))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", header))
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme::SURFACE)),
        );

    let mut state = ListState::default();
    if focused && !app.channels.is_empty() {
        state.select(Some(app.selected_index));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn render_categories(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_color: Color,
    focused: bool,
) {
    if app.categories.is_empty() {
        render_empty(f, area, border_color, "Categories", "🎮", vec![
            Line::from(Span::styled("  Press c to load", Style::default().fg(theme::DIM_TEXT))),
        ]);
        return;
    }

    let mut items: Vec<ListItem> = app
        .categories
        .iter()
        .enumerate()
        .map(|(i, game)| {
            let sel = i == app.selected_index && focused;
            let name_style = if sel {
                Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD).bg(SELECTION_BG)
            } else {
                Style::default().fg(theme::TEXT)
            };
            let line = Line::from(vec![
                Span::styled(format!(" {}", game.name), name_style),
            ]);
            styled_item(line, sel)
        })
        .collect();

    if app.pagination_cursor.is_some() {
        items.push(more_prompt());
    }

    let list = List::new(items)
        .style(Style::default().bg(theme::SURFACE))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Categories · {} ", app.categories.len()))
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme::SURFACE)),
        );

    let mut state = ListState::default();
    if focused && !app.categories.is_empty() {
        state.select(Some(app.selected_index));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn render_category_streams(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_color: Color,
    focused: bool,
    game_name: &str,
) {
    if app.category_streams.is_empty() {
        render_empty(f, area, border_color, game_name, "📺", vec![
            Line::from(Span::styled("  No streams", Style::default().fg(theme::DIM_TEXT))),
        ]);
        return;
    }

    let mut items: Vec<ListItem> = app
        .category_streams
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let sel = i == app.selected_index && focused;
            let uptime = ch.started_at.as_deref().map(format_uptime).unwrap_or_default();
            let viewers = ch.viewer_count.map(|v| format_viewers(v)).unwrap_or_default();

            let name_style = if sel {
                Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD).bg(SELECTION_BG)
            } else {
                Style::default().fg(theme::TEXT)
            };

            let line = Line::from(vec![
                Span::styled(theme::LIVE_DOT, Style::default().fg(theme::RED).bg(if sel { SELECTION_BG } else { Color::Reset })),
                Span::raw(" "),
                Span::styled(ch.display_name.clone(), name_style),
                sep(),
                fg_span(uptime, theme::DIM_TEXT, sel),
                sep(),
                fg_span(viewers, theme::GREEN, sel),
            ]);
            styled_item(line, sel)
        })
        .collect();

    if app.pagination_cursor.is_some() {
        items.push(more_prompt());
    }

    let list = List::new(items)
        .style(Style::default().bg(theme::SURFACE))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} · {} streams ", game_name, app.category_streams.len()))
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme::SURFACE)),
        );

    let mut state = ListState::default();
    if focused && !app.category_streams.is_empty() {
        state.select(Some(app.selected_index));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn render_search(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_color: Color,
    focused: bool,
    query: &str,
) {
    if app.search_results.is_empty() && !query.is_empty() {
        render_empty(f, area, border_color, &format!("Search: {}", query), "🔍", vec![
            Line::from(Span::styled(format!("  No results for \"{}\"", query), Style::default().fg(theme::DIM_TEXT))),
        ]);
        return;
    }
    if app.search_results.is_empty() {
        render_empty(f, area, border_color, "Search", "🔍", vec![
            Line::from(Span::styled("  Type to search channels", Style::default().fg(theme::DIM_TEXT))),
        ]);
        return;
    }

    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let sel = i == app.selected_index && focused;
            let dot_str = if ch.is_live { theme::LIVE_DOT } else { theme::OFFLINE_DOT };
            let dot_color = if ch.is_live { theme::RED } else { theme::DIM_TEXT };

            let name_style = if sel {
                Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD).bg(SELECTION_BG)
            } else if ch.is_live {
                Style::default().fg(theme::TEXT)
            } else {
                Style::default().fg(theme::DIM_TEXT)
            };

            let line = Line::from(vec![
                Span::styled(dot_str, Style::default().fg(dot_color).bg(if sel { SELECTION_BG } else { Color::Reset })),
                Span::raw(" "),
                Span::styled(ch.display_name.clone(), name_style),
            ]);
            styled_item(line, sel)
        })
        .collect();

    let title = format!(" Search: {} · {} results ", query, app.search_results.len());

    let list = List::new(items)
        .style(Style::default().bg(theme::SURFACE))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme::SURFACE)),
        );

    let mut state = ListState::default();
    if focused && !app.search_results.is_empty() {
        state.select(Some(app.selected_index));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn render_vods(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_color: Color,
    focused: bool,
    channel_name: &str,
) {
    if app.vods.is_empty() {
        render_empty(f, area, border_color, &format!("VODs: {}", channel_name), "📼", vec![
            Line::from(Span::styled(format!("  No VODs for {}", channel_name), Style::default().fg(theme::DIM_TEXT))),
        ]);
        return;
    }

    let mut items: Vec<ListItem> = app
        .vods
        .iter()
        .enumerate()
        .map(|(i, vod)| {
            let sel = i == app.selected_index && focused;

            let title_style = if sel {
                Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD).bg(SELECTION_BG)
            } else {
                Style::default().fg(theme::TEXT)
            };

            let line = Line::from(vec![
                Span::raw(" "),
                Span::styled(format!("[{}]", vod.duration), Style::default().fg(theme::GREEN).bg(if sel { SELECTION_BG } else { Color::Reset })),
                Span::raw(" "),
                Span::styled(vod.title.clone(), title_style),
            ]);
            styled_item(line, sel)
        })
        .collect();

    if app.pagination_cursor.is_some() {
        items.push(more_prompt());
    }

    let list = List::new(items)
        .style(Style::default().bg(theme::SURFACE))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" VODs: {} · {} videos ", channel_name, app.vods.len()))
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme::SURFACE)),
        );

    let mut state = ListState::default();
    if focused && !app.vods.is_empty() {
        state.select(Some(app.selected_index));
    }
    f.render_stateful_widget(list, area, &mut state);
}
