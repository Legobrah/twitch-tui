use tokio::process::Command;

fn is_valid_channel_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

pub async fn watch_stream(channel: &str, quality: &str) -> Result<(), String> {
    if !is_valid_channel_name(channel) {
        return Err(format!("Invalid channel name: {}", channel));
    }
    let url = format!("https://twitch.tv/{}", channel);
    Command::new("streamlink")
        .args([&url, quality, "--player", "mpv"])
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "streamlink not found. Install: sudo pacman -S streamlink mpv".to_string()
            } else {
                format!("Failed to start streamlink: {}", e)
            }
        })?;
    Ok(())
}

pub async fn watch_vod(vod_id: &str, quality: &str) -> Result<(), String> {
    if vod_id.is_empty() || !vod_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("Invalid VOD ID: {}", vod_id));
    }
    let url = format!("https://twitch.tv/videos/{}", vod_id);
    Command::new("streamlink")
        .args([&url, quality, "--player", "mpv"])
        .spawn()
        .map_err(|e| format!("Failed to start streamlink: {}", e))?;
    Ok(())
}
