# twitch-tui Design Spec

Rust TUI app for browsing, watching, and chatting on Twitch streams. Built with ratatui, tokio, twitch-irc, rusqlite. Publishes to github.com/Legobrah/twitch-tui.

## Architecture

```
┌─────────────────────────────────────────────┐
│              ratatui TUI (crossterm)         │
│  ┌──────────┐  ┌───────────┐  ┌──────────┐  │
│  │ Browse   │  │ Stream    │  │ Chat     │  │
│  │ Panel    │  │ Detail    │  │ Panel    │  │
│  │          │  │           │  │          │  │
│  │ - Search │  │ - Title   │  │ - IRC    │  │
│  │ - Cats   │  │ - Game    │  │ - Input  │  │
│  │ - Saved  │  │ - Viewers │  │          │  │
│  │ - VODs   │  │ - Uptime  │  │          │  │
│  └──────────┘  └───────────┘  └──────────┘  │
│         ▲           ▲             ▲          │
│         └───────────┼─────────────┘          │
│                     │                        │
│              App State (tokio mpsc)          │
│                     │                        │
│  ┌──────────┐ ┌─────────┐ ┌──────────────┐  │
│  │Twitch API│ │Twitch   │ │Notifications │  │
│  │(reqwest) │ │IRC      │ │(notify-send) │  │
│  └──────────┘ └─────────┘ └──────────────┘  │
│                     │                        │
│              ┌─────────────┐                 │
│              │SQLite (rsql)│                 │
│              │ Saved chans │                 │
│              │ Categories  │                 │
│              │ Settings    │                 │
│              └─────────────┘                 │
│                     │                        │
│              streamlink + mpv (subprocess)    │
└─────────────────────────────────────────────┘
```

Async throughout with tokio. Central `App` state struct with `tokio::sync::mpsc` channels feeding UI updates. SQLite for persistence. streamlink spawned via `tokio::process::Command` for playback.

## Project Structure

```
twitch-tui/
├── Cargo.toml
├── src/
│   ├── main.rs              # entry, tokio spawn, terminal setup
│   ├── app.rs               # App state, mode enum, event handling
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── browse.rs        # channel/category/VOD lists
│   │   ├── detail.rs        # stream info panel
│   │   ├── chat.rs          # IRC chat pane + input
│   │   ├── help.rs          # keybind overlay
│   │   └── theme.rs         # cyan dark colors
│   ├── twitch/
│   │   ├── mod.rs
│   │   ├── api.rs           # Helix API client (streams, games, VODs)
│   │   ├── irc.rs           # Twitch IRC connection
│   │   └── auth.rs          # OAuth token management
│   ├── db.rs                # SQLite operations
│   ├── player.rs            # streamlink + mpv subprocess
│   ├── notify.rs            # live notifications via notify-send
│   └── config.rs            # app settings (~/.config/twitch-tui/)
├── migrations/
│   └── 001_init.sql
└── README.md
```

## Crates

| Purpose | Crate |
|---------|-------|
| TUI framework | ratatui + crossterm |
| Async runtime | tokio |
| HTTP client | reqwest |
| Twitch IRC | twitch-irc |
| Database | rusqlite (bundled sqlite3) |
| Serialization | serde + serde_json |
| Logging | tracing |

## UI Layout

Three panes with dynamic focus:

```
┌─────────────────────────────────────────────────────────┐
│ twitch-tui                                    [? help]  │
├──────────────┬──────────────────┬───────────────────────┤
│ BROWSE       │ STREAM INFO      │ CHAT                  │
│              │                  │                       │
│ > shroud     │ Game: Valorant   │ user1: nice play      │
│   pokimane   │ Viewers: 24,831  │ user2: GG             │
│   xqc        │ Uptime: 3h 42m   │ user3: lol            │
│   hasanabi   │                  │                       │
│              │ "Ranked grind    │                       │
│ ── Category ──│  day 5"         │                       │
│   Just Chat..│                  │                       │
│   Valorant   │ [Watch]          │                       │
│   Fortnite   │ [Save]           │                       │
│              │                  │                       │
├──────────────┴──────────────────┴───────────────────────┤
│ [Tab] switch pane  [Enter] watch  [s] save  [/] search │
└─────────────────────────────────────────────────────────┘
```

