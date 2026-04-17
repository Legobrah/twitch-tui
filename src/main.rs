#![allow(dead_code)]
mod app;
mod config;
mod db;
mod notify;
mod player;
mod twitch;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use app::{App, AppEvent, AppMode, FocusTarget};
use config::Config;
use db::Db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let auth = twitch::auth::Auth::from_config(&config);

    let db = Db::open()?;
    let saved_channels = db.get_saved_channels()?;

    // Terminal setup
    crossterm::terminal::enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    crossterm::execute!(terminal.backend_mut(), EnterAlternateScreen)?;

    let mut app = App::new(saved_channels);
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // Background: poll saved channels for live status
    let tx_poll = tx.clone();
    let poll_logins: Vec<String> = app.saved_channels.iter().map(|c| c.name.clone()).collect();
    let auth_clone = auth.clone();
    let poll_secs = config.poll_interval_secs;
    if !poll_logins.is_empty() {
        tokio::spawn(async move {
            let api = twitch::api::TwitchApi::new(auth_clone);
            let mut previously_live: Vec<String> = Vec::new();
            loop {
                match api.get_streams(&poll_logins).await {
                    Ok(channels) => {
                        let now_live: Vec<String> =
                            channels.iter().map(|c| c.name.clone()).collect();
                        // Notify on newly live channels
                        for ch in &channels {
                            if !previously_live.contains(&ch.name) {
                                let title = format!("{} is LIVE", ch.display_name);
                                let body = match (&ch.game_name, &ch.title) {
                                    (Some(g), Some(t)) => format!("{} - {}", g, t),
                                    (Some(g), None) => g.clone(),
                                    (None, Some(t)) => t.clone(),
                                    _ => String::new(),
                                };
                                let _ = crate::notify::send_notification(&title, &body).await;
                            }
                        }
                        previously_live = now_live;
                        let _ = tx_poll.send(AppEvent::ChannelsLoaded(channels));
                    }
                    Err(e) => {
                        let _ = tx_poll.send(AppEvent::Error(format!("Poll error: {}", e)));
                    }
                }
                tokio::time::sleep(Duration::from_secs(poll_secs)).await;
            }
        });
    }

    let result = run_app(&mut terminal, &mut app, &mut rx, &db, &config, &auth, &tx);

    // Restore terminal
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<AppEvent>,
    db: &Db,
    config: &Config,
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut irc_client: Option<twitch::irc::IrcClient> = None;
    let mut current_chat_channel: Option<String> = None;

    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(key, app, db, config, auth, tx, &mut irc_client, &mut current_chat_channel);
            }
        }

        while let Ok(evt) = rx.try_recv() {
            match evt {
                AppEvent::ChannelsLoaded(channels) => {
                    app.channels = channels;
                    app.is_loading = false;
                }
                AppEvent::CategoriesLoaded(games) => {
                    app.categories = games;
                    app.is_loading = false;
                }
                AppEvent::CategoryStreamsLoaded(streams) => {
                    app.category_streams = streams;
                    app.is_loading = false;
                }
                AppEvent::SearchResults(results) => {
                    app.search_results = results;
                    app.is_loading = false;
                }
                AppEvent::VodsLoaded(vods) => {
                    app.vods = vods;
                    app.is_loading = false;
                }
                AppEvent::ChatMessage(msg) => {
                    app.chat_messages.push(msg);
                    if app.chat_messages.len() > 500 {
                        app.chat_messages.drain(0..100);
                    }
                }
                AppEvent::ChatConnected(channel) => {
                    app.chat_messages.push(twitch::ChatMessage {
                        sender: String::new(),
                        message: format!("Connected to {}", channel),
                        system: true,
                    });
                }
                AppEvent::Error(e) => {
                    app.error_message = Some(e);
                }
                AppEvent::Tick => {}
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn handle_key(
    key: KeyEvent,
    app: &mut App,
    db: &Db,
    config: &Config,
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    irc_client: &mut Option<twitch::irc::IrcClient>,
    current_chat_channel: &mut Option<String>,
) {
    if app.show_help {
        if key.code == KeyCode::Char('?') || key.code == KeyCode::Esc {
            app.show_help = false;
        }
        return;
    }

    // Chat input mode
    if app.focus == FocusTarget::Chat {
        match key.code {
            KeyCode::Esc => {
                app.focus = FocusTarget::Browse;
                return;
            }
            KeyCode::Enter => {
                if !app.chat_input.is_empty() {
                    if let Some(client) = irc_client {
                        if let Some(channel) = current_chat_channel.as_ref() {
                            let msg = app.chat_input.clone();
                            let ch = channel.clone();
                            let _ = client.say(ch, msg);
                        }
                    }
                    app.chat_input.clear();
                }
                return;
            }
            KeyCode::Backspace => {
                app.chat_input.pop();
                return;
            }
            KeyCode::Char(c) => {
                app.chat_input.push(c);
                return;
            }
            _ => {}
        }
    }

    // Search input mode
    if let AppMode::Search { query } = &mut app.mode {
        match key.code {
            KeyCode::Esc => {
                app.mode = AppMode::SavedChannels;
                app.reset_selection();
                return;
            }
            KeyCode::Enter => {
                // Watch selected search result
                if let Some(ch) = app.selected_channel() {
                    let channel_name = ch.name.clone();
                    let quality = config.default_quality.clone();
                    tokio::spawn(async move {
                        let _ = player::watch_stream(&channel_name, &quality).await;
                    });
                }
                return;
            }
            KeyCode::Backspace => {
                query.pop();
                let q = query.clone();
                spawn_search(auth, tx, &q);
                return;
            }
            KeyCode::Up => {
                app.select_prev();
                return;
            }
            KeyCode::Down => {
                app.select_next();
                return;
            }
            KeyCode::Char(c) => {
                query.push(c);
                let q = query.clone();
                spawn_search(auth, tx, &q);
                return;
            }
            KeyCode::Tab => {
                app.cycle_focus();
                return;
            }
            _ => return,
        }
    }

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => app.show_help = true,
        KeyCode::Tab => app.cycle_focus(),
        KeyCode::BackTab => {
            app.focus = match app.focus {
                FocusTarget::Browse => FocusTarget::Chat,
                FocusTarget::Detail => FocusTarget::Browse,
                FocusTarget::Chat => FocusTarget::Detail,
            };
        }
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),

        KeyCode::Enter => handle_enter(app, config, auth, tx, irc_client, current_chat_channel),

        KeyCode::Char('s') => handle_save(app, db),

        KeyCode::Char('c') => {
            app.mode = AppMode::Categories;
            app.reset_selection();
            app.is_loading = true;
            spawn_categories(auth, tx);
        }
        KeyCode::Char('f') => {
            if auth.has_token() {
                app.mode = AppMode::Followed;
                app.reset_selection();
                app.is_loading = true;
                spawn_followed(auth, tx);
            } else {
                app.error_message = Some("OAuth required. Set token in ~/.config/twitch-tui/config.toml".to_string());
            }
        }
        KeyCode::Char('/') => {
            app.mode = AppMode::Search {
                query: String::new(),
            };
            app.reset_selection();
            app.search_results.clear();
        }
        KeyCode::Char('v') => {
            if let Some(ch) = app.selected_channel() {
                let channel_name = ch.display_name.clone();
                let user_id = ch.twitch_id.clone();
                app.mode = AppMode::Vods { channel_name };
                app.reset_selection();
                app.is_loading = true;
                spawn_vods(auth, tx, &user_id);
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::SavedChannels;
            app.reset_selection();
        }
        KeyCode::Char('r') => {
            app.is_loading = true;
        }

        _ => {}
    }
}

fn handle_enter(
    app: &mut App,
    config: &Config,
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    irc_client: &mut Option<twitch::irc::IrcClient>,
    current_chat_channel: &mut Option<String>,
) {
    match &app.mode {
        AppMode::Categories => {
            if let Some(game) = app.categories.get(app.selected_index) {
                let game_id = game.id.clone();
                let game_name = game.name.clone();
                app.mode = AppMode::CategoryStreams { game_id, game_name };
                app.reset_selection();
                app.is_loading = true;
                let gid = match &app.mode {
                    AppMode::CategoryStreams { game_id, .. } => game_id.clone(),
                    _ => unreachable!(),
                };
                spawn_category_streams(auth, tx, &gid);
            }
        }
        AppMode::SavedChannels | AppMode::Followed | AppMode::CategoryStreams { .. } => {
            if let Some(ch) = app.selected_channel() {
                let channel_name = ch.name.clone();
                let quality = config.default_quality.clone();
                let cn = channel_name.clone();
                tokio::spawn(async move {
                    let _ = player::watch_stream(&cn, &quality).await;
                });
                connect_chat(auth, tx, &channel_name, irc_client, current_chat_channel);
            }
        }
        AppMode::Vods { .. } => {
            if let Some(vod) = app.vods.get(app.selected_index) {
                let vod_id = vod.id.clone();
                let quality = config.default_quality.clone();
                tokio::spawn(async move {
                    let _ = player::watch_vod(&vod_id, &quality).await;
                });
            }
        }
        _ => {}
    }
}

fn handle_save(app: &mut App, db: &Db) {
    if let Some(ch) = app.selected_channel() {
        let twitch_id = ch.twitch_id.clone();
        let name = ch.name.clone();
        let display_name = ch.display_name.clone();

        match db.is_channel_saved(&twitch_id) {
            Ok(true) => {
                if db.remove_channel(&twitch_id).is_ok() {
                    app.saved_channels.retain(|c| c.twitch_id != twitch_id);
                }
            }
            Ok(false) => {
                if db.save_channel(&twitch_id, &name, &display_name).is_ok() {
                    app.saved_channels.push(db::SavedChannel {
                        id: 0,
                        twitch_id,
                        name,
                        display_name,
                    });
                }
            }
            Err(_) => {}
        }
    }
}

// Async API spawn helpers

fn spawn_categories(auth: &twitch::auth::Auth, tx: &mpsc::UnboundedSender<AppEvent>) {
    let auth = auth.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        match api.get_top_games(20).await {
            Ok(games) => {
                let _ = tx.send(AppEvent::CategoriesLoaded(games));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::Error(format!("Categories error: {}", e)));
            }
        }
    });
}

