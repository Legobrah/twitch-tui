use tokio::process::Command;

pub async fn send_notification(title: &str, body: &str) -> Result<(), String> {
    Command::new("notify-send")
        .args(["-a", "twitch-tui", title, body])
        .spawn()
        .map_err(|e| format!("Failed to send notification: {}", e))?;
    Ok(())
}
