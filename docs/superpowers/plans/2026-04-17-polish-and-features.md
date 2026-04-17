# Twitch-TUI Polish & Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Polish the visual presentation of browse/detail panels, add a status bar, loading/empty states, quality picker, tags display, and pagination.

**Architecture:** Layer changes bottom-up: format helpers and data model first, then API pagination/tags, then app state, then UI rendering. Each task builds on the previous. The UI tasks (status bar, browse polish, detail polish, quality picker) are independent of each other once the foundation is in place.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.28, tokio 1, reqwest 0.12, chrono (new dep for uptime formatting)

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src/ui/format.rs` | Create | `format_uptime()`, `format_viewers()` helpers |
| `src/ui/status.rs` | Create | Status bar rendering (mode + key hints) |
| `src/ui/quality.rs` | Create | Quality picker overlay rendering |
| `src/twitch/mod.rs` | Modify | Add `tags: Vec<String>` to Channel |
| `src/twitch/api.rs` | Modify | Tags deserialization, pagination params + cursor returns |
| `src/app.rs` | Modify | QualitySelect mode, pagination_cursor, updated AppEvents |
| `src/main.rs` | Modify | Quality picker handling, pagination `n` key, updated event handlers |
| `src/ui/mod.rs` | Modify | Status bar layout, quality overlay rendering |
| `src/ui/browse.rs` | Modify | Polished format, headers, loading/empty states, pagination prompt |
| `src/ui/detail.rs` | Modify | Structured layout, tags badges, context hints |
| `src/ui/help.rs` | Modify | Add new keybindings (n, quality picker) |
| `Cargo.toml` | Modify | Add `chrono` dependency |

---

### Task 1: Add chrono dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add chrono to Cargo.toml**

Add `chrono` to the `[dependencies]` section in `Cargo.toml`, after the `serde_json` line:

```toml
chrono = "0.4"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check 2>&1 | tail -5`
Expected: `Finished` without errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add chrono dependency for uptime formatting"
```

---

### Task 2: Format helpers (format_uptime, format_viewers)

**Files:**
- Create: `src/ui/format.rs`
- Modify: `src/ui/mod.rs` (add `pub mod format;`)

- [ ] **Step 1: Write tests for format helpers**

Create `src/ui/format.rs` with tests at the bottom:

```rust
pub fn format_uptime(started_at: &str) -> String {
    let started = chrono::DateTime::parse_from_rfc3339(started_at);
    match started {
        Ok(t) => {
            let now = chrono::Utc::now();
            let diff = now.signed_duration_since(t);
            let total_minutes = diff.num_minutes();
            let hours = total_minutes / 60;
            let minutes = total_minutes % 60;
            let days = hours / 24;
            let hours = hours % 24;
            if days > 0 {
                format!("{}d {}h", days, hours)
            } else if hours > 0 {
                format!("{}h {:02}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            }
        }
        Err(_) => String::new(),
    }
}

pub fn format_viewers(count: u32) -> String {
    if count >= 1_000_000 {
        format!("{:.1}m", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

pub fn format_viewers_full(count: u32) -> String {
    let s = count.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_viewers() {
        assert_eq!(format_viewers(0), "0");
        assert_eq!(format_viewers(42), "42");
        assert_eq!(format_viewers(999), "999");
        assert_eq!(format_viewers(1000), "1.0k");
        assert_eq!(format_viewers(23400), "23.4k");
        assert_eq!(format_viewers(42391), "42.4k");
        assert_eq!(format_viewers(1000000), "1.0m");
        assert_eq!(format_viewers(2500000), "2.5m");
    }

    #[test]
    fn test_format_viewers_full() {
        assert_eq!(format_viewers_full(0), "0");
        assert_eq!(format_viewers_full(42), "42");
        assert_eq!(format_viewers_full(42391), "42,391");
        assert_eq!(format_viewers_full(1000000), "1,000,000");
    }

    #[test]
    fn test_format_uptime_minutes() {
        let five_min_ago = (chrono::Utc::now() - chrono::Duration::minutes(5))
            .to_rfc3339();
        assert_eq!(format_uptime(&five_min_ago), "5m");
    }

    #[test]
    fn test_format_uptime_hours_minutes() {
        let two_hours_ago = (chrono::Utc::now() - chrono::Duration::hours(2) - chrono::Duration::minutes(15))
            .to_rfc3339();
        assert_eq!(format_uptime(&two_hours_ago), "2h 15m");
    }

    #[test]
    fn test_format_uptime_days() {
        let one_day_ago = (chrono::Utc::now() - chrono::Duration::hours(27))
            .to_rfc3339();
        assert_eq!(format_uptime(&one_day_ago), "1d 3h");
    }

    #[test]
    fn test_format_uptime_invalid() {
        assert_eq!(format_uptime("not-a-date"), "");
    }
}
```

