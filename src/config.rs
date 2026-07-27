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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

impl TwitchConfig {
    /// Deserialize username, treating empty strings as None.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref().filter(|s| !s.is_empty())
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

fn merge_toml_tables(dst: &mut toml::value::Table, src: toml::value::Table) {
    for (key, value) in src {
        match (dst.get_mut(&key), value) {
            (Some(toml::Value::Table(dst_table)), toml::Value::Table(src_table)) => {
                merge_toml_tables(dst_table, src_table);
            }
            (_, value) => {
                dst.insert(key, value);
            }
        }
    }
}

fn remove_twitch_key(value: &mut toml::Value, key: &str) {
    if let toml::Value::Table(root) = value
        && let Some(toml::Value::Table(twitch)) = root.get_mut("twitch")
    {
        twitch.remove(key);
    }
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

        let mut value = if path.exists() {
            toml::from_str::<toml::Value>(&fs::read_to_string(path)?)?
        } else {
            toml::Value::Table(toml::value::Table::new())
        };
        let new_value = toml::Value::try_from(self)?;
        match (&mut value, new_value) {
            (toml::Value::Table(existing), toml::Value::Table(new_table)) => {
                merge_toml_tables(existing, new_table);
            }
            (existing, new_value) => {
                *existing = new_value;
            }
        }
        if self.twitch.oauth_token.is_none() {
            remove_twitch_key(&mut value, "oauth_token");
        }
        if self.twitch.username.is_none() {
            remove_twitch_key(&mut value, "username");
        }

        let content = toml::to_string_pretty(&value)?;
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

    #[test]
    fn test_save_preserves_unknown_keys() {
        let dir =
            std::env::temp_dir().join(format!("twitch-tui-config-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r#"
poll_interval_secs = 5
unknown_top_level = "keep"

[twitch]
client_id = "old"
custom_twitch_key = "keep"
oauth_token = "remove-me"
"#,
        )
        .unwrap();

        let config = Config {
            twitch: TwitchConfig {
                client_id: "new".to_string(),
                oauth_token: None,
                username: Some("devind".to_string()),
            },
            poll_interval_secs: 30,
            default_quality: "720p".to_string(),
            notifications_enabled: false,
            chat_enabled: false,
        };
        config.save_to(&path).unwrap();

        let value: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["unknown_top_level"].as_str(), Some("keep"));
        assert_eq!(value["twitch"]["custom_twitch_key"].as_str(), Some("keep"));
        assert_eq!(value["twitch"]["client_id"].as_str(), Some("new"));
        assert!(value["twitch"].get("oauth_token").is_none());
        assert_eq!(value["twitch"]["username"].as_str(), Some("devind"));
        let _ = fs::remove_file(path);
        let _ = fs::remove_dir(dir);
    }
}
