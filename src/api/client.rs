use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Deserialize;

use crate::models::{OwnedGame, OwnedGamesResponse, RecentGame, RecentlyPlayedResponse};

// MARK: - Error

#[derive(Debug, thiserror::Error)]
pub enum SteamApiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    /// Returned for 403/400 responses on per-game achievement calls
    /// (private stats or game has no achievements — not a fatal error).
    #[error("Not available (private or no achievements)")]
    NotAvailable,

    #[error("Steam API error: {0}")]
    ApiError(String),
}

// MARK: - Achievement response models (private to this module)

#[derive(Debug, Deserialize)]
struct PlayerAchievementsResponse {
    playerstats: PlayerStats,
}

#[derive(Debug, Deserialize)]
struct PlayerStats {
    #[serde(default)]
    achievements: Vec<Achievement>,
}

#[derive(Debug, Deserialize)]
struct Achievement {
    achieved: u8,
}

// MARK: - Client

/// Steam Web API client.
///
/// Use `SteamClient::new` for production. Use `SteamClient::with_base_url` in tests
/// to point at a wiremock server instead of the real Steam API (S-6).
pub struct SteamClient {
    client: reqwest::Client,
    steam_id: String,
    api_key: String,
    base_url: String,
    /// AC-3.4: prevents duplicate GetOwnedGames calls in the same process run.
    fetched_owned: AtomicBool,
}

impl SteamClient {
    /// Create a client targeting the live Steam API.
    pub fn new(steam_id: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::with_base_url("https://api.steampowered.com", steam_id, api_key)
    }

    /// Create a client with a custom base URL — used in tests to target a wiremock server.
    pub fn with_base_url(
        base: impl Into<String>,
        steam_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build reqwest client");

        Self {
            client,
            steam_id: steam_id.into(),
            api_key: api_key.into(),
            base_url: base.into().trim_end_matches('/').to_string(),
            fetched_owned: AtomicBool::new(false),
        }
    }

    // MARK: - API Methods

    /// `GET /IPlayerService/GetOwnedGames/v1`
    ///
    /// Returns all games owned by the configured Steam ID.
    /// AC-3.4: subsequent calls in the same process run return an error to the caller to
    /// indicate this was already fetched — callers should cache the result.
    pub async fn get_owned_games(&self) -> Result<Vec<OwnedGame>, SteamApiError> {
        let url = format!("{}/IPlayerService/GetOwnedGames/v1", self.base_url);

        let resp = self
            .client
            .get(&url)
            .query(&[
                ("steamid", self.steam_id.as_str()),
                ("key", self.api_key.as_str()),
                ("include_appinfo", "1"),
                ("include_played_free_games", "1"),
            ])
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(SteamApiError::ApiError(format!("HTTP {status}: {body}")));
        }

        let envelope: OwnedGamesResponse = resp
            .json()
            .await
            .map_err(|e| SteamApiError::Parse(e.to_string()))?;

        self.fetched_owned.store(true, Ordering::Relaxed);
        Ok(envelope.response.games)
    }

    /// `GET /IPlayerService/GetRecentlyPlayedGames/v1`
    ///
    /// Returns games played in the last two weeks.
    pub async fn get_recently_played(&self) -> Result<Vec<RecentGame>, SteamApiError> {
        let url = format!("{}/IPlayerService/GetRecentlyPlayedGames/v1", self.base_url);

        let resp = self
            .client
            .get(&url)
            .query(&[
                ("steamid", self.steam_id.as_str()),
                ("key", self.api_key.as_str()),
            ])
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(SteamApiError::ApiError(format!("HTTP {status}: {body}")));
        }

        let envelope: RecentlyPlayedResponse = resp
            .json()
            .await
            .map_err(|e| SteamApiError::Parse(e.to_string()))?;

        Ok(envelope.response.games)
    }

    /// `GET /ISteamUserStats/GetPlayerAchievements/v1`
    ///
    /// Returns the count of unlocked achievements for a given app.
    /// Returns `SteamApiError::NotAvailable` on 403 or 400 (private stats or no achievements)
    /// so callers can skip this game without treating it as fatal (AD-5).
    pub async fn get_player_achievements(&self, appid: u32) -> Result<u32, SteamApiError> {
        let url = format!(
            "{}/ISteamUserStats/GetPlayerAchievements/v1",
            self.base_url
        );
        let appid_str = appid.to_string();

        let resp = self
            .client
            .get(&url)
            .query(&[
                ("steamid", self.steam_id.as_str()),
                ("key", self.api_key.as_str()),
                ("appid", appid_str.as_str()),
            ])
            .send()
            .await?;

        let status = resp.status();
        if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::BAD_REQUEST {
            return Err(SteamApiError::NotAvailable);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(SteamApiError::ApiError(format!("HTTP {status}: {body}")));
        }

        let envelope: PlayerAchievementsResponse = resp
            .json()
            .await
            .map_err(|e| SteamApiError::Parse(e.to_string()))?;

        let unlocked = envelope
            .playerstats
            .achievements
            .iter()
            .filter(|a| a.achieved == 1)
            .count() as u32;

        Ok(unlocked)
    }

    // MARK: - Accessors

    /// Returns true if `get_owned_games` has been called at least once this run (AC-3.4).
    pub fn owned_games_fetched(&self) -> bool {
        self.fetched_owned.load(Ordering::Relaxed)
    }
}