- [ ] **Step 2: Add module declaration to ui/mod.rs**

Add `pub mod format;` to `src/ui/mod.rs` at the top, after `pub mod browse;`:

```rust
pub mod browse;
pub mod chat;
pub mod detail;
pub mod format;
pub mod help;
pub mod theme;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --lib ui::format::tests -- --nocapture`
Expected: All 5 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/ui/format.rs src/ui/mod.rs
git commit -m "feat: add format helpers for uptime and viewer counts"
```

---

### Task 3: Add tags to Channel struct

**Files:**
- Modify: `src/twitch/mod.rs`

- [ ] **Step 1: Add tags field to Channel**

In `src/twitch/mod.rs`, add `tags` field to the `Channel` struct after `thumbnail_url`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    pub twitch_id: String,
    pub name: String,
    pub display_name: String,
    #[serde(default)]
    pub is_live: bool,
    pub title: Option<String>,
    pub game_name: Option<String>,
    pub viewer_count: Option<u32>,
    pub started_at: Option<String>,
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: May see warnings about unused `tags` field but no errors. The `#[serde(default)]` ensures deserialization works when tags are missing.

- [ ] **Step 3: Commit**

```bash
git add src/twitch/mod.rs
git commit -m "feat: add tags field to Channel struct"
```

---

### Task 4: API pagination and tags deserialization

**Files:**
- Modify: `src/twitch/api.rs`

- [ ] **Step 1: Add tags to StreamData and update API methods**

Replace the `StreamData` struct (lines 19-30) with:

```rust
#[derive(Deserialize)]
#[allow(dead_code)]
struct StreamData {
    user_id: String,
    user_login: String,
    user_name: String,
    title: String,
    game_name: String,
    viewer_count: u32,
    started_at: String,
    thumbnail_url: String,
    #[serde(default)]
    tags: Vec<String>,
}
```

Add a generic pagination response wrapper after the existing response structs (after line 77):

```rust
#[derive(Deserialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    pagination: Option<Pagination>,
}

#[derive(Deserialize)]
struct Pagination {
    cursor: Option<String>,
}
```

Replace the `get_streams` method (lines 108-156) with:

```rust
pub async fn get_streams(
    &self,
    user_logins: &[String],
    after: Option<&str>,
) -> Result<(Vec<Channel>, Option<String>), String> {
    let headers = self.build_headers().map_err(|e| { error!("build_headers error: {}", e); e })?;
    let params: Vec<String> = user_logins
        .iter()
        .map(|l| format!("user_login={}", l))
        .collect();
    let mut url = format!("{}/streams?{}", self.base_url, params.join("&"));
    if let Some(cursor) = after {
        url.push_str(&format!("&after={}", cursor));
    }

    let resp = self
        .client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| { error!("Network error in get_streams: {}", e); format!("Network error: {}", e) })?;

    debug!("get_streams response status: {}", resp.status());
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        error!("get_streams API error {}: {}", status, body);
        return Err(format!("API error {}: {}", status, body));
    }

    let raw = resp.text().await.map_err(|e| format!("Read body error: {}", e))?;
    debug!("get_streams response body (first 500 chars): {}", &raw[..500.min(raw.len())]);
    let data: PaginatedResponse<StreamData> = serde_json::from_str(&raw)
        .map_err(|e| { error!("Parse error in get_streams: {}", e); format!("Parse error: {}", e) })?;

    let cursor = data.pagination.and_then(|p| p.cursor);
    let channels = data
        .data
        .into_iter()
        .map(|s| Channel {
            twitch_id: s.user_id,
            name: s.user_login,
            display_name: s.user_name,
            is_live: true,
            title: Some(s.title),
            game_name: if s.game_name.is_empty() {
                None
            } else {
                Some(s.game_name)
            },
            viewer_count: Some(s.viewer_count),
            started_at: Some(s.started_at),
            thumbnail_url: Some(s.thumbnail_url),
            tags: s.tags,
        })
        .collect();

    Ok((channels, cursor))
}
```

Replace the `get_top_games` method (lines 158-184) with:

```rust
pub async fn get_top_games(
    &self,
    first: u32,
    after: Option<&str>,
) -> Result<(Vec<Game>, Option<String>), String> {
    let headers = self.build_headers()?;
    let mut url = format!("{}/games/top?first={}", self.base_url, first);
    if let Some(cursor) = after {
        url.push_str(&format!("&after={}", cursor));
    }

    let resp = self
        .client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let data: PaginatedResponse<GameData> = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let cursor = data.pagination.and_then(|p| p.cursor);
    let games = data
        .data
        .into_iter()
        .map(|g| Game {
            id: g.id,
            name: g.name,
            box_art_url: g.box_art_url,
        })
        .collect();

    Ok((games, cursor))
}
```

