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
        let oauth_token = config
            .twitch
            .oauth_token
            .as_deref()
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(|token| token.strip_prefix("oauth:").unwrap_or(token).to_string());
        let username = config
            .twitch
            .username
            .as_deref()
            .map(str::trim)
            .filter(|username| !username.is_empty())
            .map(ToOwned::to_owned);

        Self {
            client_id: config.twitch.client_id.trim().to_string(),
            oauth_token,
            username,
        }
    }

    pub fn has_token(&self) -> bool {
        self.oauth_token
            .as_deref()
            .is_some_and(|token| !token.trim().is_empty())
    }

    pub fn helix_token(&self) -> Option<&str> {
        self.oauth_token
            .as_deref()
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(|token| token.strip_prefix("oauth:").unwrap_or(token))
    }

    pub fn open_auth_page(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.client_id.chars().all(|c| c.is_alphanumeric()) {
            return Err("Invalid client ID".into());
        }
        let scopes = "chat:read+chat:edit+user:read:follows";
        let url = format!(
            "https://id.twitch.tv/oauth2/authorize?response_type=token&client_id={}&redirect_uri=http://localhost&scope={}",
            self.client_id, scopes
        );
        Command::new("xdg-open").arg(&url).spawn()?;
        Ok(())
    }

    pub async fn validate_token(&self) -> Result<bool, String> {
        let token = self.helix_token().ok_or("No token set")?;
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
