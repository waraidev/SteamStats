use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::models::stats::{GameStats, OverallStats, Period};

// ──────────────────────────────────────────────────────────────────────────────
// Data structures
// ──────────────────────────────────────────────────────────────────────────────

/// Per-game data stored in a snapshot line.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameEntry {
    /// Playtime in minutes (maps to `playtime_forever` from Steam API).
    pub pt: u64,
    /// Achievement count at time of snapshot, if available.
    pub ach: Option<u64>,
}

/// A single snapshot line in the NDJSON file.
///
/// Schema: `{"ts": u64, "games": {"<appid>": {"pt": u64, "ach": u64|null}}}`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    /// Unix timestamp (seconds UTC) when this snapshot was recorded.
    pub ts: u64,
    /// Map of Steam appid (string) → game entry.
    pub games: HashMap<String, GameEntry>,
}

// ──────────────────────────────────────────────────────────────────────────────
// SnapshotStore
// ──────────────────────────────────────────────────────────────────────────────

/// NDJSON snapshot store.
///
/// Writes are atomic: new content is written to a `.tmp` sibling then
/// `std::fs::rename`-d into place, so a crash between the two steps leaves
/// the main file untouched (AD-4).
pub struct SnapshotStore {
    path: PathBuf,
}

impl SnapshotStore {
    /// Production constructor: uses `dirs::data_dir()` with a fallback of `.`
    /// (S-4). Emits a warning to stderr if the platform returns no data dir.
    pub fn new() -> Self {
        let base = dirs::data_dir().unwrap_or_else(|| {
            eprintln!("[warn] dirs::data_dir() returned None; using current directory as fallback");
            PathBuf::from(".")
        });
        Self {
            path: base.join("steam-stats/snapshots.ndjson"),
        }
    }

    /// Test/override constructor: use an arbitrary path.
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    // ── I/O ──────────────────────────────────────────────────────────────────

    /// Load all snapshots from the NDJSON file.
    ///
    /// - Returns an empty `Vec` if the file does not exist (AC-4.5).
    /// - Malformed lines are skipped with a `[warn]` to stderr (AC-4.5).
    pub fn load(&self) -> Vec<Snapshot> {
        let content = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return vec![],
            Err(e) => {
                eprintln!("[warn] failed to read snapshot file: {e}");
                return vec![];
            }
        };