Replace the `get_streams_by_game` method (lines 186-229) with:

```rust
pub async fn get_streams_by_game(
    &self,
    game_id: &str,
    first: u32,
    after: Option<&str>,
) -> Result<(Vec<Channel>, Option<String>), String> {
    let headers = self.build_headers()?;
    let mut url = format!(
        "{}/streams?game_id={}&first={}",
        self.base_url, game_id, first
    );
    if let Some(cursor) = after {
        url.push_str(&format!("&after={}", cursor));
    }

    let resp = self
        .client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let data: PaginatedResponse<StreamData> = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let cursor = data.pagination.and_then(|p| p.cursor);
    let channels = data
        .data
        .into_iter()
        .map(|s| Channel {
            twitch_id: s.user_id,
            name: s.user_login,
            display_name: s.user_name,
            is_live: true,
            title: Some(s.title),
            game_name: if s.game_name.is_empty() {
                None
            } else {
                Some(s.game_name)
            },
            viewer_count: Some(s.viewer_count),
            started_at: Some(s.started_at),
            thumbnail_url: Some(s.thumbnail_url),
            tags: s.tags,
        })
        .collect();

    Ok((channels, cursor))
}
```

Replace the `get_vods` method (lines 272-304) with:

```rust
pub async fn get_vods(
    &self,
    user_id: &str,
    first: u32,
    after: Option<&str>,
) -> Result<(Vec<Vod>, Option<String>), String> {
    let headers = self.build_headers()?;
    let mut url = format!(
        "{}/videos?user_id={}&first={}",
        self.base_url, user_id, first
    );
    if let Some(cursor) = after {
        url.push_str(&format!("&after={}", cursor));
    }

    let resp = self
        .client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let data: PaginatedResponse<VideoData> = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let cursor = data.pagination.and_then(|p| p.cursor);
    let vods = data
        .data
        .into_iter()
        .map(|v| Vod {
            id: v.id,
            title: v.title,
            duration: v.duration,
            created_at: v.created_at,
            thumbnail_url: v.thumbnail_url,
            user_name: v.user_name,
        })
        .collect();

    Ok((vods, cursor))
}
```

Remove the now-unused `StreamsResponse`, `GamesResponse`, `VideosResponse` structs (lines 14-17, 32-35, 63-67). They are replaced by `PaginatedResponse`.

