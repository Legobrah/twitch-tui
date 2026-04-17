use tokio::process::Command;

pub async fn watch_stream(channel: &str, quality: &str) -> Result<(), String> {
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
    let url = format!("https://twitch.tv/videos/{}", vod_id);
    Command::new("streamlink")
        .args([&url, quality, "--player", "mpv"])
        .spawn()
        .map_err(|e| format!("Failed to start streamlink: {}", e))?;
    Ok(())
}
