use std::collections::HashMap;
use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::models::stats::{GameStats, OverallStats, Period};
use crate::models::steam::{OwnedGame, RecentGame};

use super::dashboard::render_dashboard;

// MARK: - AppState

/// The loading state of the TUI application.
#[derive(Clone, Debug, PartialEq)]
pub enum AppState {
    Loading,
    Loaded,
    Error(String),
}

// MARK: - AppEvent

/// Events sent from background tasks to the UI.
#[derive(Debug)]
pub enum AppEvent {
    /// Steam API data loaded successfully.
    DataLoaded {
        owned: Vec<OwnedGame>,
        recent: Vec<RecentGame>,
    },
    /// Achievement data for a batch of games has arrived (may arrive multiple times).
    /// Value is `Some(count)` for games with achievements, `None` for unavailable/private games.
    AchievementsPartial(HashMap<u32, Option<u64>>),
    /// A non-fatal or fatal API error occurred.
    ApiError(String),
    /// A non-fatal background warning to display in the status bar (replaces eprintln! during TUI).
    StatusMessage(String),
    /// User requested quit.
    Quit,
}

// MARK: - App

/// Central application state for the TUI.
pub struct App {
    pub state: AppState,
    pub current_period: Period,
    pub stats: OverallStats,
    pub top_games: Vec<GameStats>,
    pub recent_games: Vec<RecentGame>,
    /// Cached achievement counts keyed by appid.
    /// - Key absent: not yet fetched.
    /// - `Some(n)`: unlocked count.
    /// - `None`: fetched but not available (private / no achievements).
    pub achievement_cache: HashMap<u32, Option<u64>>,
    /// Sender half for posting events to this app's event loop.
    pub tx: UnboundedSender<AppEvent>,
    /// When `true`, the background task should re-fetch all game achievements
    /// on its next cycle (set by the 'r' keybind).
    pub force_achievement_refresh: bool,
    /// Most recent non-fatal warning from background tasks, shown in the status bar.
    /// Displayed instead of partial_data_note when set.
    pub status_message: Option<String>,
}

impl App {
    /// Create a new App in the Loading state with a fresh internal channel.
    ///
    /// Used in tests where the caller needs both halves of the channel.
    pub fn new() -> (App, UnboundedReceiver<AppEvent>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let app = App::with_tx(tx);
        (app, rx)
    }

    /// Create a new App using an externally-created sender.
    ///
    /// Used from `main()` where the channel is created at the top level so that
    /// background tasks and the App share the same sender.
    pub fn with_tx(tx: UnboundedSender<AppEvent>) -> App {
        App {
            state: AppState::Loading,
            current_period: Period::FourWeeks,
            stats: OverallStats {
                games_played: 0,
                est_sessions: 0,
                achievements: None,
                new_games: 0,
                partial_data_note: None,
            },
            top_games: Vec::new(),
            recent_games: Vec::new(),
            achievement_cache: HashMap::new(),
            tx,
            force_achievement_refresh: false,
            status_message: None,
        }
    }
}

// MARK: - Event loop internals

