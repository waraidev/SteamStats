use serde::{Deserialize, Serialize};

// MARK: - Owned Games

/// Top-level envelope for the Steam GetOwnedGames API response.
/// All Steam API responses nest data under a "response" key (F-6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedGamesResponse {
    pub response: OwnedGamesBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedGamesBody {
    pub game_count: u32,
    pub games: Vec<OwnedGame>,
}

/// A single game from the GetOwnedGames API.
/// Field names match Steam's snake_case API (F-7).
/// Playtime values are in minutes (not hours).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OwnedGame {
    pub appid: u32,
    pub name: String,
    /// Total playtime in minutes across all time.
    pub playtime_forever: u64,
    /// Playtime in the last 2 weeks, in minutes. Not present if not played recently.
    #[serde(default)]
    pub playtime_2weeks: Option<u64>,
    /// Icon URL hash for building CDN image URLs.
    #[serde(default)]
    pub img_icon_url: Option<String>,
}

// MARK: - Recently Played Games

/// Top-level envelope for the Steam GetRecentlyPlayedGames API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentlyPlayedResponse {
    pub response: RecentlyPlayedBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentlyPlayedBody {
    pub total_count: u32,
    pub games: Vec<RecentGame>,
}

/// A single game from the GetRecentlyPlayedGames API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RecentGame {
    pub appid: u32,
    pub name: String,
    /// Playtime in the last 2 weeks, in minutes.
    pub playtime_2weeks: u64,
    /// Total playtime in minutes across all time.
    pub playtime_forever: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_games_response_deserializes_envelope() {
        let json = r#"{
            "response": {
                "game_count": 2,
                "games": [
                    {
                        "appid": 570,
                        "name": "Dota 2",
                        "playtime_forever": 12345,
                        "playtime_2weeks": 60,
                        "img_icon_url": "abc123"
                    },
                    {
                        "appid": 730,
                        "name": "CS:GO",
                        "playtime_forever": 9999
                    }
                ]
            }
        }"#;

        let resp: OwnedGamesResponse = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(resp.response.game_count, 2);
        assert_eq!(resp.response.games.len(), 2);
        // playtime is in minutes, not hours
        assert_eq!(resp.response.games[0].playtime_forever, 12345);
        assert_eq!(resp.response.games[0].appid, 570u32);
        assert_eq!(resp.response.games[0].playtime_2weeks, Some(60));
        assert_eq!(
            resp.response.games[0].img_icon_url.as_deref(),
            Some("abc123")
        );
    }

    #[test]
    fn owned_game_missing_optional_fields_is_none() {
        let json = r#"{
            "response": {
                "game_count": 1,
                "games": [
                    {
                        "appid": 730,
                        "name": "CS:GO",
                        "playtime_forever": 9999
                    }
                ]
            }
        }"#;

        let resp: OwnedGamesResponse = serde_json::from_str(json).expect("should deserialize");
        let game = &resp.response.games[0];
        assert!(
            game.playtime_2weeks.is_none(),
            "playtime_2weeks should be None when absent"
        );
        assert!(
            game.img_icon_url.is_none(),
            "img_icon_url should be None when absent"
        );
    }

    #[test]
    fn owned_game_appid_is_numeric_u32() {
        // Steam returns appids as numeric JSON integers, not strings
        let json = r#"{
            "response": {
                "game_count": 1,
                "games": [{ "appid": 12345, "name": "Test", "playtime_forever": 0 }]
            }
        }"#;

        let resp: OwnedGamesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response.games[0].appid, 12345u32);
    }

    #[test]
    fn recently_played_response_deserializes_envelope() {
        let json = r#"{
            "response": {
                "total_count": 1,
                "games": [
                    {
                        "appid": 570,
                        "name": "Dota 2",
                        "playtime_2weeks": 300,
                        "playtime_forever": 50000
                    }
                ]
            }
        }"#;

        let resp: RecentlyPlayedResponse = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(resp.response.total_count, 1);
        assert_eq!(resp.response.games.len(), 1);
        assert_eq!(resp.response.games[0].playtime_2weeks, 300);
        assert_eq!(resp.response.games[0].playtime_forever, 50000);
    }
}
