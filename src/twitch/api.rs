use crate::twitch::auth::Auth;
use crate::twitch::{Channel, Game, Vod};
use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use tracing::{debug, error, warn};

#[derive(Debug)]
pub struct TwitchApi {
    client: Client,
    auth: Auth,
    base_url: String,
}

// Twitch Helix search requires a valid OAuth bearer token. The TUI still needs
// search to work when a saved user token expires, so we fall back to the same
// public GraphQL endpoint/client id used by Twitch's web UI for channel search.
const TWITCH_WEB_CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";

#[derive(Deserialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    pagination: Option<Pagination>,
}

#[derive(Deserialize)]
struct Pagination {
    cursor: Option<String>,
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
    #[serde(default)]
    tags: Vec<String>,
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
    broadcaster_login: String,
    display_name: String,
    is_live: bool,
    title: Option<String>,
    game_name: Option<String>,
    thumbnail_url: Option<String>,
    started_at: Option<String>,
}

#[derive(Deserialize)]
struct GraphQlResponse<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphQlError>,
}

#[derive(Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Deserialize)]
struct GraphQlSearchData {
    #[serde(rename = "searchFor")]
    search_for: Option<GraphQlSearchFor>,
}

#[derive(Deserialize)]
struct GraphQlSearchFor {
    channels: GraphQlSearchChannels,
}

#[derive(Deserialize)]
struct GraphQlSearchChannels {
    items: Vec<GraphQlSearchChannel>,
}

#[derive(Deserialize)]
struct GraphQlSearchChannel {
    id: String,
    login: String,
    #[serde(rename = "displayName")]
    display_name: String,
    stream: Option<GraphQlSearchStream>,
}

#[derive(Deserialize)]
struct GraphQlSearchStream {
    title: Option<String>,
    #[serde(rename = "viewersCount")]
    viewers_count: Option<u32>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    game: Option<GraphQlSearchGame>,
    #[serde(rename = "previewImageURL")]
    preview_image_url: Option<String>,
}