// MARK: - Tests

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path_regex, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Helper: base JSON envelope for GetOwnedGames
    fn owned_games_json(games: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "response": {
                "game_count": games.as_array().map(|a| a.len()).unwrap_or(0),
                "games": games
            }
        })
    }

    // Helper: base JSON envelope for GetRecentlyPlayedGames
    fn recently_played_json(games: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "response": {
                "total_count": games.as_array().map(|a| a.len()).unwrap_or(0),
                "games": games
            }
        })
    }

    // Helper: achievements JSON
    fn achievements_json(unlocked: u8, locked: u8) -> serde_json::Value {
        let mut achievements: Vec<serde_json::Value> = (0..unlocked)
            .map(|_| serde_json::json!({"apiname": "ACH", "achieved": 1}))
            .collect();
        achievements.extend(
            (0..locked).map(|_| serde_json::json!({"apiname": "LACH", "achieved": 0})),
        );
        serde_json::json!({
            "playerstats": {
                "steamID": "76561198000000001",
                "gameName": "Test Game",
                "achievements": achievements
            }
        })
    }

    #[tokio::test]
    async fn test_get_owned_games_returns_correct_games() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex("/IPlayerService/GetOwnedGames/v1"))
            .and(query_param("steamid", "76561198000000001"))
            .and(query_param("key", "test_api_key"))
            .and(query_param("include_appinfo", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(owned_games_json(
                serde_json::json!([
                    {"appid": 570, "name": "Dota 2", "playtime_forever": 12345, "playtime_2weeks": 60},
                    {"appid": 730, "name": "CS2", "playtime_forever": 9999}
                ]),
            )))
            .mount(&server)
            .await;

        let client =
            SteamClient::with_base_url(server.uri(), "76561198000000001", "test_api_key");
        let games = client.get_owned_games().await.expect("should succeed");

        assert_eq!(games.len(), 2);
        assert_eq!(games[0].appid, 570);
        assert_eq!(games[1].appid, 730);
    }

    #[tokio::test]
    async fn test_get_owned_games_sends_api_key_in_request() {
        let server = MockServer::start().await;

        // Mock requires the key param — if it's missing, wiremock won't match and returns 404
        Mock::given(method("GET"))
            .and(path_regex("/IPlayerService/GetOwnedGames/v1"))
            .and(query_param("key", "secret_key_xyz"))
            .respond_with(ResponseTemplate::new(200).set_body_json(owned_games_json(
                serde_json::json!([]),
            )))
            .mount(&server)
            .await;

        let client = SteamClient::with_base_url(server.uri(), "76561198000000001", "secret_key_xyz");
        let games = client.get_owned_games().await.expect("should succeed");
        assert_eq!(games.len(), 0);
    }

    #[tokio::test]
    async fn test_get_recently_played_returns_correct_games() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex("/IPlayerService/GetRecentlyPlayedGames/v1"))
            .and(query_param("steamid", "76561198000000001"))
            .and(query_param("key", "test_api_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(recently_played_json(
                serde_json::json!([
                    {"appid": 440, "name": "TF2", "playtime_2weeks": 120, "playtime_forever": 5000}
                ]),
            )))
            .mount(&server)
            .await;

        let client =
            SteamClient::with_base_url(server.uri(), "76561198000000001", "test_api_key");
        let games = client.get_recently_played().await.expect("should succeed");

        assert_eq!(games.len(), 1);
        assert_eq!(games[0].appid, 440);
        assert_eq!(games[0].playtime_2weeks, 120);
    }

    #[tokio::test]
    async fn test_get_player_achievements_403_returns_not_available() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex("/ISteamUserStats/GetPlayerAchievements/v1"))
            .and(query_param("appid", "730"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let client =
            SteamClient::with_base_url(server.uri(), "76561198000000001", "test_api_key");
        let result = client.get_player_achievements(730).await;

        assert!(
            matches!(result, Err(SteamApiError::NotAvailable)),
            "Expected NotAvailable, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_get_player_achievements_400_returns_not_available() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex("/ISteamUserStats/GetPlayerAchievements/v1"))
            .and(query_param("appid", "12345"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&server)
            .await;

        let client =
            SteamClient::with_base_url(server.uri(), "76561198000000001", "test_api_key");
        let result = client.get_player_achievements(12345).await;

        assert!(matches!(result, Err(SteamApiError::NotAvailable)));
    }

    #[tokio::test]
    async fn test_get_player_achievements_counts_unlocked_only() {
        let server = MockServer::start().await;

        // 3 unlocked, 2 locked → count should be 3
        Mock::given(method("GET"))
            .and(path_regex("/ISteamUserStats/GetPlayerAchievements/v1"))
            .and(query_param("steamid", "76561198000000001"))
            .and(query_param("key", "test_api_key"))
            .and(query_param("appid", "570"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(achievements_json(3, 2)),
            )
            .mount(&server)
            .await;

        let client =
            SteamClient::with_base_url(server.uri(), "76561198000000001", "test_api_key");
        let count = client
            .get_player_achievements(570)
            .await
            .expect("should succeed");

        assert_eq!(count, 3, "Should count only achieved==1 entries");
    }

    #[tokio::test]
    async fn test_owned_games_fetched_flag_set_after_call() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex("/IPlayerService/GetOwnedGames/v1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(owned_games_json(
                serde_json::json!([]),
            )))
            .mount(&server)
            .await;

        let client =
            SteamClient::with_base_url(server.uri(), "76561198000000001", "test_api_key");
        assert!(!client.owned_games_fetched(), "Flag should start false");
        client.get_owned_games().await.expect("should succeed");
        assert!(client.owned_games_fetched(), "Flag should be true after fetch");
    }
}