## Keybinds

| Key | Action |
|-----|--------|
| Tab / Shift+Tab | Cycle pane focus |
| j/k or Up/Down | Navigate list |
| Enter | Watch stream (spawn mpv) |
| s | Save/unsave channel |
| / | Search channels/categories |
| c | Switch to categories view |
| v | Switch to VODs view (saved channels) |
| f | Show followed channels |
| r | Refresh live data |
| ? | Help overlay |
| q | Quit |

Chat pane focus: typing goes to chat input, Enter sends message.

## Views (Browse Pane)

- **Saved channels** — default view, shows live status of saved channels
- **Categories** — browse top games, drill into streams per game
- **Search** — live search across channels/games
- **VODs** — recent VODs from saved channels
- **Followed** — your Twitch follows (requires OAuth)

## Data Models

### SQLite Schema

```sql
CREATE TABLE channels (
    id          INTEGER PRIMARY KEY,
    twitch_id   TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    display_name TEXT NOT NULL,
    saved_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE categories (
    id          INTEGER PRIMARY KEY,
    twitch_id   TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    icon_url    TEXT
);

CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL
);

INSERT INTO settings (key, value) VALUES
    ('poll_interval_secs', '60'),
    ('default_quality', 'best'),
    ('chat_enabled', 'true'),
    ('notifications_enabled', 'true');
```

### Rust Types

```rust
struct Channel {
    twitch_id: String,
    name: String,
    display_name: String,
    is_live: bool,
    title: Option<String>,
    game: Option<String>,
    viewer_count: Option<u32>,
    uptime: Option<Duration>,
    thumbnail_url: Option<String>,
}

struct Vod {
    id: String,
    title: String,
    duration: String,
    created_at: DateTime<Utc>,
    thumbnail_url: String,
}

enum AppMode {
    SavedChannels,
    Categories,
    CategoryStreams { game_id: String },
    Search { query: String },
    Vods { channel_id: String },
    Followed,
}
```

## Config Location

`~/.config/twitch-tui/`
- `config.toml` — settings overrides (poll interval, quality, OAuth token)
- `twitch-tui.db` — SQLite database

## OAuth Flow

On first run, app prompts for Twitch OAuth token:
1. Opens browser to Twitch auth page (scoped: chat, user:read:follows)
2. User pastes token back into TUI input
3. Token stored in config.toml
4. Auto-refresh if using implicit auth; otherwise manual re-auth on expiry

## Error Handling

- **API errors** (rate limit, network): error bar at bottom, auto-retry with exponential backoff (max 3)
- **IRC disconnects**: auto-reconnect with backoff, show "Reconnecting..." in chat pane
- **streamlink/mpv not found**: clear error on watch attempt, suggest `pacman -S streamlink mpv`
- **SQLite corruption**: recreate DB, channels lost but app works
- **Expired OAuth token**: in-app prompt to re-auth, disable chat/followed until resolved

## Background Tasks

```
tokio::spawn → Poll saved channels (every 60s)
                  → Update is_live in App state
                  → notify-send on newly-live channels

tokio::spawn → IRC reader (continuous)
                  → Push messages to mpsc → chat pane

tokio::spawn → API requests (on-demand)
                  → Search, category browse, VOD fetch
```

All feed into single `mpsc::UnboundedSender<AppEvent>` consumed by main UI loop.

## Build Order (MVP)

1. Project scaffold + SQLite setup
2. Twitch API client (streams, games, search)
3. Browse pane — saved channels + live status
4. Watch streams (spawn streamlink + mpv)
5. Save/unsave channels to SQLite
6. Category browsing
7. Search
8. Chat (IRC)
9. VOD browsing
10. Live notifications
11. OAuth flow
12. Followed channels
