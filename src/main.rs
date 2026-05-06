pub mod api;
pub mod cache;
pub mod config;
pub mod models;
pub mod tui;

use std::collections::HashMap;

use tokio::sync::mpsc;

use api::client::{SteamApiError, SteamClient};
use cache::snapshot::{Snapshot, SnapshotStore};
use config::{default_config_path, Config, ConfigError};
use models::steam::OwnedGame;
use tui::app::{run_tui, App, AppEvent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── 1. Config ──────────────────────────────────────────────────────────────
    // Config::load() / prompt_and_save() MUST run before run_tui() calls
    // enable_raw_mode() — raw mode swallows stdin input (config constraint / AD-6).
    let config = match Config::load() {
        Ok(c) => c,
        Err(ConfigError::NotFound) => Config::prompt_and_save(&default_config_path())?,
        Err(e) => return Err(e.into()),
    };

    // ── 2. Snapshot store ──────────────────────────────────────────────────────
    // Load synchronously (sub-ms for expected file sizes) before starting TUI.
    // Capture prior snapshots BEFORE the new fetch so achievement fan-out can
    // compare playtime against the last known values (AD-5).
    let store = SnapshotStore::new();
    let prior_snapshots = store.load();

    // ── 3. Channel + App ──────────────────────────────────────────────────────
    let (tx, rx) = mpsc::unbounded_channel::<AppEvent>();

    let mut app = App::with_tx(tx.clone());
    app.config = Some(config.clone());

    // ── 4. Background data fetch task ─────────────────────────────────────────
    // Spawned before run_tui() so the first render already has cached data
    // (NFR-1: <500ms to first render — fetch runs concurrently with TUI startup).
    let tx_fetch = tx.clone();

    tokio::spawn(async move {
        let client = SteamClient::new(&config.steam_id, &config.steam_api_key);

        // Fetch owned games.
        let owned = match client.get_owned_games().await {
            Ok(games) => games,
            Err(e) => {
                let _ = tx_fetch.send(AppEvent::ApiError(format!("Failed to fetch games: {e}")));
                return;
            }
        };

        let recent = match client.get_recently_played().await {
            Ok(games) => games,
            Err(e) => {
                // Recently played is non-fatal — report via channel so the TUI status bar
                // can show it without writing to stderr (which would corrupt the TUI display).
                let _ = tx_fetch.send(AppEvent::StatusMessage(format!(
                    "Recently played unavailable: {e}"
                )));
                vec![]
            }
        };

        // Persist new snapshot (includes fresh playtime values).
        let snapshot = build_snapshot(&owned);
        if let Err(e) = store.append(&snapshot) {
            let _ = tx_fetch.send(AppEvent::StatusMessage(format!(
                "Snapshot save failed: {e}"
            )));
        }

        // Signal UI that base data is ready, including updated snapshots for period stats (VF-1).
        let snapshots = store.load();
        let _ = tx_fetch.send(AppEvent::DataLoaded {
            owned: owned.clone(),
            recent,
            snapshots,
        });

        // ── 5. Achievement fan-out task (AD-5) ────────────────────────────────
        // Determine recently-active appids: those in recently_played OR whose
        // playtime_forever increased vs the prior snapshot.
        let appids_to_fetch: Vec<u32> = if prior_snapshots.is_empty() {
            // First launch: fetch all games in batches of 10.
            owned.iter().map(|g| g.appid).collect()
        } else {
            determine_active_appids(&owned, &prior_snapshots)
        };

        if appids_to_fetch.is_empty() {
            return;
        }

        // Fan-out in batches of 10 (bounded concurrency — F-8 / performance constraint).
        for batch in appids_to_fetch.chunks(10) {
            let mut join_set = tokio::task::JoinSet::new();

            for &appid in batch {
                // Each task needs its own client reference. SteamClient is not Clone,
                // so we pass the api key/steam_id strings and construct per-task.
                let steam_id = config.steam_id.clone();
                let api_key = config.steam_api_key.clone();
                join_set.spawn(async move {
                    let c = SteamClient::new(&steam_id, &api_key);
                    let result = c.get_player_achievements(appid).await;
                    (appid, result)
                });
            }

            let mut batch_result: HashMap<u32, Option<u64>> = HashMap::new();
            while let Some(join_result) = join_set.join_next().await {
                match join_result {
                    Ok((appid, Ok(count))) => {
                        batch_result.insert(appid, Some(count as u64));
                    }
                    Ok((appid, Err(SteamApiError::NotAvailable))) => {
                        // 403/400 = private stats or no achievements — mark as None so we
                        // don't re-fetch this game on the next run.
                        batch_result.insert(appid, None);
                    }
                    Ok((_appid, Err(_e))) => {
                        // Transient error (502, timeout, etc.) — skip silently.
                        // Not cached, so this game will be retried on the next run.
                    }
                    Err(_e) => {
                        // Task panicked — skip this game, continue with the batch.
                    }
                }
            }

            if !batch_result.is_empty() {
                let _ = tx_fetch.send(AppEvent::AchievementsPartial(batch_result));
            }
        }
    });

    // ── 6. Run TUI (blocking — consumes main thread) ──────────────────────────
    run_tui(app, rx)?;

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a `Snapshot` from the freshly-fetched owned games list.
fn build_snapshot(owned: &[OwnedGame]) -> Snapshot {
    use cache::snapshot::GameEntry;
    let mut games = HashMap::new();
    for game in owned {
        games.insert(
            game.appid.to_string(),
            GameEntry {
                pt: game.playtime_forever,
                ach: None,
            },
        );
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Snapshot { ts, games }
}

/// Determine which appids are "recently active":
/// - Present in GetRecentlyPlayedGames (playtime_2weeks > 0), OR
/// - playtime_forever increased vs the last snapshot.
fn determine_active_appids(owned: &[OwnedGame], prior: &[Snapshot]) -> Vec<u32> {
    let last_snapshot = prior.last();

    owned
        .iter()
        .filter(|game| {
            // Condition 1: recently played (has playtime in last 2 weeks).
            if game.playtime_2weeks.map(|pt| pt > 0).unwrap_or(false) {
                return true;
            }
            // Condition 2: playtime increased vs last snapshot.
            if let Some(snap) = last_snapshot {
                if let Some(entry) = snap.games.get(&game.appid.to_string()) {
                    return game.playtime_forever > entry.pt;
                }
            }
            false
        })
        .map(|g| g.appid)
        .collect()
}