/// Drain all pending AppEvents from `rx` without blocking.
///
/// Uses `try_recv()` in a while loop so rapid event bursts are fully consumed
/// without blocking the render cycle (AD-2).
pub fn process_events(app: &mut App, rx: &mut UnboundedReceiver<AppEvent>) {
    while let Ok(ev) = rx.try_recv() {
        match ev {
            AppEvent::DataLoaded { owned, recent } => {
                app.stats.games_played = owned.len() as u64;

                // Build top_games from recently played using playtime_2weeks as the delta.
                // This is the best available breakdown until snapshot-based deltas are ready.
                let mut games: Vec<GameStats> = recent
                    .iter()
                    .filter(|g| g.playtime_2weeks > 0)
                    .map(|g| GameStats {
                        appid: g.appid,
                        name: g.name.clone(),
                        playtime_delta_minutes: g.playtime_2weeks as u64,
                        playtime_forever_minutes: g.playtime_forever as u64,
                        achievement_count: None,
                        rank: 0,
                    })
                    .collect();
                games.sort_by(|a, b| b.playtime_delta_minutes.cmp(&a.playtime_delta_minutes));
                for (i, g) in games.iter_mut().enumerate() {
                    g.rank = i + 1;
                }
                app.top_games = games;
                app.recent_games = recent;
                // Clear any stale status message from before data loaded.
                app.status_message = None;
                app.state = AppState::Loaded;
            }
            AppEvent::AchievementsPartial(batch) => {
                // Merge partial results — existing entries NOT in this batch are preserved.
                // None entries mark games as "checked, unavailable" to prevent re-fetching.
                for (appid, count_opt) in batch {
                    app.achievement_cache.insert(appid, count_opt);
                }
                // Recompute stats.achievements as sum of all Some(_) values in the cache.
                // Remains None until first partial arrives (AC-9.2).
                let total: u64 = app
                    .achievement_cache
                    .values()
                    .filter_map(|v| *v)
                    .sum();
                app.stats.achievements = Some(total);
            }
            AppEvent::ApiError(msg) => {
                app.state = AppState::Error(msg);
            }
            AppEvent::StatusMessage(msg) => {
                app.status_message = Some(msg);
            }
            AppEvent::Quit => {
                // Signal quit by transitioning to an Error state with a sentinel
                // that run_loop recognizes as a clean exit.
                app.state = AppState::Error("__quit__".to_string());
            }
        }
    }
}

/// Handle a crossterm input event.
///
/// Returns `true` if the app should quit.
pub fn handle_key(app: &mut App, ev: Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = ev {
        match code {
            // Forward period cycle: FourWeeks → SixMonths → ThisYear → Lifetime → FourWeeks
            KeyCode::Right | KeyCode::Tab => {
                app.current_period = next_period(app.current_period);
            }
            // Backward period cycle
            KeyCode::Left | KeyCode::BackTab => {
                app.current_period = prev_period(app.current_period);
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                return true;
            }
            // 'r': trigger full achievement re-fetch on next background cycle.
            KeyCode::Char('r') => {
                app.force_achievement_refresh = true;
            }
            _ => {}
        }
    }
    false
}

/// Cycle period forward.
fn next_period(p: Period) -> Period {
    match p {
        Period::FourWeeks => Period::SixMonths,
        Period::SixMonths => Period::ThisYear,
        Period::ThisYear => Period::Lifetime,
        Period::Lifetime => Period::FourWeeks,
    }
}

/// Cycle period backward.
fn prev_period(p: Period) -> Period {
    match p {
        Period::FourWeeks => Period::Lifetime,
        Period::SixMonths => Period::FourWeeks,
        Period::ThisYear => Period::SixMonths,
        Period::Lifetime => Period::ThisYear,
    }
}

// MARK: - TUI entry point

/// Initialize the terminal and run the TUI event loop.
///
/// Mirrors rally-tui's `run_tui` pattern exactly (AD-2):
///   1. `process_events()` drains the channel (non-blocking, try_recv loop)
///   2. `terminal.draw()` renders current state
///   3. `event::poll(250ms)` + `handle_key()` handles keyboard input
pub fn run_tui(mut app: App, mut rx: UnboundedReceiver<AppEvent>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app, &mut rx);

    // Always restore terminal, even on error.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    rx: &mut UnboundedReceiver<AppEvent>,
) -> io::Result<()> {
    loop {
        // 1. Drain background events (non-blocking)
        process_events(app, rx);

        // Check for quit sentinel from Quit event
        if matches!(&app.state, AppState::Error(s) if s == "__quit__") {
            break;
        }

        // 2. Render
        terminal.draw(|f| {
            let area = f.area();
            render_dashboard(f, area, app);
        })?;

        // 3. Poll for keyboard input (250ms controls render rate — AD-2, S-5)
        if event::poll(Duration::from_millis(250))? {
            let ev = event::read()?;
            if handle_key(app, ev) {
                break;
            }
        }
    }
    Ok(())
}

