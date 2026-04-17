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
use tracing::{debug, error, info, warn};

use app::{App, AppEvent, AppMode, FocusTarget};
use config::Config;
use db::Db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Init tracing to file
    let log_dir = directories::ProjectDirs::from("", "", "twitch-tui")
        .expect("Could not determine data directory")
        .data_dir()
        .to_path_buf();
    std::fs::create_dir_all(&log_dir).ok();
    let log_path = log_dir.join("twitch-tui.log");
    let log_file = std::fs::File::create(&log_path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = log_file.metadata()?.permissions();
        perms.set_mode(0o600);
        log_file.set_permissions(perms)?;
    }
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(log_file)
        .with_ansi(false)
        .init();

    info!("twitch-tui starting");
    info!("Log file: {:?}", log_path);

    let config = Config::load()?;
    info!("Config loaded from {:?}", Config::config_dir());
    debug!("Config: client_id={}, has_token={}, poll_interval={}",
        &config.twitch.client_id[..8.min(config.twitch.client_id.len())],
        config.twitch.oauth_token.is_some(),
        config.poll_interval_secs);

    let auth = twitch::auth::Auth::from_config(&config);
    info!("Auth: has_token={}, username={:?}", auth.has_token(), auth.username);

    let db = Db::open()?;
    let saved_channels = db.get_saved_channels()?;
    info!("DB opened, {} saved channels: {:?}", saved_channels.len(),
        saved_channels.iter().map(|c| c.name.clone()).collect::<Vec<_>>());

    // Terminal setup
    crossterm::terminal::enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    crossterm::execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    info!("Terminal initialized");

    let mut app = App::new(saved_channels);
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // Background: poll saved channels for live status
    let tx_poll = tx.clone();
    let poll_logins: Vec<String> = app.saved_channels.iter().map(|c| c.name.clone()).collect();
    let auth_clone = auth.clone();
    let poll_secs = config.poll_interval_secs;
    if !poll_logins.is_empty() {
        info!("Starting background poller for {} channels", poll_logins.len());
        tokio::spawn(async move {
            let api = twitch::api::TwitchApi::new(auth_clone);
            let mut previously_live: Vec<String> = Vec::new();
            loop {
                debug!("Polling streams for {:?}", poll_logins);
                match api.get_streams(&poll_logins, None).await {
                    Ok((channels, _cursor)) => {
                        info!("Poll returned {} live channels", channels.len());
                        let now_live: Vec<String> =
                            channels.iter().map(|c| c.name.clone()).collect();
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
                        let _ = tx_poll.send(AppEvent::ChannelsLoaded(channels, None));
                    }
                    Err(e) => {
                        error!("Poll error: {}", e);
                        let _ = tx_poll.send(AppEvent::Error(format!("Poll error: {}", e)));
                    }
                }
                tokio::time::sleep(Duration::from_secs(poll_secs)).await;
            }
        });
    } else {
        info!("No saved channels, skipping poller");
    }

    let result = run_app(&mut terminal, &mut app, &mut rx, &db, &config, &auth, &tx);

    // Restore terminal
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    terminal.show_cursor()?;
    info!("twitch-tui exiting");

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
                AppEvent::ChannelsLoaded(channels, cursor) => {
                    info!("AppEvent::ChannelsLoaded({} channels)", channels.len());
                    if app.pagination_cursor.is_some() {
                        app.channels.extend(channels);
                    } else {
                        app.channels = channels;
                    }
                    app.pagination_cursor = cursor;
                    app.is_loading = false;
                }
                AppEvent::CategoriesLoaded(games, cursor) => {
                    info!("AppEvent::CategoriesLoaded({} games)", games.len());
                    if app.pagination_cursor.is_some() {
                        app.categories.extend(games);
                    } else {
                        app.categories = games;
                    }
                    app.pagination_cursor = cursor;
                    app.is_loading = false;
                }
                AppEvent::CategoryStreamsLoaded(streams, cursor) => {
                    info!("AppEvent::CategoryStreamsLoaded({} streams)", streams.len());
                    if app.pagination_cursor.is_some() {
                        app.category_streams.extend(streams);
                    } else {
                        app.category_streams = streams;
                    }
                    app.pagination_cursor = cursor;
                    app.is_loading = false;
                }
                AppEvent::SearchResults(results, cursor) => {
                    info!("AppEvent::SearchResults({} results)", results.len());
                    if app.pagination_cursor.is_some() {
                        app.search_results.extend(results);
                    } else {
                        app.search_results = results;
                    }
                    app.pagination_cursor = cursor;
                    app.is_loading = false;
                }
                AppEvent::VodsLoaded(vods, cursor) => {
                    info!("AppEvent::VodsLoaded({} vods)", vods.len());
                    if app.pagination_cursor.is_some() {
                        app.vods.extend(vods);
                    } else {
                        app.vods = vods;
                    }
                    app.pagination_cursor = cursor;
                    app.is_loading = false;
                }
                AppEvent::ChatMessage(msg) => {
                    debug!("Chat: {} ({} chars)", msg.sender, msg.message.len());
                    app.chat_messages.push(msg);
                    if app.chat_messages.len() > 500 {
                        app.chat_messages.drain(0..100);
                    }
                }
                AppEvent::ChatConnected(channel) => {
                    info!("ChatConnected: {}", channel);
                    app.chat_messages.push(twitch::ChatMessage {
                        sender: String::new(),
                        message: format!("Connected to {}", channel),
                        system: true,
                    });
                }
                AppEvent::Error(e) => {
                    error!("AppEvent::Error: {}", e);
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

    // Quality picker mode
    if let AppMode::QualitySelect { quality_index, channel_name, channel_display_name: _ } = &mut app.mode {
        match key.code {
            KeyCode::Esc => {
                let cn = channel_name.clone();
                let quality = config.default_quality.clone();
                tokio::spawn(async move {
                    if let Err(e) = player::watch_stream(&cn, &quality).await {
                        tracing::error!("Player error: {}", e);
                    }
                });
                app.mode = AppMode::SavedChannels;
                app.reset_selection();
                return;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if *quality_index < app::QUALITY_OPTIONS.len() - 1 {
                    *quality_index += 1;
                }
                return;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if *quality_index > 0 {
                    *quality_index -= 1;
                }
                return;
            }
            KeyCode::Enter => {
                let cn = channel_name.clone();
                let quality = app::QUALITY_OPTIONS[*quality_index].to_string();
                tokio::spawn(async move {
                    if let Err(e) = player::watch_stream(&cn, &quality).await {
                        tracing::error!("Player error: {}", e);
                    }
                });
                app.mode = AppMode::SavedChannels;
                app.reset_selection();
                return;
            }
            _ => return,
        }
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
                            info!("Sending chat message to {}", ch);
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
                app.pagination_cursor = None;
                app.reset_selection();
                return;
            }
            KeyCode::Enter => {
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
        KeyCode::Char('q') => {
            info!("Quit requested");
            app.should_quit = true;
        }
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
            info!("Switching to categories mode");
            app.mode = AppMode::Categories;
            app.pagination_cursor = None;
            app.reset_selection();
            app.is_loading = true;
            spawn_categories(auth, tx);
        }
        KeyCode::Char('f') => {
            if auth.has_token() {
                info!("Fetching followed channels");
                app.mode = AppMode::Followed;
                app.pagination_cursor = None;
                app.reset_selection();
                app.is_loading = true;
                spawn_followed(auth, tx);
            } else {
                warn!("No OAuth token for followed channels");
                app.error_message = Some("OAuth required. Set token in ~/.config/twitch-tui/config.toml".to_string());
            }
        }
        KeyCode::Char('/') => {
            info!("Entering search mode");
            app.mode = AppMode::Search {
                query: String::new(),
            };
            app.pagination_cursor = None;
            app.reset_selection();
            app.search_results.clear();
        }
        KeyCode::Char('v') => {
            if let Some(ch) = app.selected_channel() {
                let channel_name = ch.display_name.clone();
                let user_id = ch.twitch_id.clone();
                info!("Fetching VODs for {} ({})", channel_name, user_id);
                app.mode = AppMode::Vods { channel_name };
                app.pagination_cursor = None;
                app.reset_selection();
                app.is_loading = true;
                spawn_vods(auth, tx, &user_id, None);
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::SavedChannels;
            app.pagination_cursor = None;
            app.reset_selection();
        }
        KeyCode::Char('r') => {
            info!("Refresh requested");
            app.is_loading = true;
        }

        KeyCode::Char('n') => {
            if app.pagination_cursor.is_some() {
                info!("Loading next page");
                app.is_loading = true;
                let cursor = app.pagination_cursor.clone();
                match &app.mode {
                    AppMode::SavedChannels | AppMode::Followed => {
                        let logins: Vec<String> = app.saved_channels.iter().map(|c| c.name.clone()).collect();
                        let auth = auth.clone();
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            let api = twitch::api::TwitchApi::new(auth);
                            match api.get_streams(&logins, cursor.as_deref()).await {
                                Ok((channels, c)) => { let _ = tx.send(AppEvent::ChannelsLoaded(channels, c)); }
                                Err(e) => { let _ = tx.send(AppEvent::Error(format!("{}", e))); }
                            }
                        });
                    }
                    AppMode::Categories => {
                        spawn_categories_page(auth, tx, cursor.as_deref());
                    }
                    AppMode::CategoryStreams { game_id, .. } => {
                        spawn_category_streams(auth, tx, game_id, cursor.as_deref());
                    }
                    AppMode::Vods { .. } => {
                        if let Some(ch) = app.selected_channel() {
                            spawn_vods(auth, tx, &ch.twitch_id, cursor.as_deref());
                        }
                    }
                    _ => {}
                }
            }
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
                info!("Entering category: {} ({})", game_name, game_id);
                app.mode = AppMode::CategoryStreams { game_id, game_name };
                app.pagination_cursor = None;
                app.reset_selection();
                app.is_loading = true;
                let gid = match &app.mode {
                    AppMode::CategoryStreams { game_id, .. } => game_id.clone(),
                    _ => unreachable!(),
                };
                spawn_category_streams(auth, tx, &gid, None);
            }
        }
        AppMode::SavedChannels | AppMode::Followed | AppMode::CategoryStreams { .. } => {
            if let Some(ch) = app.selected_channel() {
                let channel_name = ch.name.clone();
                let channel_display_name = ch.display_name.clone();
                let chat_channel = channel_name.clone();
                info!("Opening quality picker for: {}", channel_name);
                let default_quality = &config.default_quality;
                let quality_index = app::QUALITY_OPTIONS
                    .iter()
                    .position(|q| *q == default_quality)
                    .unwrap_or(0);
                app.mode = AppMode::QualitySelect {
                    channel_name,
                    channel_display_name,
                    quality_index,
                };
                connect_chat(auth, tx, &chat_channel, irc_client, current_chat_channel);
            }
        }
        AppMode::Vods { .. } => {
            if let Some(vod) = app.vods.get(app.selected_index) {
                let vod_id = vod.id.clone();
                info!("Watching VOD: {}", vod_id);
                let quality = config.default_quality.clone();
                tokio::spawn(async move {
                    if let Err(e) = player::watch_vod(&vod_id, &quality).await {
                        tracing::error!("VOD player error: {}", e);
                    }
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
                info!("Removing saved channel: {}", name);
                if db.remove_channel(&twitch_id).is_ok() {
                    app.saved_channels.retain(|c| c.twitch_id != twitch_id);
                }
            }
            Ok(false) => {
                info!("Saving channel: {}", name);
                if db.save_channel(&twitch_id, &name, &display_name).is_ok() {
                    app.saved_channels.push(db::SavedChannel {
                        id: 0,
                        twitch_id,
                        name,
                        display_name,
                    });
                }
            }
            Err(e) => {
                error!("DB error checking saved: {}", e);
            }
        }
    }
}

