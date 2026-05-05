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
    AchievementsPartial(HashMap<u32, u64>),
    /// A non-fatal or fatal API error occurred.
    ApiError(String),
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
    /// Cached achievement counts keyed by appid. None means "not yet fetched".
    pub achievement_cache: HashMap<u32, Option<u64>>,
    /// Sender half for posting events to this app's event loop.
    pub tx: UnboundedSender<AppEvent>,
}

impl App {
    /// Create a new App in the Loading state with a fresh channel.
    pub fn new() -> (App, UnboundedReceiver<AppEvent>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let app = App {
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
        };
        (app, rx)
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
                // Transition Loading → Loaded. Compute lightweight placeholder stats
                // until the snapshot store (later bundle) provides real deltas.
                app.stats.games_played = owned.len() as u64;
                app.recent_games = recent;
                app.state = AppState::Loaded;
            }
            AppEvent::AchievementsPartial(batch) => {
                for (appid, count) in batch {
                    app.achievement_cache.insert(appid, Some(count));
                }
            }
            AppEvent::ApiError(msg) => {
                app.state = AppState::Error(msg);
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
        batch.insert(730u32, 42u64);
        batch.insert(4000u32, 7u64);

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
}