// MARK: - Tests (STEP-22)

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};

    fn make_app() -> (App, UnboundedReceiver<AppEvent>) {
        App::new()
    }

    fn key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
    }

    // --- DataLoaded transitions Loading → Loaded ---

    #[test]
    fn data_loaded_transitions_loading_to_loaded() {
        let (mut app, mut rx) = make_app();
        assert_eq!(app.state, AppState::Loading);

        app.tx
            .send(AppEvent::DataLoaded {
                owned: vec![],
                recent: vec![],
            })
            .unwrap();

        process_events(&mut app, &mut rx);
        assert_eq!(app.state, AppState::Loaded);
    }

    #[test]
    fn data_loaded_updates_games_played_count() {
        let (mut app, mut rx) = make_app();

        let owned = vec![
            crate::models::steam::OwnedGame {
                appid: 1,
                name: "Game A".to_string(),
                playtime_forever: 60,
                playtime_2weeks: None,
                img_icon_url: None,
            },
            crate::models::steam::OwnedGame {
                appid: 2,
                name: "Game B".to_string(),
                playtime_forever: 120,
                playtime_2weeks: None,
                img_icon_url: None,
            },
        ];

        app.tx
            .send(AppEvent::DataLoaded {
                owned,
                recent: vec![],
            })
            .unwrap();

        process_events(&mut app, &mut rx);
        assert_eq!(app.stats.games_played, 2);
    }

    // --- ApiError transitions to Error state ---

    #[test]
    fn api_error_transitions_to_error_state() {
        let (mut app, mut rx) = make_app();

        app.tx
            .send(AppEvent::ApiError("connection refused".to_string()))
            .unwrap();

        process_events(&mut app, &mut rx);
        assert_eq!(
            app.state,
            AppState::Error("connection refused".to_string())
        );
    }

    // --- AchievementsPartial merges into cache ---

    #[test]
    fn achievements_partial_merges_into_cache() {
        let (mut app, mut rx) = make_app();

        let mut batch = HashMap::new();
        batch.insert(730u32, Some(42u64));
        batch.insert(4000u32, Some(7u64));

        app.tx
            .send(AppEvent::AchievementsPartial(batch))
            .unwrap();

        process_events(&mut app, &mut rx);
        assert_eq!(app.achievement_cache.get(&730), Some(&Some(42)));
        assert_eq!(app.achievement_cache.get(&4000), Some(&Some(7)));
        assert_eq!(app.achievement_cache.get(&999), None); // not in cache
    }

    // --- handle_key: Right cycles forward ---

    #[test]
    fn handle_key_right_cycles_period_forward() {
        let (mut app, _rx) = make_app();
        assert_eq!(app.current_period, Period::FourWeeks);

        handle_key(&mut app, key_event(KeyCode::Right));
        assert_eq!(app.current_period, Period::SixMonths);

        handle_key(&mut app, key_event(KeyCode::Right));
        assert_eq!(app.current_period, Period::ThisYear);

        handle_key(&mut app, key_event(KeyCode::Right));
        assert_eq!(app.current_period, Period::Lifetime);

        // Wrap: Lifetime → FourWeeks
        handle_key(&mut app, key_event(KeyCode::Right));
        assert_eq!(app.current_period, Period::FourWeeks);
    }

    // --- handle_key: Left cycles backward ---

    #[test]
    fn handle_key_left_reverse_cycle_fourweeks_to_lifetime() {
        let (mut app, _rx) = make_app();
        assert_eq!(app.current_period, Period::FourWeeks);

        // FourWeeks → Lifetime (wrap)
        handle_key(&mut app, key_event(KeyCode::Left));
        assert_eq!(app.current_period, Period::Lifetime);

        handle_key(&mut app, key_event(KeyCode::Left));
        assert_eq!(app.current_period, Period::ThisYear);

        handle_key(&mut app, key_event(KeyCode::Left));
        assert_eq!(app.current_period, Period::SixMonths);

        handle_key(&mut app, key_event(KeyCode::Left));
        assert_eq!(app.current_period, Period::FourWeeks);
    }

    // --- handle_key: q returns true (quit signal) ---

    #[test]
    fn handle_key_q_returns_quit_signal() {
        let (mut app, _rx) = make_app();
        let should_quit = handle_key(&mut app, key_event(KeyCode::Char('q')));
        assert!(should_quit);
    }

    // --- handle_key: Tab also cycles forward ---

    #[test]
    fn handle_key_tab_cycles_forward_like_right() {
        let (mut app, _rx) = make_app();
        handle_key(&mut app, key_event(KeyCode::Tab));
        assert_eq!(app.current_period, Period::SixMonths);
    }

    // MARK: - Tests (STEP-25): Achievement fan-out

    // --- AchievementsPartial: first batch populates cache and sets stats.achievements ---

    #[test]
    fn achievement_partial_first_batch_sets_sum() {
        let (mut app, mut rx) = make_app();
        assert_eq!(app.stats.achievements, None, "starts as None before any partial");

        let mut batch = HashMap::new();
        batch.insert(730u32, Some(50u64));
        batch.insert(440u32, Some(100u64));

        app.tx
            .send(AppEvent::AchievementsPartial(batch))
            .unwrap();

        process_events(&mut app, &mut rx);

        assert_eq!(
            app.achievement_cache.get(&730),
            Some(&Some(50)),
            "game 730 should have 50 achievements"
        );
        assert_eq!(
            app.achievement_cache.get(&440),
            Some(&Some(100)),
            "game 440 should have 100 achievements"
        );
        assert_eq!(
            app.stats.achievements,
            Some(150),
            "stats.achievements should be sum of all Some values"
        );
    }

    // --- AchievementsPartial: existing entry preserved when new partial doesn't include it ---

    #[test]
    fn achievement_partial_preserves_existing_cache_entries() {
        let (mut app, mut rx) = make_app();

        // First partial: game 730 gets 50 achievements.
        let mut first = HashMap::new();
        first.insert(730u32, Some(50u64));
        app.tx
            .send(AppEvent::AchievementsPartial(first))
            .unwrap();
        process_events(&mut app, &mut rx);

        assert_eq!(app.achievement_cache.get(&730), Some(&Some(50)));
        assert_eq!(app.stats.achievements, Some(50));

        // Second partial: only game 440 — game 730's entry must be preserved.
        let mut second = HashMap::new();
        second.insert(440u32, Some(100u64));
        app.tx
            .send(AppEvent::AchievementsPartial(second))
            .unwrap();
        process_events(&mut app, &mut rx);

        assert_eq!(
            app.achievement_cache.get(&730),
            Some(&Some(50)),
            "game 730 must be preserved (not in second batch)"
        );
        assert_eq!(
            app.achievement_cache.get(&440),
            Some(&Some(100)),
            "game 440 should be added"
        );
        assert_eq!(
            app.stats.achievements,
            Some(150),
            "sum should be 50 + 100 = 150"
        );
    }

    // --- stats.achievements is None when achievement_cache is empty ---

    #[test]
    fn achievement_stats_is_none_when_cache_is_empty() {
        let (app, _rx) = make_app();
        assert_eq!(
            app.stats.achievements, None,
            "stats.achievements must be None before any AchievementsPartial event"
        );
        assert!(
            app.achievement_cache.is_empty(),
            "cache must start empty"
        );
    }

    // --- stats.achievements sums only Some(_) values, ignoring None entries ---

    #[test]
    fn achievement_stats_sums_some_values_ignores_none() {
        let (mut app, mut rx) = make_app();

        // Pre-populate cache with a None entry (private game — no achievements available).
        app.achievement_cache.insert(999u32, None);

        // Send a partial with real data.
        let mut batch = HashMap::new();
        batch.insert(730u32, Some(42u64));
        app.tx
            .send(AppEvent::AchievementsPartial(batch))
            .unwrap();
        process_events(&mut app, &mut rx);

        assert_eq!(
            app.stats.achievements,
            Some(42),
            "None entries should not contribute to the sum"
        );
    }

    // --- 'r' keybind sets force_achievement_refresh = true ---

    #[test]
    fn handle_key_r_sets_force_achievement_refresh() {
        let (mut app, _rx) = make_app();
        assert!(!app.force_achievement_refresh, "starts false");

        handle_key(&mut app, key_event(KeyCode::Char('r')));

        assert!(
            app.force_achievement_refresh,
            "'r' keybind should set force_achievement_refresh = true"
        );
    }
}
