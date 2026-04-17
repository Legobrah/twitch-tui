# Twitch-TUI Polish & Features Design

## Goal

Polish the visual presentation of the browse and detail panels, add UX improvements (status bar, loading/empty states), and implement quality picker, tags display, and pagination.

## Scope

Visual polish, UX improvements, and new features. No chat changes.

---

## 1. Status Bar

A persistent 1-line bar rendered above the three-panel layout.

**Left side:** Current mode name — `Following`, `Saved Channels`, `Categories`, `Category: VALORANT`, `Search`, `VODs: pokimane`

**Right side:** Context-sensitive key hints that change based on mode and focus:
- Default: `j/k navigate · Enter watch · s save · ? help`
- Search: `type to search · Esc back · Enter watch`
- Categories: `Enter select category · Esc back`
- Chat focused: `type message · Enter send · Esc back`
- VODs: `Enter play · Esc back`

**Layout change:** Current layout is three panels filling the screen. New layout splits a 1-line status bar off the top, then the three panels fill the remainder.

**Files:** `ui/mod.rs` (layout change), `ui/status.rs` (new file)

---

## 2. Browse Panel Polish

**Current:** Flat list with `● name` or `○ name`.

**Polished live channel format:**
```
● shroud  2h 15m  VALORANT         23.4k
```

Components:
- Red dot `●` for live, hollow `○` for offline
- Uptime as relative time (`47m`, `2h 15m`, `1d 3h`)
- Game name in dim text, fixed-width column truncated with `…`
- Viewer count right-aligned, green, abbreviated (`23.4k` not `23400`)
- Selected item: cyan highlight with left border accent
- Offline channels: dimmed, just name + "offline"

**Panel header** shows context:
- `Following · 6 channels · 4 live`
- `Category: VALORANT · 20 streams`
- `Search: "valorant" · 15 results`
- `VODs: pokimane · 10 videos`

**Uptime formatter:** New helper function `format_uptime(started_at: &str) -> String` parses ISO 8601 timestamp, computes diff from now, returns human-readable string.

**Viewer count formatter:** New helper `format_viewers(count: u32) -> String` — under 1000 as-is, 1k-999k as `XX.Xk`, 1m+ as `X.Xm`.

**Files:** `ui/browse.rs`, `ui/format.rs` (new file for helpers)

---

## 3. Detail Panel Polish

**Current:** Flat text with raw ISO timestamp.

**Polished structure:**
```
● pokimane          LIVE for 4h 02m
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
RANKED DAY!! !socials

League of Legends

△ 42,391 viewers

[English] [Competitive] [Champion]

[s] save · [Enter] watch · [v] vods
```

Elements:
- Name in bold, live duration in green next to it
- Separator line using `━` characters in dim cyan
- Title on its own line
- Game name in dim text
- Viewer count with `△` icon, green color
- Tags as colored pill badges (tinted background + colored text)
- Context hints at bottom showing available actions

For offline channels: name + "Offline" in dim text, no separator.

For VODs: show title, duration, creation date, channel name.

**Files:** `ui/detail.rs`, `ui/format.rs` (uptime formatter reused)

---

## 4. Quality Picker

When Enter is pressed on a stream, show a centered popup overlay listing quality options:

```
╭─ Select Quality ───────────────╮
│  > best                        │
│    1080p                       │
│    720p                        │
│    480p                        │
│    360p                        │
│    audio_only                  │
│                                │
│  ↑↓ navigate · Enter select ·  │
│  Esc use default (best)        │
╰────────────────────────────────╯
```

**Behavior:**
- j/k or arrows navigate, Enter selects, Esc dismisses and uses config default
- Quality options: `["best", "1080p", "720p", "480p", "360p", "audio_only"]`
- Dimmed background behind overlay
- Default selection is config's `default_quality`

**New state:**
```rust
AppMode::QualitySelect {
    channel_name: String,
    channel_display_name: String,
    quality_index: usize,
}
```

On quality selection, spawn `player::watch_stream` with chosen quality.

**Files:** `app.rs` (new mode), `ui/quality.rs` (new file), `main.rs` (handle Enter for quality select)

---

## 5. Tags & Metadata

**API changes:**
- Add `tags: Vec<String>` to `StreamData` deserialization
- Add `tags: Vec<String>` to `Channel` struct
- Map tags from API response into Channel in all stream-fetching methods

The Twitch Helix `/streams` endpoint returns `tags` as an array of strings directly — no extra API calls needed.

**Display:** Tags shown as colored pill badges in detail panel only. Each tag gets a pill with:
- Background: tinted color at low opacity
- Text: full color
- Colors cycle through the theme palette: cyan, green, pink

**Files:** `twitch/api.rs`, `twitch/mod.rs`, `ui/detail.rs`

---

## 6. Pagination

**Current:** Hardcoded `first=20`. No way to see more results.

**New behavior:**
- After last item in list: `── press 'n' for more ──`
- Press `n` fetches next page using Twitch `after` cursor
- Results append to existing list
- When no cursor returned (end of results), hide the "more" prompt

**API changes:**
- All paginated methods return `(Vec<T>, Option<String>)` — data + cursor
- Methods accept optional `after: Option<&str>` parameter
- Cursor passed as `after` query param to Twitch API

**App state changes:**
```rust
pagination_cursor: Option<String>,
```

**AppEvent changes:**
```rust
ChannelsLoaded(Vec<Channel>, Option<String>),
CategoriesLoaded(Vec<Game>, Option<String>),
CategoryStreamsLoaded(Vec<Channel>, Option<String>),
SearchResults(Vec<Channel>, Option<String>),
VodsLoaded(Vec<Vod>, Option<String>),
```

Event handlers append data instead of replacing when paginating.

**Key binding:** `n` triggers next page fetch using stored cursor and current mode.

Applies to: followed channels, category streams, search results, VODs, categories.

**Files:** `twitch/api.rs`, `app.rs`, `main.rs`

---

## 7. Loading & Empty States

**Loading:**
- When `is_loading` is true, show `⏳ Loading...` centered in the affected panel
- Dim cyan color, simple and non-animated
- Replaces the list content while loading

**Empty states:**
- No saved channels: `No saved channels. Press / to search or f to browse followed.`
- No search results: `No results for "query"`
- No VODs: `No VODs available for this channel`
- Categories empty: `Failed to load categories`

Each empty state is a centered Paragraph with helpful message + hint in dim text.

**Files:** `ui/browse.rs`, `ui/detail.rs`

---

## File Summary

| File | Action | Purpose |
|------|--------|---------|
| `ui/status.rs` | Create | Status bar rendering |
| `ui/quality.rs` | Create | Quality picker overlay |
| `ui/format.rs` | Create | Uptime + viewer count formatters |
| `ui/mod.rs` | Modify | Add status bar to layout, render quality overlay |
| `ui/browse.rs` | Modify | Polished channel format, headers, empty/loading states, pagination prompt |
| `ui/detail.rs` | Modify | Structured layout, tags, context hints, empty states |
| `app.rs` | Modify | QualitySelect mode, pagination_cursor, updated AppEvents |
| `twitch/mod.rs` | Modify | Add tags to Channel |
| `twitch/api.rs` | Modify | Tags deserialization, pagination (cursor params + returns) |
| `main.rs` | Modify | Quality picker key handling, pagination key binding, updated event handlers |
