use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub twitch: TwitchConfig,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_quality")]
    pub default_quality: String,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    #[serde(default = "default_true")]
    pub chat_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TwitchConfig {
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub oauth_token: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            twitch: TwitchConfig::default(),
            poll_interval_secs: 60,
            default_quality: "best".to_string(),
            notifications_enabled: true,
            chat_enabled: true,
        }
    }
}

impl Default for TwitchConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            oauth_token: None,
            username: None,
        }
    }
}

fn default_poll_interval() -> u64 {
    60
}
fn default_quality() -> String {
    "best".to_string()
}
fn default_true() -> bool {
    true
}

impl Config {
    pub fn config_dir() -> PathBuf {
        ProjectDirs::from("", "", "twitch-tui")
            .expect("Could not determine config directory")
            .config_dir()
            .to_path_buf()
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_dir().join("config.toml");
        if !path.exists() {
            return Self::create_default(&path);
        }
        let content = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    fn create_default(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Config::default();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        config.save_to(path)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_dir().join("config.toml");
        self.save_to(&path)
    }

    fn save_to(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.poll_interval_secs, 60);
        assert_eq!(config.default_quality, "best");
        assert!(config.notifications_enabled);
        assert!(config.chat_enabled);
        assert!(config.twitch.client_id.is_empty());
        assert!(config.twitch.oauth_token.is_none());
    }

    #[test]
    fn test_config_roundtrip() {
        let config = Config {
            twitch: TwitchConfig {
                client_id: "test_id".to_string(),
                oauth_token: Some("test_token".to_string()),
                username: Some("testuser".to_string()),
            },
            poll_interval_secs: 30,
            default_quality: "720p".to_string(),
            notifications_enabled: false,
            chat_enabled: true,
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.twitch.client_id, "test_id");
        assert_eq!(parsed.twitch.oauth_token, Some("test_token".to_string()));
        assert_eq!(parsed.poll_interval_secs, 30);
        assert_eq!(parsed.default_quality, "720p");
        assert!(!parsed.notifications_enabled);
    }
}
