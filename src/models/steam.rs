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
