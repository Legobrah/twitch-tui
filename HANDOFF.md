# twitch-tui — Polish & Improvement Handoff

Context for GLM (or any agent) to continue this review/polish pass.

## Project summary

Rust TUI app using `ratatui` + `crossterm` + `tokio`. Three-pane layout:
- Browse (left 30%) — saved/followed/search/categories/VODs
- Detail (center 35%) — stream/VOD info
- Chat (right 35%) — Twitch IRC

Status bar top row, error bar optional under it, help overlay + quality overlay modal.

## File map

```
src/
  main.rs      — entry + event loop + key dispatch + async spawns (805 lines)
  app.rs       — App state, AppMode, FocusTarget, AppEvent enum
  config.rs    — TOML config in ~/.config/twitch-tui/config.toml
  db.rs        — rusqlite, saved_channels table
  notify.rs    — notify-send wrapper (9 lines)
  player.rs    — spawn streamlink + mpv (validates channel names)
  twitch/
    api.rs     — Helix REST client (472 lines)
    auth.rs    — Auth struct w/ oauth_token + username
    irc.rs     — twitch-irc wrapper, anon + authed connects
    mod.rs     — Channel / Game / Vod / ChatMessage structs
  ui/
    mod.rs     — Frame dispatcher, three-pane layout
    theme.rs   — Color constants (cyan/dark palette)
    status.rs  — Top status bar
    browse.rs  — List rendering per mode (473 lines)
    detail.rs  — Channel/VOD detail pane
    chat.rs    — Message list + input + word wrap
    help.rs    — Help overlay (scrollable)
    quality.rs — Quality picker overlay
    format.rs  — format_uptime, format_viewers (has tests)
```

## Current git state (at handoff start)

`master` has 25b4467 as tip. Uncommitted working changes in:
`app.rs, config.rs, main.rs, ui/{browse,chat,detail,help,mod,quality,status,theme}.rs`
(~467 insertions, ~235 deletions — user already did recent polish pass)

Recent commits show a focused UI polish arc: structured detail panel, colored chat, browse formatting, error auto-dismiss, security hardening (reqwest query params, file perms).

## Theme palette (src/ui/theme.rs)

```
CYAN     #00d4ff   focus/accent
BG       #060816   background (near-black navy)
SURFACE  #0e1122   pill/panel surface
TEXT     #cdd6f4
DIM_TEXT #9399b2
RED      #f38ba8   live dot / errors
GREEN    #a6e3a1   viewer counts / ok
YELLOW   #f9e2af   game names
BORDER   #1e283c   unfocused borders
LIVE_DOT '●'  OFFLINE_DOT '○'  POINTER '▸'
```

Font: JetBrainsMono Nerd Font. All new widgets must match.

## Known bugs / gaps found during review

### HIGH priority
1. **`r` refresh is a stub** — `main.rs:458-461` sets `is_loading=true` but never fires a fetch. Must dispatch per mode.
2. **Chat never clears on channel switch** — old channel's messages carry into new channel. Need to clear `app.chat_messages` + reconnect when `connect_chat` actually changes channel.
3. **Search debouncing absent** — every keystroke in `AppMode::Search` fires `spawn_search` → API hammering. Add debounce (250–400ms) via a `pending_search_token`.
4. **Chat auto-scroll uses item index not line index** — `chat.rs:162-166` uses `items.len()-1`, which works for single-line msgs but loses precision for wrapped. Good enough; revisit only if visible glitch.
5. **Help scroll unbounded** — `main.rs:259` uses `saturating_add(1)`, scrolls past content. Cap at `(total_lines - viewport).max(0)`.

### MEDIUM
6. **Followed mode lives in `app.channels`** — fine semantically, but refresh logic + `mode_label` treat them distinct; careful with `n` pagination (currently hits `get_streams` with saved_channels logins, not followed list — `main.rs:469-480`). Bug when paginating Followed.
7. **Search Enter watches immediately** — no quality picker path for search results (`main.rs:351-360`). Inconsistent with Saved/Followed Enter behavior.
8. **No jump keys** — `gg`/`G`, PgUp/PgDn, Ctrl-D/U missing.
9. **Status bar lacks auth status** — would be nice: show `● authed as <user>` or `○ anon` on right side.
10. **Quality picker hint position** — `quality.rs:56` renders at `y + overlay_height` which may overflow in small terminals. No bounds check.
11. **Self-mention highlight** — chat should color/bold lines mentioning `config.twitch.username`.