// Async API spawn helpers

fn spawn_categories(auth: &twitch::auth::Auth, tx: &mpsc::UnboundedSender<AppEvent>) {
    let auth = auth.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        debug!("Fetching top games");
        match api.get_top_games(20, None).await {
            Ok((games, cursor)) => {
                info!("Fetched {} games", games.len());
                let _ = tx.send(AppEvent::CategoriesLoaded(games, cursor));
            }
            Err(e) => {
                error!("Categories fetch error: {}", e);
                let _ = tx.send(AppEvent::Error(format!("Categories error: {}", e)));
            }
        }
    });
}

fn spawn_categories_page(
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    after: Option<&str>,
) {
    let auth = auth.clone();
    let tx = tx.clone();
    let c = after.unwrap_or_default().to_string();
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        match api.get_top_games(20, Some(&c)).await {
            Ok((games, cursor)) => {
                let _ = tx.send(AppEvent::CategoriesLoaded(games, cursor));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::Error(format!("Pagination error: {}", e)));
            }
        }
    });
}

fn spawn_category_streams(
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    game_id: &str,
    after: Option<&str>,
) {
    let auth = auth.clone();
    let tx = tx.clone();
    let gid = game_id.to_string();
    let after = after.map(|s| s.to_string());
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        debug!("Fetching streams for game {}", gid);
        match api.get_streams_by_game(&gid, 20, after.as_deref()).await {
            Ok((streams, cursor)) => {
                info!("Fetched {} streams for game", streams.len());
                let _ = tx.send(AppEvent::CategoryStreamsLoaded(streams, cursor));
            }
            Err(e) => {
                error!("Category streams error: {}", e);
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
        let _ = tx.send(AppEvent::SearchResults(Vec::new(), None));
        return;
    }
    let auth = auth.clone();
    let tx = tx.clone();
    let q = query.to_string();
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        debug!("Searching channels: {}", q);
        match api.search_channels(&q, 20).await {
            Ok(results) => {
                info!("Search '{}' returned {} results", q, results.len());
                let _ = tx.send(AppEvent::SearchResults(results, None));
            }
            Err(e) => {
                error!("Search error: {}", e);
                let _ = tx.send(AppEvent::Error(format!("Search error: {}", e)));
            }
        }
    });
}

