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
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
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
            loop {
                match api.get_streams(&poll_logins).await {
                    Ok(channels) => {
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

    let result = run_app(&mut terminal, &mut app, &mut rx, &db, &config, &auth);

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
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        // Crossterm events (100ms timeout)
        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(key, app, db, config, auth);
            }
        }

        // App events from background tasks
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
) {
    // Help overlay dismiss
    if app.show_help {
        if key.code == KeyCode::Char('?') || key.code == KeyCode::Esc {
            app.show_help = false;
        }
        return;
    }

    // Chat input mode — capture all typing
    if app.focus == FocusTarget::Chat {
        match key.code {
            KeyCode::Esc => {
                app.focus = FocusTarget::Browse;
                return;
            }
            KeyCode::Enter => {
                if !app.chat_input.is_empty() {
                    // TODO: send via IRC in Phase 4
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

        KeyCode::Enter => handle_enter(app, config),

        KeyCode::Char('s') => handle_save(app, db),

        KeyCode::Char('c') => {
            app.mode = AppMode::Categories;
            app.reset_selection();
            app.is_loading = true;
        }
        KeyCode::Char('f') => {
            app.mode = AppMode::Followed;
            app.reset_selection();
            app.is_loading = true;
        }
        KeyCode::Esc => {
            app.mode = AppMode::SavedChannels;
            app.reset_selection();
        }

        KeyCode::Char('r') => {
            app.is_loading = true;
            // Refresh handled by background poller
        }

        _ => {}
    }
}

fn handle_enter(app: &mut App, config: &Config) {
    match &app.mode {
        AppMode::Categories => {
            // Enter category → show streams
            if let Some(game) = app.categories.get(app.selected_index) {
                let game_id = game.id.clone();
                let game_name = game.name.clone();
                app.mode = AppMode::CategoryStreams { game_id, game_name };
                app.reset_selection();
                app.is_loading = true;
                // TODO: spawn API fetch for category streams in Phase 3
            }
        }
        AppMode::SavedChannels | AppMode::Followed | AppMode::CategoryStreams { .. } | AppMode::Search { .. } => {
            // Watch selected stream
            if let Some(ch) = app.selected_channel() {
                let channel_name = ch.name.clone();
                let quality = config.default_quality.clone();
                tokio::spawn(async move {
                    if let Err(e) = player::watch_stream(&channel_name, &quality).await {
                        eprintln!("{}", e);
                    }
                });
            }
        }
        AppMode::Vods { .. } => {
            // Watch selected VOD
            if let Some(vod) = app.vods.get(app.selected_index) {
                let vod_id = vod.id.clone();
                let quality = config.default_quality.clone();
                tokio::spawn(async move {
                    if let Err(e) = player::watch_vod(&vod_id, &quality).await {
                        eprintln!("{}", e);
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
                if db.remove_channel(&twitch_id).is_ok() {
                    app.saved_channels
                        .retain(|c| c.twitch_id != twitch_id);
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