#[derive(Deserialize)]
struct GraphQlSearchGame {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    name: Option<String>,
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
        if let Some(token) = self.auth.helix_token() {
            headers.insert(
                "Authorization",
                format!("Bearer {}", token)
                    .parse()
                    .map_err(|_| "Invalid token")?,
            );
        }
        Ok(headers)
    }

    async fn parse_json_response<T: DeserializeOwned>(
        resp: reqwest::Response,
        context: &str,
    ) -> Result<T, String> {
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            error!("{} API error {}: {}", context, status, body);
            return Err(format!("{} API error {}: {}", context, status, body));
        }

        resp.json()
            .await
            .map_err(|e| format!("{} parse error: {}", context, e))
    }

    pub async fn get_streams(
        &self,
        user_logins: &[String],
        after: Option<&str>,
    ) -> Result<(Vec<Channel>, Option<String>), String> {
        let headers = self.build_headers().map_err(|e| {
            error!("build_headers error: {}", e);
            e
        })?;
        let mut query: Vec<(&str, String)> = user_logins
            .iter()
            .map(|l| ("user_login", l.clone()))
            .collect();
        if let Some(cursor) = after {
            query.push(("after", cursor.to_string()));
        }

        let resp = self
            .client
            .get(format!("{}/streams", self.base_url))
            .query(&query)
            .headers(headers)
            .send()
            .await
            .map_err(|e| {
                error!("Network error in get_streams: {}", e);
                format!("Network error: {}", e)
            })?;

        debug!("get_streams response status: {}", resp.status());
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!("get_streams API error {}: {}", status, body);
            return Err(format!("API error {}: {}", status, body));
        }

        let raw = resp
            .text()
            .await
            .map_err(|e| format!("Read body error: {}", e))?;
        debug!(
            "get_streams response body (first 500 chars): {}",
            &raw[..500.min(raw.len())]
        );
        let data: PaginatedResponse<StreamData> = serde_json::from_str(&raw).map_err(|e| {
            error!("Parse error in get_streams: {}", e);
            format!("Parse error: {}", e)
        })?;

        let cursor = data.pagination.and_then(|p| p.cursor);
        let channels = data
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
                tags: s.tags,
            })
            .collect();
        Ok((channels, cursor))
    }

    pub async fn get_top_games(
        &self,
        first: u32,
        after: Option<&str>,
    ) -> Result<(Vec<Game>, Option<String>), String> {
        let headers = self.build_headers()?;
        let mut query: Vec<(&str, String)> = vec![("first", first.to_string())];
        if let Some(cursor) = after {
            query.push(("after", cursor.to_string()));
        }

        let resp = self
            .client
            .get(format!("{}/games/top", self.base_url))
            .query(&query)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let data: PaginatedResponse<GameData> =
            Self::parse_json_response(resp, "Top games").await?;

        let cursor = data.pagination.and_then(|p| p.cursor);
        let games = data
            .data
            .into_iter()
            .map(|g| Game {
                id: g.id,
                name: g.name,
                box_art_url: g.box_art_url,
            })
            .collect();
        Ok((games, cursor))
    }

    pub async fn get_streams_by_game(
        &self,
        game_id: &str,
        first: u32,
        after: Option<&str>,
    ) -> Result<(Vec<Channel>, Option<String>), String> {
        let headers = self.build_headers()?;
        let mut query: Vec<(&str, String)> = vec![
            ("game_id", game_id.to_string()),
            ("first", first.to_string()),
        ];
        if let Some(cursor) = after {
            query.push(("after", cursor.to_string()));
        }

        let resp = self
            .client
            .get(format!("{}/streams", self.base_url))
            .query(&query)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let data: PaginatedResponse<StreamData> =
            Self::parse_json_response(resp, "Streams by game").await?;

        let cursor = data.pagination.and_then(|p| p.cursor);
        let channels = data
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
                tags: s.tags,
            })
            .collect();
        Ok((channels, cursor))
    }

    pub async fn search_channels(&self, query: &str, first: u32) -> Result<Vec<Channel>, String> {
        match self.search_channels_helix(query, first).await {
            Ok(results) => Ok(results),
            Err(err) if Self::is_auth_error(&err) => {
                warn!("Helix search auth failed; retrying with Twitch web search fallback");
                self.search_channels_graphql(query, first)
                    .await
                    .map_err(|fallback_err| {
                        format!(
                            "Search failed because the OAuth token is invalid or expired, and the fallback search also failed: {}",
                            fallback_err
                        )
                    })
            }
            Err(err) => Err(err),
        }
    }

    async fn search_channels_helix(&self, query: &str, first: u32) -> Result<Vec<Channel>, String> {
        let headers = self.build_headers()?;
        let query_params: Vec<(&str, String)> =
            vec![("query", query.to_string()), ("first", first.to_string())];

        let resp = self
            .client
            .get(format!("{}/search/channels", self.base_url))
            .query(&query_params)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let data: SearchChannelsResponse =
            Self::parse_json_response(resp, "Search channels").await?;

        Ok(data
            .data
            .into_iter()
            .map(|c| Channel {
                twitch_id: c.id,
                name: c.broadcaster_login,
                display_name: c.display_name,
                is_live: c.is_live,
                title: c.title,
                game_name: c.game_name,
                viewer_count: None,
                started_at: c.started_at,
                thumbnail_url: c.thumbnail_url,
                tags: Vec::new(),
            })
            .collect())
    }

    fn is_auth_error(error: &str) -> bool {
        error.contains("401 Unauthorized")
            || error.contains("\"status\":401")
            || error.contains("OAuth token")
    }

    async fn search_channels_graphql(
        &self,
        query: &str,
        first: u32,
    ) -> Result<Vec<Channel>, String> {
        const QUERY: &str = r#"
query Search_Query($noQuery: Boolean!, $platform: String!, $target: SearchForTarget, $userQuery: String!) {
  searchFor(userQuery: $userQuery, platform: $platform, target: $target) @skip(if: $noQuery) {
    channels {
      items {
        id
        login
        displayName
        stream {
          title
          viewersCount
          createdAt
          game {
            displayName
            name
          }
          previewImageURL(width: 440, height: 248)
        }
      }
    }
  }
}
"#;

        let payload = serde_json::json!({
            "query": QUERY,
            "variables": {
                "noQuery": query.trim().is_empty(),
                "platform": "mobile_web",
                "target": null,
                "userQuery": query,
            },
        });

        let resp = self
            .client
            .post("https://gql.twitch.tv/gql")
            .header("Client-Id", TWITCH_WEB_CLIENT_ID)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            error!("Search fallback API error {}: {}", status, body);
            return Err(format!("Search fallback API error {}: {}", status, body));
        }

        let response: GraphQlResponse<GraphQlSearchData> = resp
            .json()
            .await
            .map_err(|e| format!("Search fallback parse error: {}", e))?;

        if !response.errors.is_empty() {
            let messages = response
                .errors
                .into_iter()
                .map(|err| err.message)
                .collect::<Vec<_>>()
                .join("; ");
            error!("Search fallback GraphQL error: {}", messages);
            return Err(format!("Search fallback GraphQL error: {}", messages));
        }

        let items = response
            .data
            .and_then(|data| data.search_for)
            .map(|search| search.channels.items)
            .unwrap_or_default();

        Ok(Self::graphql_items_to_channels(items, first))
    }

    fn graphql_items_to_channels(items: Vec<GraphQlSearchChannel>, first: u32) -> Vec<Channel> {
        let limit = usize::try_from(first).unwrap_or(usize::MAX);
        items
            .into_iter()
            .take(limit)
            .map(|c| {
                let (is_live, title, game_name, viewer_count, started_at, thumbnail_url) =
                    match c.stream {
                        Some(stream) => {
                            let game_name =
                                stream.game.and_then(|game| game.display_name.or(game.name));
                            (
                                true,
                                stream.title,
                                game_name,
                                stream.viewers_count,
                                stream.created_at,
                                stream.preview_image_url,
                            )
                        }
                        None => (false, None, None, None, None, None),
                    };

                Channel {
                    twitch_id: c.id,
                    name: c.login,
                    display_name: c.display_name,
                    is_live,
                    title,
                    game_name,
                    viewer_count,
                    started_at,
                    thumbnail_url,
                    tags: Vec::new(),
                }
            })
            .collect()
    }

    pub async fn get_vods(
        &self,
        user_id: &str,
        first: u32,
        after: Option<&str>,
    ) -> Result<(Vec<Vod>, Option<String>), String> {
        let headers = self.build_headers()?;
        let mut query: Vec<(&str, String)> = vec![
            ("user_id", user_id.to_string()),
            ("first", first.to_string()),
        ];
        if let Some(cursor) = after {
            query.push(("after", cursor.to_string()));
        }

        let resp = self
            .client
            .get(format!("{}/videos", self.base_url))
            .query(&query)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        let data: PaginatedResponse<VideoData> = Self::parse_json_response(resp, "VODs").await?;

        let cursor = data.pagination.and_then(|p| p.cursor);
        let vods = data
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
            .collect();
        Ok((vods, cursor))
    }

    pub async fn get_followed_channels(&self, user_id: &str) -> Result<Vec<Channel>, String> {
        let headers = self.build_headers()?;
        let query: Vec<(&str, String)> = vec![
            ("user_id", user_id.to_string()),
            ("first", "100".to_string()),
        ];

        let resp = self
            .client
            .get(format!("{}/channels/followed", self.base_url))
            .query(&query)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        #[derive(Deserialize)]
        struct FollowedResponse {
            data: Vec<FollowedData>,
        }
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct FollowedData {
            broadcaster_id: String,
            broadcaster_login: String,
            broadcaster_name: String,
        }

        let data: FollowedResponse = Self::parse_json_response(resp, "Followed channels").await?;

        Ok(data
            .data
            .into_iter()
            .map(|f| Channel {
                twitch_id: f.broadcaster_id,
                name: f.broadcaster_login,
                display_name: f.broadcaster_name,
                is_live: false,
                title: None,
                game_name: None,
                viewer_count: None,
                started_at: None,
                thumbnail_url: None,
                tags: Vec::new(),
            })
            .collect())
    }

    pub async fn get_current_user(&self) -> Result<UserInfo, String> {
        let headers = self.build_headers()?;

        let resp = self
            .client
            .get(format!("{}/users", self.base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        #[derive(Deserialize)]
        struct UsersResponse {
            data: Vec<UserData>,
        }
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct UserData {
            id: String,
            login: String,
            display_name: String,
        }

        let data: UsersResponse = Self::parse_json_response(resp, "Current user").await?;

        data.data
            .into_iter()
            .next()
            .map(|u| UserInfo {
                id: u.id,
                login: u.login,
                display_name: u.display_name,
            })
            .ok_or_else(|| "No user data returned".to_string())
    }
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: String,
    pub login: String,
    pub display_name: String,
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
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer test_token");
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

    #[test]
    fn test_detects_search_auth_errors() {
        assert!(TwitchApi::is_auth_error(
            "Search channels API error 401 Unauthorized: {\"status\":401,\"message\":\"Invalid OAuth token\"}"
        ));
        assert!(TwitchApi::is_auth_error("OAuth token is missing"));
        assert!(!TwitchApi::is_auth_error("Network error: timed out"));
    }

    #[test]
    fn test_maps_graphql_search_items_to_channels() {
        let raw = r#"
{
  "data": {
    "searchFor": {
      "channels": {
        "items": [
          {
            "id": "121336528",
            "login": "burna",
            "displayName": "burna",
            "stream": {
              "title": "BIT HOT",
              "viewersCount": 67,
              "createdAt": "2026-07-08T15:56:55Z",
              "game": { "displayName": "Just Chatting", "name": "Just Chatting" },
              "previewImageURL": "https://static-cdn.jtvnw.net/previews-ttv/live_user_burna-440x248.jpg"
            }
          },
          {
            "id": "53430798",
            "login": "burn",
            "displayName": "Burn",
            "stream": null
          }
        ]
      }
    }
  }
}
"#;
        let response: GraphQlResponse<GraphQlSearchData> = serde_json::from_str(raw).unwrap();
        let items = response
            .data
            .and_then(|data| data.search_for)
            .map(|search| search.channels.items)
            .unwrap();

        let channels = TwitchApi::graphql_items_to_channels(items, 20);

        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].twitch_id, "121336528");
        assert_eq!(channels[0].name, "burna");
        assert!(channels[0].is_live);
        assert_eq!(channels[0].title.as_deref(), Some("BIT HOT"));
        assert_eq!(channels[0].game_name.as_deref(), Some("Just Chatting"));
        assert_eq!(channels[0].viewer_count, Some(67));
        assert_eq!(
            channels[0].started_at.as_deref(),
            Some("2026-07-08T15:56:55Z")
        );
        assert!(
            channels[0]
                .thumbnail_url
                .as_deref()
                .is_some_and(|url| url.contains("live_user_burna"))
        );
        assert!(!channels[1].is_live);
    }
}