        let mut snapshots = Vec::new();
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<Snapshot>(trimmed) {
                Ok(s) => snapshots.push(s),
                Err(e) => {
                    eprintln!("[warn] skipping malformed snapshot line {}: {e}", i + 1);
                }
            }
        }
        snapshots
    }

    /// Atomically append a snapshot to the NDJSON file.
    ///
    /// Pattern (AD-4):
    /// 1. Create parent directory if needed.
    /// 2. Read existing file content (empty string if missing).
    /// 3. Append the new JSON line.
    /// 4. Write everything to `<path>.tmp`.
    /// 5. `std::fs::rename(<path>.tmp, <path>)`.
    pub fn append(&self, snapshot: &Snapshot) -> std::io::Result<()> {
        // Ensure parent exists.
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Read existing content (or start fresh).
        let existing = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e),
        };

        // Serialize the new snapshot.
        let new_line = serde_json::to_string(snapshot)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Build full file content.
        let mut content = existing;
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&new_line);
        content.push('\n');

        // Write to .tmp then rename atomically.
        let tmp_path = self.path.with_extension("ndjson.tmp");
        std::fs::write(&tmp_path, &content)?;
        std::fs::rename(&tmp_path, &self.path)?;

        Ok(())
    }

    // ── Delta computation ─────────────────────────────────────────────────────

    /// Compute per-game and overall stats for the given period.
    ///
    /// - Filters snapshots to the period window (`period.window_start_ts`).
    /// - Per-game delta = `max_pt - min_pt` within the window (AC-4.2).
    ///   This is `latest.pt - earliest.pt`, NOT `latest.pt - 0` — the boundary
    ///   condition is critical: a game with 1000 minutes at window start and
    ///   1050 at window end produces `playtime_delta_minutes = 50`.
    /// - `est_sessions` = distinct calendar days (UTC) where ≥1 game's playtime
    ///   increased in consecutive pairs (AC-4.3).
    /// - `new_games` = appids whose first appearance across ALL snapshots falls
    ///   within the window (AC-7.1).
    /// - `Period::Lifetime` returns all-time totals (no window filtering).
    pub fn delta_for_period(&self, period: &Period, now_ts: u64) -> (Vec<GameStats>, OverallStats) {
        let snapshots = self.load();
        Self::delta_from_snapshots(&snapshots, period, now_ts)
    }

    /// Compute period stats from an already-loaded snapshot slice.
    ///
    /// Extracted from `delta_for_period` so the App can recompute on period switch
    /// without re-reading from disk (VF-1 / AC-2.2, AC-2.3, AC-3.4).
    pub fn delta_from_snapshots(
        snapshots: &[Snapshot],
        period: &Period,
        now_ts: u64,
    ) -> (Vec<GameStats>, OverallStats) {
        let empty_stats = || OverallStats {
            games_played: 0,
            est_sessions: 0,
            achievements: None,
            new_games: 0,
            partial_data_note: None,
        };

        // For Lifetime, return zeroed overall stats (no deltas without boundary).
        if *period == Period::Lifetime {
            return (vec![], empty_stats());
        }

        let window_start = match period.window_start_ts(now_ts) {
            Some(ts) => ts,
            None => return (vec![], empty_stats()),
        };

        // Filter to snapshots within the window.
        let windowed: Vec<&Snapshot> = snapshots.iter().filter(|s| s.ts >= window_start).collect();

        if windowed.is_empty() {
            return (vec![], empty_stats());
        }

        // Build per-game min/max playtime across the windowed snapshots.
        let mut game_min: HashMap<&str, u64> = HashMap::new();
        let mut game_max: HashMap<&str, u64> = HashMap::new();

        for snap in &windowed {
            for (appid, entry) in &snap.games {
                let min = game_min.entry(appid.as_str()).or_insert(u64::MAX);
                if entry.pt < *min {
                    *min = entry.pt;
                }
                let max = game_max.entry(appid.as_str()).or_insert(0);
                if entry.pt > *max {
                    *max = entry.pt;
                }
            }
        }

        // Build GameStats vec — only games with non-zero delta.
        let mut game_stats: Vec<GameStats> = game_min
            .iter()
            .filter_map(|(appid, &min_pt)| {
                let max_pt = *game_max.get(appid)?;
                let delta = max_pt.saturating_sub(min_pt);
                if delta == 0 {
                    return None;
                }
                Some(GameStats {
                    appid: appid.parse::<u32>().unwrap_or(0),
                    name: appid.to_string(), // Name resolved upstream from API data
                    playtime_delta_minutes: delta,
                    playtime_forever_minutes: max_pt,
                    achievement_count: None,
                    rank: 0, // filled in below
                })
            })
            .collect();

        // Sort by delta descending, assign 1-based ranks.
        game_stats.sort_by(|a, b| b.playtime_delta_minutes.cmp(&a.playtime_delta_minutes));
        for (i, g) in game_stats.iter_mut().enumerate() {
            g.rank = i + 1;
        }

        let est_sessions = Self::session_estimate(&windowed);
        let new_games = Self::new_games_in_period(snapshots, window_start);

        let overall = OverallStats {
            games_played: game_stats.len() as u64,
            est_sessions,
            achievements: None,
            new_games,
            partial_data_note: None,
        };

        (game_stats, overall)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Count distinct calendar days (UTC) where ≥1 game's playtime increased
    /// between consecutive snapshot pairs (AC-4.3).
    ///
    /// Only looks at consecutive pairs in the provided slice; the slice should
    /// already be pre-filtered to the window.
    fn session_estimate(snapshots: &[&Snapshot]) -> u64 {
        if snapshots.len() < 2 {
            return 0;
        }

        let mut session_days: HashSet<NaiveDate> = HashSet::new();

        for pair in snapshots.windows(2) {
            let (earlier, later) = (pair[0], pair[1]);
            let mut any_increase = false;

            for (appid, later_entry) in &later.games {
                if let Some(earlier_entry) = earlier.games.get(appid) {
                    if later_entry.pt > earlier_entry.pt {
                        any_increase = true;
                        break;
                    }
                }
            }

            if any_increase {
                // Record the calendar date (UTC) of the *later* snapshot.
                if let Some(dt) = DateTime::<Utc>::from_timestamp(later.ts as i64, 0) {
                    session_days.insert(dt.date_naive());
                }
            }
        }

        session_days.len() as u64
    }

    /// Count appids whose first appearance across ALL snapshots has
    /// `ts >= window_start` (AC-7.1).
    fn new_games_in_period(all_snapshots: &[Snapshot], window_start: u64) -> u64 {
        // Find the earliest timestamp each appid was seen.
        let mut first_seen: HashMap<&str, u64> = HashMap::new();

        for snap in all_snapshots {
            for appid in snap.games.keys() {
                let entry = first_seen.entry(appid.as_str()).or_insert(snap.ts);
                if snap.ts < *entry {
                    *entry = snap.ts;
                }
            }
        }

        first_seen
            .values()
            .filter(|&&first_ts| first_ts >= window_start)
            .count() as u64
    }
}

