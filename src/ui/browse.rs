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

const SELECTION_BG: Color = Color::Rgb(15, 20, 40);

/// Build a Span with the selection background applied when `selected` is true.
fn fg_span<'a>(content: impl Into<std::borrow::Cow<'a, str>>, color: Color, selected: bool) -> Span<'a> {
    let mut style = Style::default().fg(color);
    if selected {
        style = style.bg(SELECTION_BG);
    }
    Span::styled(content, style)
}

/// Separator span (two spaces) that picks up the item background.
fn sep<'a>() -> Span<'a> {
    Span::raw("  ")
}

/// Build a ListItem from a Line, applying the selection background if selected.
fn styled_item<'a>(line: Line<'a>, selected: bool) -> ListItem<'a> {
    let bg = if selected { SELECTION_BG } else { Color::Reset };
    ListItem::new(line).style(Style::default().bg(bg))
}

/// Centered pagination prompt with box-drawing characters.
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
        let loading = Paragraph::new(" Loading...")
            .style(Style::default().fg(theme::DIM_TEXT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", header_title(&mode)))
                    .border_style(Style::default().fg(border_color)),
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

fn render_channels(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_color: ratatui::style::Color,
    focused: bool,
    mode: &AppMode,
) {
    if app.channels.is_empty() {
        let msg = match mode {
            AppMode::SavedChannels => "No saved channels.\nPress / to search or f to browse followed.".to_string(),
            AppMode::Followed => "No followed channels found.".to_string(),
            _ => "No channels.".to_string(),
        };
        let empty = Paragraph::new(msg)
            .style(Style::default().fg(theme::DIM_TEXT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", header_title(mode)))
                    .border_style(Style::default().fg(border_color)),
            );
        f.render_widget(empty, area);
        return;
    }

    let live_count = app.channels.iter().filter(|c| c.is_live).count();
    let total = app.channels.len();
    let header = match mode {
        AppMode::Followed => format!(" Following · {} ch · {} live ", total, live_count),
        _ => format!(" Saved · {} ch · {} live ", total, live_count),
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

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", header))
            .border_style(Style::default().fg(border_color)),
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
    border_color: ratatui::style::Color,
    focused: bool,
) {
    if app.categories.is_empty() {
        let empty = Paragraph::new("No categories loaded.\nPress 'c' to load categories.")
            .style(Style::default().fg(theme::DIM_TEXT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Categories ")
                    .border_style(Style::default().fg(border_color)),
            );
        f.render_widget(empty, area);
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

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Categories · {} ", app.categories.len()))
            .border_style(Style::default().fg(border_color)),
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
    border_color: ratatui::style::Color,
    focused: bool,
    game_name: &str,
) {
    if app.category_streams.is_empty() {
        let empty = Paragraph::new("No streams in this category.")
            .style(Style::default().fg(theme::DIM_TEXT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", game_name))
                    .border_style(Style::default().fg(border_color)),
            );
        f.render_widget(empty, area);
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

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} · {} streams ", game_name, app.category_streams.len()))
            .border_style(Style::default().fg(border_color)),
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
    border_color: ratatui::style::Color,
    focused: bool,
    query: &str,
) {
    if app.search_results.is_empty() && !query.is_empty() {
        let empty = Paragraph::new(format!("No results for \"{}\"", query))
            .style(Style::default().fg(theme::DIM_TEXT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Search: {} ", query))
                    .border_style(Style::default().fg(border_color)),
            );
        f.render_widget(empty, area);
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
                Span::styled(dot_str, Style::default().fg(dot_color).bg(if sel { SELECTION_BG } else { Color:: Reset })),
                Span::raw(" "),
                Span::styled(ch.display_name.clone(), name_style),
            ]);
            styled_item(line, sel)
        })
        .collect();

    let title = if query.is_empty() {
        " Search ".to_string()
    } else {
        format!(" Search: {} · {} results ", query, app.search_results.len())
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color)),
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
    border_color: ratatui::style::Color,
    focused: bool,
    channel_name: &str,
) {
    if app.vods.is_empty() {
        let empty = Paragraph::new(format!("No VODs available for {}", channel_name))
            .style(Style::default().fg(theme::DIM_TEXT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" VODs: {} ", channel_name))
                    .border_style(Style::default().fg(border_color)),
            );
        f.render_widget(empty, area);
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

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" VODs: {} · {} videos ", channel_name, app.vods.len()))
            .border_style(Style::default().fg(border_color)),
    );

    let mut state = ListState::default();
    if focused && !app.vods.is_empty() {
        state.select(Some(app.selected_index));
    }
    f.render_stateful_widget(list, area, &mut state);
}