Keep `SearchChannelsResponse` as-is (the search endpoint is not paginated in this plan).

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | tail -10`

There will be compilation errors in `main.rs` because API methods now return tuples. We'll fix those in Task 6. For now, just verify the API module itself parses correctly — the errors should be in `main.rs` only.

Expected: Errors about mismatched types in `main.rs`, but `src/twitch/api.rs` itself compiles.

- [ ] **Step 3: Commit**

```bash
git add src/twitch/api.rs
git commit -m "feat: add pagination cursors and tags to API methods"
```

---

### Task 5: Update app state (QualitySelect, pagination_cursor, AppEvents)

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Update AppMode, AppEvent, and App struct**

Replace the entire `src/app.rs` with:

```rust
use crate::db::SavedChannel;
use crate::twitch::{Channel, ChatMessage, Game, Vod};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    SavedChannels,
    Categories,
    CategoryStreams {
        game_id: String,
        game_name: String,
    },
    Search {
        query: String,
    },
    Vods {
        channel_name: String,
    },
    Followed,
    QualitySelect {
        channel_name: String,
        channel_display_name: String,
        quality_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusTarget {
    Browse,
    Detail,
    Chat,
}

#[derive(Debug)]
pub enum AppEvent {
    ChannelsLoaded(Vec<Channel>, Option<String>),
    CategoriesLoaded(Vec<Game>, Option<String>),
    CategoryStreamsLoaded(Vec<Channel>, Option<String>),
    SearchResults(Vec<Channel>, Option<String>),
    VodsLoaded(Vec<Vod>, Option<String>),
    ChatMessage(ChatMessage),
    ChatConnected(String),
    Error(String),
    Tick,
}

pub const QUALITY_OPTIONS: &[&str] = &["best", "1080p", "720p", "480p", "360p", "audio_only"];

pub struct App {
    pub mode: AppMode,
    pub focus: FocusTarget,
    pub saved_channels: Vec<SavedChannel>,
    pub channels: Vec<Channel>,
    pub categories: Vec<Game>,
    pub category_streams: Vec<Channel>,
    pub search_results: Vec<Channel>,
    pub vods: Vec<Vod>,
    pub chat_messages: Vec<ChatMessage>,
    pub chat_input: String,
    pub selected_index: usize,
    pub show_help: bool,
    pub error_message: Option<String>,
    pub is_loading: bool,
    pub should_quit: bool,
    pub pagination_cursor: Option<String>,
}

impl App {
    pub fn new(saved_channels: Vec<SavedChannel>) -> Self {
        Self {
            mode: AppMode::SavedChannels,
            focus: FocusTarget::Browse,
            saved_channels,
            channels: Vec::new(),
            categories: Vec::new(),
            category_streams: Vec::new(),
            search_results: Vec::new(),
            vods: Vec::new(),
            chat_messages: Vec::new(),
            chat_input: String::new(),
            selected_index: 0,
            show_help: false,
            error_message: None,
            is_loading: false,
            should_quit: false,
            pagination_cursor: None,
        }
    }

    pub fn current_channels(&self) -> &[Channel] {
        match &self.mode {
            AppMode::SavedChannels | AppMode::Followed => &self.channels,
            AppMode::CategoryStreams { .. } => &self.category_streams,
            AppMode::Search { .. } => &self.search_results,
            _ => &self.channels,
        }
    }

    pub fn selected_channel(&self) -> Option<&Channel> {
        self.current_channels().get(self.selected_index)
    }

    pub fn select_next(&mut self) {
        let len = self.current_channels().len();
        if len > 0 && self.selected_index < len - 1 {
            self.selected_index += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusTarget::Browse => FocusTarget::Detail,
            FocusTarget::Detail => FocusTarget::Chat,
            FocusTarget::Chat => FocusTarget::Browse,
        };
    }

    pub fn reset_selection(&mut self) {
        self.selected_index = 0;
    }

    pub fn mode_label(&self) -> &str {
        match &self.mode {
            AppMode::SavedChannels => "Saved Channels",
            AppMode::Categories => "Categories",
            AppMode::CategoryStreams { game_name, .. } => game_name,
            AppMode::Search { .. } => "Search",
            AppMode::Vods { channel_name } => channel_name,
            AppMode::Followed => "Following",
            AppMode::QualitySelect { .. } => "Quality",
        }
    }

    pub fn key_hints(&self) -> &str {
        if self.show_help {
            return "? close help";
        }
        if let FocusTarget::Chat = self.focus {
            return "type message · Enter send · Esc back";
        }
        match &self.mode {
            AppMode::Search { .. } => "type to search · Esc back · Enter watch",
            AppMode::Categories => "Enter select · Esc back · ? help",
            AppMode::Vods { .. } => "Enter play · Esc back · ? help",
            AppMode::QualitySelect { .. } => "j/k choose · Enter select · Esc cancel",
            _ => "j/k nav · Enter watch · s save · n more · ? help",
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | grep "^error" | head -5`

There will be errors in `main.rs` from the changed `AppEvent` variants and `App` field. We fix those in Task 6.

Expected: Errors only in `main.rs`, not `app.rs`.

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "feat: add QualitySelect mode, pagination cursor, and mode helpers to App"
```

---

### Task 6: Update main.rs for new API signatures and pagination

**Files:**
- Modify: `src/main.rs`

This is the biggest single file change. We update: API call sites for tuple returns, event handlers for cursors, add `n` key for pagination, and add quality picker handling.

- [ ] **Step 1: Update background poller (lines 70-102)**

In the tokio::spawn block inside `main()`, change the poll match arm from:

```rust
match api.get_streams(&poll_logins).await {
    Ok(channels) => {
        // ...
        let _ = tx_poll.send(AppEvent::ChannelsLoaded(channels));
    }
```

to:

```rust
match api.get_streams(&poll_logins, None).await {
    Ok((channels, _cursor)) => {
        // ... (keep existing live detection logic unchanged)
        let _ = tx_poll.send(AppEvent::ChannelsLoaded(channels, None));
    }
```

- [ ] **Step 2: Update event handler in run_app (lines 139-187)**

Replace the event match block with:

```rust
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
            debug!("Chat: {}: {}", msg.sender, msg.message);
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
```

- [ ] **Step 3: Add quality picker and pagination handling to handle_key**

In `handle_key`, add a block for `QualitySelect` mode right after the `if app.show_help` block and before the chat input mode block. Insert after the `return; }` on the help block (after line 211):

```rust
    // Quality picker mode
    if let AppMode::QualitySelect { quality_index, channel_name, channel_display_name } = &mut app.mode {
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
```

- [ ] **Step 4: Add 'n' key for pagination**

In the main key match block (the `match key.code` at line 292), add this arm before the `_ => {}` default:

```rust
KeyCode::Char('n') => {
    if app.pagination_cursor.is_some() {
        info!("Loading next page");
        app.is_loading = true;
        spawn_next_page(auth, tx, &app.mode, app.pagination_cursor.as_deref());
    }
}
```

- [ ] **Step 5: Update handle_enter to use quality picker**

Replace the `handle_enter` function with:

```rust
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
                app.reset_selection();
                app.pagination_cursor = None;
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
                connect_chat(auth, tx, &app.selected_channel().map(|c| c.name.clone()).unwrap_or_default(), irc_client, current_chat_channel);
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
```

Note: The `connect_chat` call in the `SavedChannels|Followed|CategoryStreams` arm needs the channel name before entering QualitySelect. We grab it from `selected_channel()` before setting the mode. Adjust the chat connect to use a clone captured before mode change:

```rust
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
```

- [ ] **Step 6: Update mode transitions to reset pagination_cursor**

In the key handler, add `app.pagination_cursor = None;` alongside every `app.reset_selection();` call. Specifically in these key handlers:
- `KeyCode::Char('c')` (categories)
- `KeyCode::Char('f')` (followed)
- `KeyCode::Char('/')` (search)
- `KeyCode::Char('v')` (vods)
- `KeyCode::Esc` (back to saved)

- [ ] **Step 7: Update spawn helper functions**

Replace `spawn_categories` with:

```rust
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
```

Replace `spawn_category_streams` with:

```rust
fn spawn_category_streams(
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    game_id: &str,
    after: Option<&str>,
) {
    let auth = auth.clone();
    let tx = tx.clone();
    let gid = game_id.to_string();
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        debug!("Fetching streams for game {}", gid);
        match api.get_streams_by_game(&gid, 20, after).await {
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
```

Replace `spawn_search` with:

```rust
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
```

Replace `spawn_vods` with:

```rust
fn spawn_vods(
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    user_id: &str,
    after: Option<&str>,
) {
    let auth = auth.clone();
    let tx = tx.clone();
    let uid = user_id.to_string();
    tokio::spawn(async move {
        let api = twitch::api::TwitchApi::new(auth);
        debug!("Fetching VODs for user {}", uid);
        match api.get_vods(&uid, 10, after).await {
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
```

Replace `spawn_followed` with (update the API call to tuple destructuring):

In the `spawn_followed` function, change:
- `api.get_streams(&poll_logins).await` → `api.get_streams(&logins, None).await`
- Destructure the result: `Ok((live, _cursor)) => { ... }`

Add a new function for pagination:

```rust
fn spawn_next_page(
    auth: &twitch::auth::Auth,
    tx: &mpsc::UnboundedSender<AppEvent>,
    mode: &AppMode,
    cursor: Option<&str>,
) {
    match mode {
        AppMode::SavedChannels | AppMode::Followed => {
            let logins: Vec<String> = Vec::new();
            let auth = auth.clone();
            let tx = tx.clone();
            let c = cursor.unwrap_or_default().to_string();
            tokio::spawn(async move {
                let api = twitch::api::TwitchApi::new(auth);
                match api.get_streams(&logins, Some(&c)).await {
                    Ok((channels, cursor)) => {
                        let _ = tx.send(AppEvent::ChannelsLoaded(channels, cursor));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::Error(format!("Pagination error: {}", e)));
                    }
                }
            });
        }
        AppMode::Categories => {
            spawn_categories_page(auth, tx, cursor);
        }
        AppMode::CategoryStreams { game_id, .. } => {
            let gid = game_id.clone();
            spawn_category_streams(auth, tx, &gid, cursor);
        }
        AppMode::Vods { .. } => {
            if let Some(ch) = app.selected_channel() {
                spawn_vods(auth, tx, &ch.twitch_id, cursor);
            }
        }
        _ => {}
    }
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
```

Note: `spawn_next_page` needs access to the current channel for VODs pagination. Since it doesn't have `app`, we need to pass the user_id. Update the `n` key handler to extract the needed info:

```rust
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
```

- [ ] **Step 8: Verify compilation**

Run: `cargo check 2>&1 | tail -10`
Expected: `Finished` with no errors.

- [ ] **Step 9: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire up pagination, quality picker, and updated API signatures"
```

---

### Task 7: Status bar UI

**Files:**
- Create: `src/ui/status.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Create status bar module**

Create `src/ui/status.rs`:

```rust
use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::Paragraph,
    Frame,
};
use super::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let mode_text = format!(" Mode: {} ", app.mode_label());
    let hints_text = format!(" {} ", app.key_hints());

    let para = Paragraph::new(format!("{:<width$}{}", mode_text, hints_text, width = area.width as usize / 2))
        .style(
            Style::default()
                .fg(theme::CYAN)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(para, area);
}
```

- [ ] **Step 2: Add module and update layout in ui/mod.rs**

Replace `src/ui/mod.rs` with:

```rust
pub mod browse;
pub mod chat;
pub mod detail;
pub mod format;
pub mod help;
pub mod quality;
pub mod status;
pub mod theme;

use crate::app::{App, AppMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::Paragraph,
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Top: status bar (1 line) + optional error bar (1 line)
    let error_height = if app.error_message.is_some() { 1 } else { 0 };
    let top_height = 1 + error_height;
    let main_area = Rect::new(size.x, size.y + top_height, size.width, size.height.saturating_sub(top_height));
    let status_area = Rect::new(size.x, size.y, size.width, 1);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(main_area);

    status::render(f, app, status_area);
    browse::render(f, app, chunks[0]);
    detail::render(f, app, chunks[1]);
    chat::render(f, app, chunks[2]);

    if let Some(err) = &app.error_message {
        let error_bar = Paragraph::new(err.clone())
            .style(Style::default().fg(theme::RED));
        let bar_area = Rect::new(size.x, size.y + 1, size.width, 1);
        f.render_widget(error_bar, bar_area);
    }

    if app.show_help {
        help::render(f, main_area);
    }

    if let AppMode::QualitySelect { .. } = &app.mode {
        quality::render(f, app, main_area);
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```bash
git add src/ui/status.rs src/ui/mod.rs
git commit -m "feat: add status bar with mode label and key hints"
```

---

### Task 8: Quality picker overlay

**Files:**
- Create: `src/ui/quality.rs`

- [ ] **Step 1: Create quality picker overlay**

Create `src/ui/quality.rs`:

```rust
use crate::app::{App, AppMode, QUALITY_OPTIONS};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use super::theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let (quality_index, channel_display_name) = match &app.mode {
        AppMode::QualitySelect { quality_index, channel_display_name, .. } => (*quality_index, channel_display_name.clone()),
        _ => return,
    };

    let overlay_width = 36u16;
    let overlay_height = (QUALITY_OPTIONS.len() as u16) + 4;
    let x = area.width.saturating_sub(overlay_width) / 2;
    let y = area.height.saturating_sub(overlay_height) / 2;
    let overlay_area = Rect::new(x, y, overlay_width, overlay_height);

    f.render_widget(Clear, overlay_area);

    let items: Vec<ListItem> = QUALITY_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, q)| {
            let prefix = if i == quality_index { "> " } else { "  " };
            let style = if i == quality_index {
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            ListItem::new(format!("{}{}", prefix, q)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Select Quality — {} ", channel_display_name))
            .border_style(Style::default().fg(theme::CYAN)),
    );

    let mut state = ListState::default();
    state.select(Some(quality_index));

    f.render_stateful_widget(list, overlay_area, &mut state);

    let hint_area = Rect::new(x, y + overlay_height, overlay_width, 1);
    let hint = Paragraph::new(" ↑↓ navigate · Enter select · Esc default")
        .style(Style::default().fg(theme::DIM_TEXT));
    f.render_widget(hint, hint_area);
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: `Finished`.

- [ ] **Step 3: Commit**

```bash
git add src/ui/quality.rs
git commit -m "feat: add quality picker overlay UI"
```

---

### Task 9: Browse panel polish

**Files:**
- Modify: `src/ui/browse.rs`

- [ ] **Step 1: Rewrite browse.rs with polished formatting**

Replace entire `src/ui/browse.rs` with:

```rust
use crate::app::{App, AppMode, FocusTarget};
use crate::ui::format::{format_uptime, format_viewers};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use super::theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == FocusTarget::Browse;
    let border_color = if focused { theme::CYAN } else { theme::BORDER };
    let mode = app.mode.clone();

    if app.is_loading {
        let loading = Paragraph::new(" ⏳ Loading...")
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
        AppMode::Followed => format!(" Following · {} channels · {} live ", total, live_count),
        _ => format!(" Saved · {} channels · {} live ", total, live_count),
    };

    let width = area.width.saturating_sub(2) as usize;
    let items: Vec<ListItem> = app
        .channels
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let style = if i == app.selected_index && focused {
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else if ch.is_live {
                Style::default().fg(theme::TEXT)
            } else {
                Style::default().fg(theme::DIM_TEXT)
            };
            let line = if ch.is_live {
                let uptime = ch.started_at.as_deref().map(format_uptime).unwrap_or_default();
                let game = ch.game_name.as_deref().unwrap_or("");
                let viewers = ch.viewer_count.map(format_viewers).unwrap_or_default();
                let name_w = 16.min(width);
                let game_w = (width as usize).saturating_sub(name_w + 8 + 8);
                let game_display = if game.len() > game_w && game_w > 3 {
                    format!("{}…", &game[..game_w.saturating_sub(1)])
                } else {
                    game.to_string()
                };
                format!("{:\u{2002}<name_w$} {:<8} {:<game_w$} {}", ch.display_name, uptime, game_display, viewers, name_w = name_w, game_w = game_w)
            } else {
                format!("{}  offline", ch.display_name)
            };
            ListItem::new(line).style(style)
        })
        .collect();

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

    render_pagination(f, app.pagination_cursor.is_some(), area, &mut state);
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
            .title(format!(" Categories · {} ", app.categories.len()))
            .border_style(Style::default().fg(border_color)),
    );

    let mut state = ListState::default();
    if focused && !app.categories.is_empty() {
        state.select(Some(app.selected_index));
    }
    f.render_stateful_widget(list, area, &mut state);

    render_pagination(f, app.pagination_cursor.is_some(), area, &mut state);
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

    let width = area.width.saturating_sub(2) as usize;
    let items: Vec<ListItem> = app
        .category_streams
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let style = if i == app.selected_index && focused {
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            let uptime = ch.started_at.as_deref().map(format_uptime).unwrap_or_default();
            let viewers = ch.viewer_count.map(format_viewers).unwrap_or_default();
            let name_w = 16.min(width);
            ListItem::new(format!("{:\u{2002}<name_w$} {:<8} {}", ch.display_name, uptime, viewers, name_w = name_w))
                .style(style)
        })
        .collect();

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

    render_pagination(f, app.pagination_cursor.is_some(), area, &mut state);
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
            let live = if ch.is_live {
                theme::LIVE_DOT
            } else {
                theme::OFFLINE_DOT
            };
            let style = if i == app.selected_index && focused {
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else if ch.is_live {
                Style::default().fg(theme::TEXT)
            } else {
                Style::default().fg(theme::DIM_TEXT)
            };
            ListItem::new(format!("{} {}", live, ch.display_name)).style(style)
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

    let items: Vec<ListItem> = app
        .vods
        .iter()
        .enumerate()
        .map(|(i, vod)| {
            let style = if i == app.selected_index && focused {
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            ListItem::new(format!(" [{}] {}", vod.duration, vod.title)).style(style)
        })
        .collect();

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

    render_pagination(f, app.pagination_cursor.is_some(), area, &mut state);
}

fn render_pagination(
    f: &mut Frame,
    has_more: bool,
    _area: Rect,
    _state: &mut ListState,
) {
    // The "press 'n' for more" prompt is shown as the last list item in render methods above.
    // This is a placeholder for potential future rendering.
    // For now, pagination is indicated in the status bar key hints.
    let _ = has_more;
}
```

Note: The `render_pagination` function is currently a no-op placeholder for the "press 'n' for more" visual. The actual text prompt will be appended as the last item in each list. Update the channel list items to append a pagination item. In `render_channels`, after building `items`, add:

```rust
    if app.pagination_cursor.is_some() {
        items.push(
            ListItem::new(" ── press 'n' for more ──")
                .style(Style::default().fg(theme::DIM_TEXT))
        );
    }
```

Do the same in `render_categories`, `render_category_streams`, and `render_vods` (but not `render_search` since search doesn't paginate).

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | tail -10`
Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add src/ui/browse.rs
git commit -m "feat: polished browse panel with formatted channels, headers, empty states"
```

---

### Task 10: Detail panel polish

**Files:**
- Modify: `src/ui/detail.rs`

- [ ] **Step 1: Rewrite detail.rs with structured layout and tags**

Replace entire `src/ui/detail.rs` with:

```rust
use crate::app::{App, AppMode};
use crate::ui::format::{format_uptime, format_viewers_full};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use super::theme;

const TAG_COLORS: &[ratatui::style::Color] = &[
    ratatui::style::Color::Rgb(0, 212, 255),
    ratatui::style::Color::Rgb(166, 227, 161),
    ratatui::style::Color::Rgb(243, 139, 168),
    ratatui::style::Color::Rgb(249, 226, 175),
];

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    // VOD mode: show VOD detail
    if let AppMode::Vods { .. } = &app.mode {
        render_vod_detail(f, app, area);
        return;
    }

    let channel = match app.selected_channel() {
        Some(ch) => ch,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Stream Info ")
                .style(Style::default().fg(theme::BORDER));
            let hint = Paragraph::new("Select a channel to see details")
                .block(block)
                .style(Style::default().fg(theme::DIM_TEXT));
            f.render_widget(hint, area);
            return;
        }
    };

    let mut lines: Vec<Line> = Vec::new();

    // Header line: name + live status
    if channel.is_live {
        let uptime = channel.started_at.as_deref().map(format_uptime).unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(theme::LIVE_DOT.to_string(), Style::default().fg(theme::RED)),
            Span::raw(" "),
            Span::styled(
                channel.display_name.clone(),
                Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("LIVE for {}", uptime),
                Style::default().fg(theme::GREEN),
            ),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(theme::OFFLINE_DOT.to_string(), Style::default().fg(theme::DIM_TEXT)),
            Span::raw(" "),
            Span::styled(
                channel.display_name.clone(),
                Style::default().fg(theme::DIM_TEXT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Offline", Style::default().fg(theme::DIM_TEXT)),
        ]));
    }

    // Separator
    let sep_width = area.width.saturating_sub(2) as usize;
    let sep: String = "━".repeat(sep_width);
    lines.push(Line::from(Span::styled(sep, Style::default().fg(theme::BORDER))));

    // Title
    if let Some(title) = &channel.title {
        lines.push(Line::from(Span::styled(
            title.clone(),
            Style::default().fg(theme::TEXT),
        )));
    }

    // Game
    if let Some(game) = &channel.game_name {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            game.clone(),
            Style::default().fg(theme::DIM_TEXT),
        )));
    }

    // Viewers
    if let Some(viewers) = channel.viewer_count {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("△ ", Style::default().fg(theme::GREEN)),
            Span::styled(
                format!("{} viewers", format_viewers_full(viewers)),
                Style::default().fg(theme::GREEN),
            ),
        ]));
    }

    // Tags
    if !channel.tags.is_empty() {
        lines.push(Line::from(""));
        let tag_spans: Vec<Span> = channel
            .tags
            .iter()
            .enumerate()
            .flat_map(|(i, tag)| {
                let color = TAG_COLORS[i % TAG_COLORS.len()];
                let mut spans = vec![
                    Span::styled(format!("[{}]", tag), Style::default().fg(color)),
                ];
                if i < channel.tags.len() - 1 {
                    spans.push(Span::raw(" "));
                }
                spans
            })
            .collect();
        lines.push(Line::from(tag_spans));
    }

    // Context hints at bottom
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[s] save · [Enter] watch · [v] vods",
        Style::default().fg(theme::DIM_TEXT),
    )));

    let text: Vec<Line> = lines;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Stream Info ")
        .style(Style::default().fg(theme::CYAN));
    let para = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

fn render_vod_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let vod = match app.vods.get(app.selected_index) {
        Some(v) => v,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" VOD Info ")
                .style(Style::default().fg(theme::BORDER));
            let hint = Paragraph::new("Select a VOD to see details")
                .block(block)
                .style(Style::default().fg(theme::DIM_TEXT));
            f.render_widget(hint, area);
            return;
        }
    };

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(
            vod.title.clone(),
            Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
        ),
    ]));

    let sep_width = area.width.saturating_sub(2) as usize;
    let sep: String = "━".repeat(sep_width);
    lines.push(Line::from(Span::styled(sep, Style::default().fg(theme::BORDER))));

    lines.push(Line::from(Span::styled(
        format!("Duration: {}", vod.duration),
        Style::default().fg(theme::GREEN),
    )));

    lines.push(Line::from(Span::styled(
        format!("Created: {}", vod.created_at),
        Style::default().fg(theme::DIM_TEXT),
    )));

    lines.push(Line::from(Span::styled(
        format!("Channel: {}", vod.user_name),
        Style::default().fg(theme::DIM_TEXT),
    )));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[Enter] play · [Esc] back",
        Style::default().fg(theme::DIM_TEXT),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" VOD Info ")
        .style(Style::default().fg(theme::CYAN));
    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | tail -10`
Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add src/ui/detail.rs
git commit -m "feat: polished detail panel with structured layout, tags, context hints"
```

---

### Task 11: Update help overlay

**Files:**
- Modify: `src/ui/help.rs`

- [ ] **Step 1: Update help text with new keybindings**

Replace the `help_text` string in `src/ui/help.rs` with:

```rust
    let help_text = "\
Navigation
  Tab/Shift+Tab  Switch pane
  j/k, Up/Down    Navigate list
  n               Load more results
  Esc             Back to saved channels

Actions
  Enter           Watch stream (quality picker)
  s               Save/unsave channel
  /               Search channels
  c               Categories view
  v               VODs (selected channel)
  f               Followed channels
  r               Refresh
  q               Quit

Chat pane
  Tab             Switch to chat
  type to compose, Enter to send
  Esc             Back to browse

Quality picker
  j/k or ↑↓      Select quality
  Enter           Confirm selection
  Esc             Use default quality

?               Toggle this help";
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | tail -5`
Expected: `Finished`.

- [ ] **Step 3: Commit**

```bash
git add src/ui/help.rs
git commit -m "feat: update help overlay with new keybindings"
```

---

### Task 12: Build and test

**Files:** None (verification only)

- [ ] **Step 1: Run full test suite**

Run: `cargo test 2>&1`
Expected: All tests pass (existing tests + new format tests).

- [ ] **Step 2: Build release**

Run: `cargo build --release 2>&1 | tail -5`
Expected: `Finished` with no errors.

- [ ] **Step 3: Final commit (if any fixes needed)**

If any compilation fixes were needed during the build:

```bash
git add -A
git commit -m "fix: compilation fixes for polish and features"
```