impl Default for SnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests (STEP-13)
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_snapshot(ts: u64, games: &[(&str, u64)]) -> Snapshot {
        let mut game_map = HashMap::new();
        for (appid, pt) in games {
            game_map.insert(appid.to_string(), GameEntry { pt: *pt, ach: None });
        }
        Snapshot {
            ts,
            games: game_map,
        }
    }

    // ── append + load round-trip ──────────────────────────────────────────────

    #[test]
    fn test_append_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("snapshots.ndjson");
        let store = SnapshotStore::with_path(path);

        let snap = make_snapshot(1000, &[("123", 500)]);
        store.append(&snap).unwrap();

        let loaded = store.load();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].ts, 1000);
        assert_eq!(loaded[0].games["123"].pt, 500);
    }

    #[test]
    fn test_append_multiple_snapshots() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("snapshots.ndjson");
        let store = SnapshotStore::with_path(path);

        store.append(&make_snapshot(1000, &[("1", 100)])).unwrap();
        store.append(&make_snapshot(2000, &[("1", 150)])).unwrap();
        store.append(&make_snapshot(3000, &[("1", 200)])).unwrap();

        let loaded = store.load();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].ts, 1000);
        assert_eq!(loaded[1].ts, 2000);
        assert_eq!(loaded[2].ts, 3000);
    }

    // ── Atomicity: no .tmp file after successful write ────────────────────────

    #[test]
    fn test_append_atomicity_no_tmp_file_remains() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("snapshots.ndjson");
        let store = SnapshotStore::with_path(path.clone());

        let snap = make_snapshot(1000, &[("42", 99)]);
        store.append(&snap).unwrap();

        // The .tmp file must not exist after a successful write.
        let tmp = path.with_extension("ndjson.tmp");
        assert!(
            !tmp.exists(),
            ".tmp file should not remain after atomic write"
        );
    }

    // ── delta_for_period boundary condition ───────────────────────────────────

    #[test]
    fn test_delta_for_period_boundary_50_not_1050() {
        // Critical boundary: delta = latest.pt - earliest.pt, NOT latest.pt - 0
        let dir = tempdir().unwrap();
        let path = dir.path().join("snapshots.ndjson");
        let store = SnapshotStore::with_path(path);

        // now = 1746316800 (2026-05-04 UTC), window = FourWeeks (28 days back)
        let now: u64 = 1746316800;
        let window_start = now - 28 * 24 * 3600; // 2026-04-06

        // Both snapshots are within the window.
        let snap1 = make_snapshot(window_start + 100, &[("999", 1000)]);
        let snap2 = make_snapshot(window_start + 200, &[("999", 1050)]);
        store.append(&snap1).unwrap();
        store.append(&snap2).unwrap();

        let (games, overall) = store.delta_for_period(&Period::FourWeeks, now);
        assert_eq!(games.len(), 1);
        assert_eq!(
            games[0].playtime_delta_minutes, 50,
            "delta should be 50 (1050-1000), not 1050"
        );
        assert_eq!(overall.games_played, 1);
    }

    // ── session_estimate calendar deduplication ───────────────────────────────

    #[test]
    fn test_session_estimate_deduplicates_same_calendar_day() {
        // 3 snapshots: 2 on day A, 1 on day B — both show increases.
        // Day A should count as 1 session (not 2), so result = 2.

        // Day A: 2026-04-06 00:00:00 UTC = 1743897600
        // Day A later: 2026-04-06 12:00:00 UTC = 1743940800
        // Day B: 2026-04-07 00:00:00 UTC = 1743984000
        let day_a_early: u64 = 1743897600;
        let day_a_late: u64 = 1743940800;
        let day_b: u64 = 1743984000;

        let now: u64 = 1746316800;
        let window_start = now - 28 * 24 * 3600;

        let dir = tempdir().unwrap();
        let path = dir.path().join("snapshots.ndjson");
        let store = SnapshotStore::with_path(path);

        // All three within window, game increases at each step.
        store
            .append(&make_snapshot(window_start + 1, &[("1", 100)]))
            .unwrap();
        store
            .append(&make_snapshot(day_a_early, &[("1", 110)]))
            .unwrap();
        store
            .append(&make_snapshot(day_a_late, &[("1", 120)]))
            .unwrap();
        store.append(&make_snapshot(day_b, &[("1", 130)])).unwrap();

        let (_, overall) = store.delta_for_period(&Period::FourWeeks, now);
        assert_eq!(
            overall.est_sessions, 2,
            "3 snapshots on 2 days with increases → 2 sessions"
        );
    }

    // ── new_games_in_period ───────────────────────────────────────────────────

    #[test]
    fn test_new_games_in_period_counts_correctly() {
        let now: u64 = 1746316800;
        let window_start = now - 28 * 24 * 3600;

        let dir = tempdir().unwrap();
        let path = dir.path().join("snapshots.ndjson");
        let store = SnapshotStore::with_path(path);

        // Game "old" appeared before the window — should NOT be counted.
        // Game "new" first appears inside the window — SHOULD be counted.
        let before_window = window_start - 1000;
        let inside_window = window_start + 1000;

        store
            .append(&make_snapshot(before_window, &[("old", 50)]))
            .unwrap();
        store
            .append(&make_snapshot(inside_window, &[("old", 60), ("new", 10)]))
            .unwrap();
        store
            .append(&make_snapshot(
                inside_window + 1000,
                &[("old", 70), ("new", 20)],
            ))
            .unwrap();

        let (_, overall) = store.delta_for_period(&Period::FourWeeks, now);
        assert_eq!(overall.new_games, 1, "only 'new' game should be counted");
    }

    #[test]
    fn test_new_games_zero_when_none_in_period() {
        let now: u64 = 1746316800;
        let window_start = now - 28 * 24 * 3600;

        let dir = tempdir().unwrap();
        let path = dir.path().join("snapshots.ndjson");
        let store = SnapshotStore::with_path(path);

        // Both games appeared before the window.
        let before = window_start - 5000;
        store
            .append(&make_snapshot(before, &[("1", 100), ("2", 200)]))
            .unwrap();
        store
            .append(&make_snapshot(
                window_start + 1000,
                &[("1", 110), ("2", 210)],
            ))
            .unwrap();

        let (_, overall) = store.delta_for_period(&Period::FourWeeks, now);
        assert_eq!(overall.new_games, 0, "AC-7.2: no new games → shows 0");
    }

    // ── load with malformed line ──────────────────────────────────────────────

    #[test]
    fn test_load_skips_malformed_lines_without_panic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("snapshots.ndjson");

        // Write valid + malformed + valid lines manually.
        let snap1 = make_snapshot(1000, &[("1", 100)]);
        let snap2 = make_snapshot(3000, &[("1", 150)]);
        let valid1 = serde_json::to_string(&snap1).unwrap();
        let valid2 = serde_json::to_string(&snap2).unwrap();
        std::fs::write(
            &path,
            format!("{valid1}\nnot valid json at all\n{valid2}\n"),
        )
        .unwrap();

        let store = SnapshotStore::with_path(path);
        let loaded = store.load();

        // Should return the 2 valid snapshots — no panic, no Err.
        assert_eq!(loaded.len(), 2, "malformed line should be skipped");
        assert_eq!(loaded[0].ts, 1000);
        assert_eq!(loaded[1].ts, 3000);
    }

    // ── top_games sort order and 1-based ranks (VF-8) ────────────────────────

    #[test]
    fn test_delta_for_period_top_games_sorted_descending_with_1based_ranks() {
        let now: u64 = 1746316800;
        let window_start = now - 28 * 24 * 3600;

        let dir = tempdir().unwrap();
        let path = dir.path().join("snapshots.ndjson");
        let store = SnapshotStore::with_path(path);

        // Two snapshots: game A gains 90min, game B gains 30min, game C gains 60min.
        // Expected sort (desc): A(90) → C(60) → B(30), ranks 1, 2, 3.
        let snap1 = make_snapshot(window_start + 100, &[("A", 100), ("B", 200), ("C", 300)]);
        let snap2 = make_snapshot(window_start + 200, &[("A", 190), ("B", 230), ("C", 360)]);
        store.append(&snap1).unwrap();
        store.append(&snap2).unwrap();

        let (games, _) = store.delta_for_period(&Period::FourWeeks, now);

        assert_eq!(games.len(), 3, "all 3 games should appear");
        assert!(
            games[0].playtime_delta_minutes >= games[1].playtime_delta_minutes
                && games[1].playtime_delta_minutes >= games[2].playtime_delta_minutes,
            "top_games must be sorted descending by playtime_delta_minutes"
        );
        assert_eq!(games[0].rank, 1);
        assert_eq!(games[1].rank, 2);
        assert_eq!(games[2].rank, 3);
        // Spot-check the expected order by delta values.
        assert_eq!(
            games[0].playtime_delta_minutes, 90,
            "game A should be first"
        );
        assert_eq!(
            games[1].playtime_delta_minutes, 60,
            "game C should be second"
        );
        assert_eq!(
            games[2].playtime_delta_minutes, 30,
            "game B should be third"
        );
    }

    // ── load missing file → empty vec ─────────────────────────────────────────

    #[test]
    fn test_load_missing_file_returns_empty_vec() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("does_not_exist.ndjson");
        let store = SnapshotStore::with_path(path);
        let loaded = store.load();
        assert!(loaded.is_empty());
    }
}