fn spawn_category_streams(
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    game_id: &str,
) {
    let auth = auth.clone();
    let tx = tx.clone();
    let gid = game_id.to_string();
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        match api.get_streams_by_game(&gid, 20).await {
            Ok(streams) => {
                let _ = tx.send(AppEvent::CategoryStreamsLoaded(streams));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::Error(format!("Category streams error: {}", e)));
            }
        }
    });
}

fn spawn_search(
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    query: &str,
) {
    if query.is_empty() {
        let _ = tx.send(AppEvent::SearchResults(Vec::new()));
        return;
    }
    let auth = auth.clone();
    let tx = tx.clone();
    let q = query.to_string();
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        match api.search_channels(&q, 20).await {
            Ok(results) => {
                let _ = tx.send(AppEvent::SearchResults(results));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::Error(format!("Search error: {}", e)));
            }
        }
    });
}

fn spawn_vods(
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    user_id: &str,
) {
    let auth = auth.clone();
    let tx = tx.clone();
    let uid = user_id.to_string();
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        match api.get_vods(&uid, 10).await {
            Ok(vods) => {
                let _ = tx.send(AppEvent::VodsLoaded(vods));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::Error(format!("VODs error: {}", e)));
            }
        }
    });
}

fn connect_chat(
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    channel: &str,
    irc_client: &mut Option<twitch::irc::IrcClient>,
    current_chat_channel: &mut Option<String>,
) {
    // Skip if already connected to this channel
    if current_chat_channel.as_deref() == Some(channel) {
        return;
    }

    let result = if let (Some(username), Some(token)) = (&auth.username, &auth.oauth_token) {
        twitch::irc::connect_authenticated(username, token, channel, tx.clone())
    } else {
        twitch::irc::connect_anonymous(channel, tx.clone())
    };

    match result {
        Ok(client) => {
            *irc_client = Some(client);
            *current_chat_channel = Some(channel.to_string());
            let _ = tx.send(AppEvent::ChatConnected(channel.to_string()));
        }
        Err(e) => {
            let _ = tx.send(AppEvent::Error(format!("Chat connect error: {}", e)));
        }
    }
}

