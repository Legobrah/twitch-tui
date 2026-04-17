use crate::app::{App, AppMode, FocusTarget};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use super::theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == FocusTarget::Browse;
    let border_color = if focused { theme::CYAN } else { theme::BORDER };
    let mode = app.mode.clone();

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
    let title = match mode {
        AppMode::Followed => " Followed ",
        _ => " Saved ",
    };

    let items: Vec<ListItem> = app
        .channels
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let prefix = if ch.is_live {
                theme::LIVE_DOT
            } else {
                theme::OFFLINE_DOT
            };
            let viewers = ch
                .viewer_count
                .map(|v| format!(" ({})", v))
                .unwrap_or_default();
            let line = format!("{} {}{}", prefix, ch.display_name, viewers);
            let style = if i == app.selected_index && focused {
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else if ch.is_live {
                Style::default().fg(theme::TEXT)
            } else {
                Style::default().fg(theme::DIM_TEXT)
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
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
    let items: Vec<ListItem> = app
        .categories
        .iter()
        .enumerate()
        .map(|(i, game)| {
            let style = if i == app.selected_index && focused {
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            ListItem::new(format!(" {}", game.name)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Categories ")
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
    let items: Vec<ListItem> = app
        .category_streams
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let viewers = ch
                .viewer_count
                .map(|v| format!(" ({})", v))
                .unwrap_or_default();
            let line = format!("{} {}{}", theme::LIVE_DOT, ch.display_name, viewers);
            let style = if i == app.selected_index && focused {
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", game_name))
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
    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let live = if ch.is_live {
                theme::LIVE_DOT
            } else {
                theme::OFFLINE_DOT
            };
            let line = format!("{} {}", live, ch.display_name);
            let style = if i == app.selected_index && focused {
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Search: {} ", query))
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
    let items: Vec<ListItem> = app
        .vods
        .iter()
        .enumerate()
        .map(|(i, vod)| {
            let line = format!(" [{}] {}", vod.duration, vod.title);
            let style = if i == app.selected_index && focused {
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" VODs: {} ", channel_name))
            .border_style(Style::default().fg(border_color)),
    );

    let mut state = ListState::default();
    if focused && !app.vods.is_empty() {
        state.select(Some(app.selected_index));
    }
    f.render_stateful_widget(list, area, &mut state);
}
