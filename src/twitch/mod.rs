pub mod api;
pub mod auth;
pub mod irc;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    pub twitch_id: String,
    pub name: String,
    pub display_name: String,
    #[serde(default)]
    pub is_live: bool,
    pub title: Option<String>,
    pub game_name: Option<String>,
    pub viewer_count: Option<u32>,
    pub started_at: Option<String>,
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub box_art_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Vod {
    pub id: String,
    pub title: String,
    pub duration: String,
    pub created_at: String,
    pub thumbnail_url: String,
    pub user_name: String,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub message: String,
    pub system: bool,
}