fn spawn_followed(auth: &twitch::auth::Auth, tx: &mpsc::UnboundedSender<AppEvent>) {
    let auth = auth.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        match api.get_current_user().await {
            Ok(user) => match api.get_followed_channels(&user.id).await {
                Ok(channels) => {
                    // Fetch live status for followed
                    let logins: Vec<String> = channels.iter().map(|c| c.name.clone()).collect();
                    if !logins.is_empty() {
                        match api.get_streams(&logins).await {
                            Ok(live) => {
                                let mut merged = channels;
                                for ch in &mut merged {
                                    if let Some(live_ch) = live.iter().find(|l| l.name == ch.name) {
                                        ch.is_live = true;
                                        ch.title = live_ch.title.clone();
                                        ch.game_name = live_ch.game_name.clone();
                                        ch.viewer_count = live_ch.viewer_count;
                                        ch.started_at = live_ch.started_at.clone();
                                    }
                                }
                                let _ = tx.send(AppEvent::ChannelsLoaded(merged));
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::Error(format!("Followed live check: {}", e)));
                                let _ = tx.send(AppEvent::ChannelsLoaded(channels));
                            }
                        }
                    } else {
                        let _ = tx.send(AppEvent::ChannelsLoaded(channels));
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("Followed error: {}", e)));
                }
            },
            Err(e) => {
                let _ = tx.send(AppEvent::Error(format!("User info error: {}", e)));
            }
        }
    });
}
