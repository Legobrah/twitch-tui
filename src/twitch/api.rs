use crate::twitch::auth::Auth;
use crate::twitch::{Channel, Game, Vod};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug)]
pub struct TwitchApi {
    client: Client,
    auth: Auth,
    base_url: String,
}

#[derive(Deserialize)]
struct StreamsResponse {
    data: Vec<StreamData>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct StreamData {
    user_id: String,
    user_login: String,
    user_name: String,
    title: String,
    game_name: String,
    viewer_count: u32,
    started_at: String,
    thumbnail_url: String,
}

#[derive(Deserialize)]
struct GamesResponse {
    data: Vec<GameData>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct GameData {
    id: String,
    name: String,
    box_art_url: Option<String>,
}

#[derive(Deserialize)]
struct SearchChannelsResponse {
    data: Vec<SearchChannelData>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct SearchChannelData {
    id: String,
    login: String,
    display_name: String,
    is_live: bool,
}

#[derive(Deserialize)]
struct VideosResponse {
    data: Vec<VideoData>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct VideoData {
    id: String,
    title: String,
    duration: String,
    created_at: String,
    thumbnail_url: String,
    user_name: String,
}

impl TwitchApi {
    pub fn new(auth: Auth) -> Self {
        Self {
            client: Client::new(),
            auth,
            base_url: "https://api.twitch.tv/helix".to_string(),
        }
    }

    fn build_headers(&self) -> Result<reqwest::header::HeaderMap, String> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Client-Id",
            self.auth
                .client_id
                .parse()
                .map_err(|_| "Invalid client ID")?,
        );
        if let Some(token) = &self.auth.oauth_token {
            headers.insert(
                "Authorization",
                format!("Bearer {}", token)
                    .parse()
                    .map_err(|_| "Invalid token")?,
            );
        }
        Ok(headers)
    }

    pub async fn get_streams(&self, user_logins: &[String]) -> Result<Vec<Channel>, String> {
        let headers = self.build_headers()?;
        let params: Vec<String> = user_logins
            .iter()
            .map(|l| format!("user_login={}", l))
            .collect();
        let url = format!("{}/streams?{}", self.base_url, params.join("&"));

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }

        let data: StreamsResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        Ok(data
            .data
            .into_iter()
            .map(|s| Channel {
                twitch_id: s.user_id,
                name: s.user_login,
                display_name: s.user_name,
                is_live: true,
                title: Some(s.title),
                game_name: if s.game_name.is_empty() {
                    None
                } else {
                    Some(s.game_name)
                },
                viewer_count: Some(s.viewer_count),
                started_at: Some(s.started_at),
                thumbnail_url: Some(s.thumbnail_url),
            })
            .collect())
    }

    pub async fn get_top_games(&self, first: u32) -> Result<Vec<Game>, String> {
        let headers = self.build_headers()?;
        let url = format!("{}/games/top?first={}", self.base_url, first);

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let data: GamesResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        Ok(data
            .data
            .into_iter()
            .map(|g| Game {
                id: g.id,
                name: g.name,
                box_art_url: g.box_art_url,
            })
            .collect())
    }

    pub async fn get_streams_by_game(
        &self,
        game_id: &str,
        first: u32,
    ) -> Result<Vec<Channel>, String> {
        let headers = self.build_headers()?;
        let url = format!(
            "{}/streams?game_id={}&first={}",
            self.base_url, game_id, first
        );

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let data: StreamsResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        Ok(data
            .data
            .into_iter()
            .map(|s| Channel {
                twitch_id: s.user_id,
                name: s.user_login,
                display_name: s.user_name,
                is_live: true,
                title: Some(s.title),
                game_name: if s.game_name.is_empty() {
                    None
                } else {
                    Some(s.game_name)
                },
                viewer_count: Some(s.viewer_count),
                started_at: Some(s.started_at),
                thumbnail_url: Some(s.thumbnail_url),
            })
            .collect())
    }

    pub async fn search_channels(
        &self,
        query: &str,
        first: u32,
    ) -> Result<Vec<Channel>, String> {
        let headers = self.build_headers()?;
        let url = format!(
            "{}/search/channels?query={}&first={}",
            self.base_url, query, first
        );

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let data: SearchChannelsResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        Ok(data
            .data
            .into_iter()
            .map(|c| Channel {
                twitch_id: c.id,
                name: c.login,
                display_name: c.display_name,
                is_live: c.is_live,
                title: None,
                game_name: None,
                viewer_count: None,
                started_at: None,
                thumbnail_url: None,
            })
            .collect())
    }

    pub async fn get_vods(&self, user_id: &str, first: u32) -> Result<Vec<Vod>, String> {
        let headers = self.build_headers()?;
        let url = format!(
            "{}/videos?user_id={}&first={}",
            self.base_url, user_id, first
        );

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let data: VideosResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        Ok(data
            .data
            .into_iter()
            .map(|v| Vod {
                id: v.id,
                title: v.title,
                duration: v.duration,
                created_at: v.created_at,
                thumbnail_url: v.thumbnail_url,
                user_name: v.user_name,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_api() -> TwitchApi {
        TwitchApi::new(Auth {
            client_id: "test".to_string(),
            oauth_token: Some("test_token".to_string()),
            username: Some("testuser".to_string()),
        })
    }

    #[test]
    fn test_build_headers() {
        let api = test_api();
        let headers = api.build_headers().unwrap();
        assert_eq!(headers.get("Client-Id").unwrap(), "test");
        assert_eq!(
            headers.get("Authorization").unwrap(),
            "Bearer test_token"
        );
    }

    #[test]
    fn test_build_headers_no_token() {
        let api = TwitchApi::new(Auth {
            client_id: "test".to_string(),
            oauth_token: None,
            username: None,
        });
        let headers = api.build_headers().unwrap();
        assert_eq!(headers.get("Client-Id").unwrap(), "test");
        assert!(headers.get("Authorization").is_none());
    }
}
