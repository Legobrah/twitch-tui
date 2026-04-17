use crate::config::Config;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Auth {
    pub client_id: String,
    pub oauth_token: Option<String>,
    pub username: Option<String>,
}

impl Auth {
    pub fn from_config(config: &Config) -> Self {
        Self {
            client_id: config.twitch.client_id.clone(),
            oauth_token: config.twitch.oauth_token.clone(),
            username: config.twitch.username.clone(),
        }
    }

    pub fn has_token(&self) -> bool {
        self.oauth_token.is_some()
    }

    pub fn open_auth_page(&self) -> Result<(), Box<dyn std::error::Error>> {
        let scopes = "chat:read+chat:edit+user:read:follows";
        let url = format!(
            "https://id.twitch.tv/oauth2/authorize?response_type=token&client_id={}&redirect_uri=http://localhost&scope={}",
            self.client_id, scopes
        );
        Command::new("xdg-open").arg(&url).spawn()?;
        Ok(())
    }

    pub async fn validate_token(&self) -> Result<bool, String> {
        let token = self.oauth_token.as_ref().ok_or("No token set")?;
        let client = reqwest::Client::new();
        let resp = client
            .get("https://id.twitch.tv/oauth2/validate")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(resp.status().is_success())
    }
}
