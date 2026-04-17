# twitch-tui

Terminal TUI for browsing, watching, and chatting on Twitch streams.

## Features

- Browse saved channels with live status
- Browse top categories and game streams
- Search channels
- Watch streams via streamlink + mpv (ad-free)
- Twitch IRC chat (anonymous or authenticated)
- VOD browsing and playback
- Desktop notifications when saved channels go live
- Followed channels view (requires OAuth)

## Requirements

- Rust 1.85+
- [streamlink](https://streamlink.github.io/) + [mpv](https://mpv.io/) for playback
- notify-send for desktop notifications

```bash
sudo pacman -S streamlink mpv libnotify
```

## Install

```bash
git clone https://github.com/Legobrah/twitch-tui.git
cd twitch-tui
cargo build --release
./target/release/twitch-tui
```

## Configuration

Config lives at `~/.config/twitch-tui/config.toml`:

```toml
[twitch]
client_id = "your-twitch-app-client-id"
oauth_token = "your-oauth-token"
username = "your-twitch-username"

poll_interval_secs = 60
default_quality = "best"
notifications_enabled = true
chat_enabled = true
```

### Getting OAuth credentials

1. Register an app at [Twitch Developer Console](https://dev.twitch.tv/console)
2. Set `client_id` in config
3. Run the app — press `f` to view followed channels, which will prompt for OAuth
4. Or manually get a token from: `https://id.twitch.tv/oauth2/authorize?response_type=token&client_id=YOUR_ID&redirect_uri=http://localhost&scope=chat:read+chat:edit+user:read:follows`

OAuth is optional. Without it, you can still browse, watch, and use anonymous chat.

## Keybinds

| Key | Action |
|-----|--------|
| Tab / Shift+Tab | Switch pane focus |
| j/k or Up/Down | Navigate list |
| Enter | Watch stream |
| s | Save/unsave channel |
| / | Search |
| c | Categories view |
| v | VODs for selected channel |
| f | Followed channels (requires OAuth) |
| r | Refresh |
| ? | Help overlay |
| q | Quit |

In chat pane: type to compose, Enter to send, Esc to leave.

## License

MIT
