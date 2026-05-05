use chrono::Datelike;
use serde::{Deserialize, Serialize};

/// Label for the estimated sessions stat — spec constraint: must be exactly this string.
/// All UI code must reference this constant rather than hardcoding the label.
pub const LABEL_EST_SESSIONS: &str = "Est. Sessions";

/// The time periods available for dashboard filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Period {
    FourWeeks,
    SixMonths,
    ThisYear,
    Lifetime,
}

impl Period {
    /// Returns the Unix timestamp (seconds) for the start of this period relative to `now`,
    /// or `None` for `Lifetime` (no window boundary — use all data).
    ///
    /// AC-2.3: Lifetime bypasses delta computation and uses `playtime_forever` directly.
    pub fn window_start_ts(&self, now: u64) -> Option<u64> {
        match self {
            Period::FourWeeks => Some(now.saturating_sub(28 * 24 * 3600)),
            Period::SixMonths => Some(now.saturating_sub(182 * 24 * 3600)),
            Period::ThisYear => {
                // Jan 1 of the UTC year containing `now`, at midnight UTC
                let dt = chrono::DateTime::from_timestamp(now as i64, 0)?;
                let jan1 = chrono::NaiveDate::from_ymd_opt(dt.year(), 1, 1)?
                    .and_hms_opt(0, 0, 0)?;
                let ts = jan1.and_utc().timestamp();
                Some(ts as u64)
            }
            Period::Lifetime => None,
        }
    }

    /// Human-readable label shown in the UI period selector.
    pub fn label(&self) -> &str {
        match self {
            Period::FourWeeks => "4 Weeks",
            Period::SixMonths => "6 Months",
            Period::ThisYear => "This Year",
            Period::Lifetime => "Lifetime",
        }
    }
}

/// Per-game statistics for the selected period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStats {
    pub appid: u32,
    pub name: String,
    /// Minutes played during the selected period (delta between snapshots).
    pub playtime_delta_minutes: u64,
    /// All-time minutes played (from `playtime_forever`).
    pub playtime_forever_minutes: u64,
    /// Achievement count for this game, if fetched.
    #[serde(default)]
    pub achievement_count: Option<u64>,
    /// Rank by playtime_delta within the current period (1-based).
    pub rank: usize,
}

/// Aggregated stats shown on the dashboard for the selected period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallStats {
    /// Number of games with non-zero playtime in the period.
    pub games_played: u64,
    /// Estimated sessions: distinct calendar days where playtime increased (AC-4.3).
    /// Must be labeled "Est. Sessions" in the UI — use `LABEL_EST_SESSIONS`.
    pub est_sessions: u64,
    /// Total achievements unlocked during the period. None if not yet fetched.
    #[serde(default)]
    pub achievements: Option<u64>,
    /// Count of games whose first cached playtime falls within the period (AC-7.1).
    pub new_games: u64,
    /// "Based on N days of data" note for AC-2.4. None means no note displayed.
    #[serde(default)]
    pub partial_data_note: Option<String>,
}

// NOTE: compute_delta will be implemented in cache/snapshot.rs (later bundle).
// Signature for reference:
//   pub fn compute_delta(
//       snapshots: &[crate::cache::snapshot::Snapshot],
//       period: Period,
//       now: u64,
//   ) -> (Vec<GameStats>, OverallStats)