### LOW / nice-to-have
12. Loading spinner is static `⟳` — animate frames.
13. Chat timestamps (toggle via config).
14. No "clear all saved" / bulk ops.
15. Help overlay isn't a centered card, just fills main_area. Could be a centered modal.
16. VODs: after Enter, no "now playing" feedback.
17. Auto-load next page when selection hits bottom (instead of requiring `n`).
18. `SELECTION_BG` is hardcoded in `browse.rs:12` — should live in `theme.rs`.
19. Error bar is thin (1 row); short messages OK but multi-line errors truncate.

## Key design invariants (don't break)

- **Caveman mode is ACTIVE** in this session; but write normal English in code/comments.
- **All widgets must match the cyan/dark theme**; no new colors outside `theme.rs` constants.
- **Config at `~/.config/twitch-tui/config.toml`** is user-editable. Don't clobber unknown keys.
- **File perms 0o600** for log + config (already enforced).
- **Player validates channel names** (alphanumeric + `_`) — preserve this to prevent command injection via crafted channel names.
- **No amending commits** — new commits only (per user's repo workflow).
- **Event loop tick**: `crossterm::event::poll(100ms)`; draw every iteration. Keep work in `handle_key` + `run_app` non-blocking — spawn tokio tasks for I/O.

## How to apply / test changes

```fish
cd ~/Projects/twitch-tui
cargo check                 # fast sanity
cargo test                  # format.rs + config.rs have tests
cargo run                   # needs config.toml with client_id
```

For UI visual testing: user runs it in a Kitty terminal on Hyprland. Cannot render TUI from agent — must reason from code + user feedback.

Logs: `$XDG_DATA_HOME/twitch-tui/twitch-tui.log` (0o600).

## Thumbnail support (added in this session)

- `src/thumb.rs` owns the cache + debounced fetch (150 ms) + LRU of 32.
- Cache key: `{twitch_id}@{started_at}` — new broadcast invalidates old image.
- Picker built via `Picker::from_query_stdio()` *before* `enable_raw_mode`; fallback is `None` → no thumbnails rendered.
- Fetch resolves Twitch's `{width}`/`{height}` placeholders to 360×202.
- `AppEvent::ThumbnailReady(key, DynamicImage)` — main thread constructs the `StatefulProtocol` via `picker.new_resize_protocol()` since it's not Send-friendly.
- Renders in top 12 rows of detail pane; only when channel is live + picker ok + cache hit + pane tall enough.
- Crates added: `ratatui-image = "4"`, `image = "0.25"` with `jpeg` feature only.
- Tested on Kitty (CachyOS). On non-Kitty terminals `ratatui-image` falls back to halfblocks automatically.

## Planned polish pass (this session)

Targets (in order):
1. Fix `r` refresh — dispatch real refetch per mode.
2. Clear chat on channel change in `connect_chat`.
3. Debounce search.
4. Bound help scroll.
5. Fix Followed pagination.
6. Auth status in status bar.
7. Self-mention highlight in chat.
8. `g`/`G` jump top/bottom in lists.
9. PgUp / PgDn half-page nav.
10. Move `SELECTION_BG` to theme.
11. Quality picker hint bounds-check.

Stretch:
- Animated loading spinner (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` braille frames cycling on Tick).
- Chat timestamps (opt-in via config).
- Auto-load on bottom-scroll.
- Centered help modal.

## What NOT to touch without user sign-off

- `twitch/api.rs` endpoint surface (works, recently hardened)
- `Cargo.toml` deps (pinned, no need to add more for polish)
- `db.rs` schema (migrations dir is empty; don't introduce migrations now)
- OAuth flow (no built-in flow; user sets token manually — don't add oauth server)

## After polish: commit style

Recent commits follow: `type: short imperative summary`
- feat, fix, refactor prefixes
- lowercase, no trailing period
- one-liner subject, no body unless truly needed

Example: `fix: refresh key refetches current mode`
