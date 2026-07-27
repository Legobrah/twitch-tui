# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo check                # fast compile sanity check
cargo test                 # run all tests (format.rs, config.rs, db.rs)
cargo test test_name       # run a single test
cargo run                  # run the TUI (requires config.toml with client_id)
cargo build --release      # release build
```

Tests exist in `src/ui/format.rs`, `src/config.rs`, and `src/db.rs` (uses `open_in_memory()` for isolated tests).

Runtime requirements: `streamlink` + `mpv` for playback, `notify-send` for desktop notifications.

## Architecture

Rust TUI app: `ratatui` + `crossterm` for rendering, `tokio` async runtime. Edition 2024, MSRV 1.85.

### Three-Pane Layout

`ui/mod.rs::render()` splits the terminal into:
- **Left (30%)** -- browse pane: saved/followed/search/categories/VODs
- **Center (35%)** -- detail pane: stream/VOD info + thumbnail
- **Right (35%)** -- chat pane: Twitch IRC messages + input

Status bar at top (1 row). Optional error bar below it. Help overlay and quality picker render as modals over the main area.

### Event Loop (`main.rs`)

Single-threaded crossterm event loop polling at 100ms. The `run_app` function:
1. Polls for crossterm `Event` (keyboard, resize)
2. Dispatches key events to `handle_key` (mode-aware routing)
3. Drains `AppEvent`s from the `mpsc` channel (results from async tasks)
4. Redraws every tick

Async work (API calls, chat IRC, thumbnail fetches) is spawned as tokio tasks that send results back through the `mpsc::UnboundedSender<AppEvent>` channel. All state mutations happen on the main thread.

### State (`app.rs`)

`App` struct holds all mutable state: mode, focus, list selections, chat messages, pagination cursors, error state, thumbnail cache. `AppMode` enum determines which list is active and how keybindings behave. `AppEvent` enum is the message type for async-to-main communication.

### Twitch Integration (`twitch/`)

- **`api.rs`** -- Helix REST client (`TwitchApi`). Handles streams, games, search, followed, VODs endpoints with cursor-based pagination.
- **`auth.rs`** -- `Auth` struct wrapping `client_id` + optional OAuth token/username.
- **`irc.rs`** -- `twitch-irc` wrapper. Two connect paths: anonymous and authenticated. Spawns a reader task that converts `Privmsg` to `AppEvent::ChatMessage`.
- **`mod.rs`** -- Shared data types: `Channel`, `Game`, `Vod`, `ChatMessage`.

### UI Modules (`ui/`)

Each module exports a `render(f, app, area)` function:
- **`browse.rs`** -- List rendering, mode-dependent (saved/categories/search/VODs/followed). Largest UI file.
- **`detail.rs`** -- Channel/VOD detail pane with thumbnail when live.
- **`chat.rs`** -- Message list with word-wrap + input line.
- **`status.rs`** -- Top status bar.
- **`help.rs`** -- Scrollable help overlay.
- **`quality.rs`** -- Quality picker overlay.
- **`theme.rs`** -- All color constants and unicode glyphs. New colors must go here.
- **`format.rs`** -- `format_uptime`, `format_viewers` (with tests).

### Supporting Modules

- **`config.rs`** -- TOML config load/save at `~/.config/twitch-tui/config.toml`. File permissions enforced to 0o600.
- **`db.rs`** -- `rusqlite` (bundled) database at `~/.config/twitch-tui/twitch-tui.db`. Schema created inline in `run_migrations()`. No migration files (the `migrations/` dir is empty).
- **`player.rs`** -- Spawns `streamlink` + `mpv`. Validates channel names (alphanumeric + `_`) to prevent command injection.
- **`thumb.rs`** -- Thumbnail LRU cache (32 entries). Debounced fetch (150ms) with sequence-based invalidation. Uses `ratatui-image` for terminal image rendering (Kitty protocol, falls back to halfblocks).
- **`notify.rs`** -- `notify-send` wrapper.

### Data Paths

- Config: `~/.config/twitch-tui/config.toml`
- Database: `~/.config/twitch-tui/twitch-tui.db`
- Logs: `$XDG_DATA_HOME/twitch-tui/twitch-tui.log` (0o600)

## Key Design Constraints

- All async I/O runs in tokio tasks; state mutations only on the main thread.
- All colors must be defined in `ui/theme.rs`. No hardcoded colors elsewhere.
- File permissions 0o600 for config, database, and log files.
- `player.rs` validates channel names -- preserve this to prevent command injection.
- Config is user-editable TOML; don't clobber unknown keys on save.
- The `picker` (image protocol detector) must be initialized before `enable_raw_mode`.

## Commit Style

Conventional commits: `type: short imperative summary` (lowercase, no trailing period). Types: `feat`, `fix`, `refactor`. New commits only, no amending.