fn spawn_vods(
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    user_id: &str,
    after: Option<&str>,
) {
    let auth = auth.clone();
    let tx = tx.clone();
    let uid = user_id.to_string();
    let after = after.map(|s| s.to_string());
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        debug!("Fetching VODs for user {}", uid);
        match api.get_vods(&uid, 10, after.as_deref()).await {
            Ok((vods, cursor)) => {
                info!("Fetched {} VODs", vods.len());
                let _ = tx.send(AppEvent::VodsLoaded(vods, cursor));
            }
            Err(e) => {
                error!("VODs error: {}", e);
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
    if current_chat_channel.as_deref() == Some(channel) {
        debug!("Already connected to {}", channel);
        return;
    }

    info!("Connecting to chat: {} (authenticated={})", channel, auth.has_token());
    let result = if let (Some(username), Some(token)) = (&auth.username, &auth.oauth_token) {
        twitch::irc::connect_authenticated(username, token, channel, tx.clone())
    } else {
        twitch::irc::connect_anonymous(channel, tx.clone())
    };

    match result {
        Ok(client) => {
            info!("Chat connected to {}", channel);
            *irc_client = Some(client);
            *current_chat_channel = Some(channel.to_string());
            let _ = tx.send(AppEvent::ChatConnected(channel.to_string()));
        }
        Err(e) => {
            error!("Chat connect error: {}", e);
            let _ = tx.send(AppEvent::Error(format!("Chat connect error: {}", e)));
        }
    }
}

fn spawn_followed(auth: &twitch::auth::Auth, tx: &mpsc::UnboundedSender<AppEvent>) {
    let auth = auth.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        debug!("Fetching current user");
        match api.get_current_user().await {
            Ok(user) => {
                info!("Current user: {} ({})", user.display_name, user.id);
                match api.get_followed_channels(&user.id).await {
                    Ok(channels) => {
                        info!("Fetched {} followed channels", channels.len());
                        let logins: Vec<String> = channels.iter().map(|c| c.name.clone()).collect();
                        if !logins.is_empty() {
                            match api.get_streams(&logins, None).await {
                                Ok((live, _cursor)) => {
                                    info!("{} followed channels are live", live.len());
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
                                    let _ = tx.send(AppEvent::ChannelsLoaded(merged, None));
                                }
                                Err(e) => {
                                    error!("Followed live check error: {}", e);
                                    let _ = tx.send(AppEvent::Error(format!("Followed live check: {}", e)));
                                    let _ = tx.send(AppEvent::ChannelsLoaded(channels, None));
                                }
                            }
                        } else {
                            let _ = tx.send(AppEvent::ChannelsLoaded(channels, None));
                        }
                    }
                    Err(e) => {
                        error!("Followed channels error: {}", e);
                        let _ = tx.send(AppEvent::Error(format!("Followed error: {}", e)));
                    }
                }
            }
            Err(e) => {
                error!("Get current user error: {}", e);
                let _ = tx.send(AppEvent::Error(format!("User info error: {}", e)));
            }
        }
    });
}
