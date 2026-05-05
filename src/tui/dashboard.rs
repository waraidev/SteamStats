use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::models::stats::LABEL_EST_SESSIONS;

use super::app::{App, AppState};
use super::widgets::game_row::GameRow;
use super::widgets::period_bar::PeriodBar;
use super::widgets::stat_card::StatCard;

// MARK: - Main render entry point

/// Render the full dashboard to the given frame area.
///
/// Layout (top → bottom):
///   [period_bar: 1 line]
///   [body: fills remaining space]
///   [hint_bar: 1 line]
///   [status_bar: 1 line]
///
/// Within body (AD-3):
///   [stats_header: 4 lines]
///   [games_area: fills remaining]
///   [recent_area: 8 lines]
pub fn render_dashboard(f: &mut Frame, area: Rect, app: &App) {
    // Outer vertical layout: period bar | body | hint bar | status bar
    let outer = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);

    let period_bar_area = outer[0];
    let body_area = outer[1];
    let hint_bar_area = outer[2];
    let status_bar_area = outer[3];

    // Period bar
    f.render_widget(PeriodBar { selected: app.current_period }, period_bar_area);

    // Body sub-layout
    render_body(f, body_area, app);

    // Hint bar
    let hint = Line::from(vec![
        Span::raw("← → "),
        Span::styled("switch period", Style::default().add_modifier(Modifier::DIM)),
        Span::raw("   "),
        Span::raw("q "),
        Span::styled("quit", Style::default().add_modifier(Modifier::DIM)),
    ]);
    f.render_widget(Paragraph::new(hint), hint_bar_area);

    // Status bar — prefer status_message (background errors) over partial_data_note
    let status_text = app
        .status_message
        .as_deref()
        .or(app.stats.partial_data_note.as_deref())
        .unwrap_or("")
        .to_string();
    f.render_widget(
        Paragraph::new(Line::from(Span::raw(status_text))),
        status_bar_area,
    );
}

// MARK: - Body rendering

fn render_body(f: &mut Frame, area: Rect, app: &App) {
    let body = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(0),
        Constraint::Length(8),
    ])
    .split(area);

    let stats_header_area = body[0];
    let games_area = body[1];
    let recent_area = body[2];

    render_stats_header(f, stats_header_area, app);
    render_games_section(f, games_area, app);
    render_recent_section(f, recent_area, app);
}

// MARK: - Stats header

fn render_stats_header(f: &mut Frame, area: Rect, app: &App) {
    // Thin separator line between stats and games section
    let separator = Block::default().borders(Borders::BOTTOM);
    let inner = separator.inner(area);
    f.render_widget(separator, area);

    // Four equal columns: Games Played | Est. Sessions | Achievements | New Games
    let cols = Layout::horizontal([
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
        Constraint::Ratio(1, 4),
    ])
    .split(inner);

    let games_value = app.stats.games_played.to_string();
    let sessions_value = app.stats.est_sessions.to_string();
    // AC-9.2: achievements None renders as "—" (em-dash), not "0"
    let achievements_value = app
        .stats
        .achievements
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".to_string());
    let new_games_value = app.stats.new_games.to_string();

    f.render_widget(
        StatCard {
            label: "Games Played",
            value: games_value,
        },
        cols[0],
    );
    f.render_widget(
        StatCard {
            label: LABEL_EST_SESSIONS,
            value: sessions_value,
        },
        cols[1],
    );
    f.render_widget(
        StatCard {
            label: "Achievements",
            value: achievements_value,
        },
        cols[2],
    );
    f.render_widget(
        StatCard {
            label: "New Games",
            value: new_games_value,
        },
        cols[3],
    );
}

// MARK: - Games section

fn render_games_section(f: &mut Frame, area: Rect, app: &App) {
    // Empty state: only show when Loaded with no data (AC-1.4)
    if matches!(app.state, AppState::Loaded) && app.top_games.is_empty() {
        let text = if app.recent_games.is_empty() {
            "No activity in this period — run the app more frequently to build up history"
        } else {
            "Building period stats — run the app a few more times to accumulate snapshots"
        };
        let msg = Paragraph::new(text).block(Block::default().borders(Borders::NONE));
        f.render_widget(msg, area);
        return;
    }

    // Loading state
    if matches!(app.state, AppState::Loading) {
        let msg = Paragraph::new("Loading…");
        f.render_widget(msg, area);
        return;
    }

    // Error state
    if let AppState::Error(ref err) = app.state {
        let msg = Paragraph::new(format!("Error: {err}"));
        f.render_widget(msg, area);
        return;
    }

    // Normal: render each game as a GameRow.
    // Compute total minutes for percent share.
    let total_minutes: u64 = app
        .top_games
        .iter()
        .map(|g| g.playtime_delta_minutes)
        .sum();

    let row_height = 1u16;
    let available_rows = (area.height / row_height).min(app.top_games.len() as u16);

    for (i, game) in app.top_games.iter().take(available_rows as usize).enumerate() {
        let row_area = Rect {
            x: area.x,
            y: area.y + i as u16 * row_height,
            width: area.width,
            height: row_height,
        };

        let percent = if total_minutes > 0 {
            (game.playtime_delta_minutes as f64 / total_minutes as f64) * 100.0
        } else {
            0.0
        };
        let hours = game.playtime_delta_minutes as f64 / 60.0;

        f.render_widget(
            GameRow {
                rank: game.rank,
                name: game.name.clone(),
                percent,
                hours,
            },
            row_area,
        );
    }
}

// MARK: - Recently played section

fn render_recent_section(f: &mut Frame, area: Rect, app: &App) {
    // AC-6.2: empty recent games → "No recent activity"
    if app.recent_games.is_empty() {
        let block = Block::default()
            .title(" Recently Played ")
            .borders(Borders::TOP);
        let inner = block.inner(area);
        f.render_widget(block, area);
        f.render_widget(Paragraph::new("No recent activity"), inner);
        return;
    }

    let block = Block::default()
        .title(" Recently Played ")
        .borders(Borders::TOP);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Show up to 5 recently played games (FR-6)
    let rows = Layout::vertical(
        std::iter::repeat_n(Constraint::Length(1), app.recent_games.len().min(5))
            .collect::<Vec<_>>(),
    )
    .split(inner);

    for (i, game) in app.recent_games.iter().take(5).enumerate() {
        if i >= rows.len() {
            break;
        }
        let hours_2w = if game.playtime_2weeks > 0 {
            format!("{:.1}h", game.playtime_2weeks as f64 / 60.0)
        } else {
            "—".to_string()
        };
        let line = Line::from(vec![
            Span::raw(&game.name),
            Span::raw("  "),
            Span::styled(hours_2w, Style::default().add_modifier(Modifier::DIM)),
        ]);
        f.render_widget(Paragraph::new(line), rows[i]);
    }
}

// MARK: - Tests (STEP-20)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::stats::{GameStats, OverallStats, Period};
    use crate::models::steam::RecentGame;
    use crate::tui::app::{App, AppEvent, AppState};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::collections::HashMap;
    use tokio::sync::mpsc;

    fn make_loaded_app() -> App {
        let (tx, _rx) = mpsc::unbounded_channel::<AppEvent>();
        App {
            state: AppState::Loaded,
            current_period: Period::FourWeeks,
            stats: OverallStats {
                games_played: 3,
                est_sessions: 5,
                achievements: Some(20),
                new_games: 1,
                partial_data_note: None,
            },
            top_games: vec![
                GameStats {
                    appid: 730,
                    name: "Counter-Strike 2".to_string(),
                    playtime_delta_minutes: 120,
                    playtime_forever_minutes: 5000,
                    achievement_count: None,
                    rank: 1,
                },
                GameStats {
                    appid: 4000,
                    name: "Garry's Mod".to_string(),
                    playtime_delta_minutes: 60,
                    playtime_forever_minutes: 2000,
                    achievement_count: None,
                    rank: 2,
                },
            ],
            recent_games: vec![RecentGame {
                appid: 730,
                name: "Counter-Strike 2".to_string(),
                playtime_2weeks: 120,
                playtime_forever: 5000,
            }],
            achievement_cache: HashMap::new(),
            tx,
            force_achievement_refresh: false,
            status_message: None,
        }
    }

    fn render_app(app: &App, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render_dashboard(f, f.area(), app);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buf_string(buf: &ratatui::buffer::Buffer) -> String {
        buf.content.iter().map(|c| c.symbol().to_string()).collect()
    }

    // --- AC-9.2: achievements None renders as "—" ---

    #[test]
    fn achievements_none_renders_as_em_dash() {
        let (tx, _rx) = mpsc::unbounded_channel::<AppEvent>();
        let app = App {
            state: AppState::Loaded,
            current_period: Period::FourWeeks,
            stats: OverallStats {
                games_played: 1,
                est_sessions: 1,
                achievements: None, // <-- None
                new_games: 0,
                partial_data_note: None,
            },
            top_games: vec![],
            recent_games: vec![],
            achievement_cache: HashMap::new(),
            tx,
            force_achievement_refresh: false,
            status_message: None,
        };

        let buf = render_app(&app, 80, 24);
        let content = buf_string(&buf);
        // Must contain em-dash, not "0"
        assert!(
            content.contains('—'),
            "buffer should contain '—' for None achievements"
        );
    }

    // --- AC-6.2: empty recent games shows "No recent activity" ---

    #[test]
    fn empty_recent_games_shows_no_recent_activity() {
        let (tx, _rx) = mpsc::unbounded_channel::<AppEvent>();
        let app = App {
            state: AppState::Loaded,
            current_period: Period::FourWeeks,
            stats: OverallStats {
                games_played: 0,
                est_sessions: 0,
                achievements: None,
                new_games: 0,
                partial_data_note: None,
            },
            top_games: vec![],
            recent_games: vec![], // <-- empty
            achievement_cache: HashMap::new(),
            tx,
            force_achievement_refresh: false,
            status_message: None,
        };

        let buf = render_app(&app, 80, 24);
        let content = buf_string(&buf);
        assert!(
            content.contains("No recent activity"),
            "buffer should contain 'No recent activity'"
        );
    }

    // --- AC-2.4: partial_data_note appears in status bar ---

    #[test]
    fn partial_data_note_appears_in_status_bar() {
        let (tx, _rx) = mpsc::unbounded_channel::<AppEvent>();
        let app = App {
            state: AppState::Loaded,
            current_period: Period::FourWeeks,
            stats: OverallStats {
                games_played: 2,
                est_sessions: 3,
                achievements: Some(5),
                new_games: 0,
                partial_data_note: Some("Based on 5 days of data".to_string()),
            },
            top_games: vec![],
            recent_games: vec![],
            achievement_cache: HashMap::new(),
            tx,
            force_achievement_refresh: false,
            status_message: None,
        };

        let buf = render_app(&app, 80, 24);
        let content = buf_string(&buf);
        assert!(
            content.contains("Based on 5 days of data"),
            "buffer should contain partial_data_note text"
        );
    }

    // --- AC-1.4: empty top_games + Loaded shows "No activity in this period" ---

    #[test]
    fn empty_top_games_loaded_shows_no_activity_message() {
        let (tx, _rx) = mpsc::unbounded_channel::<AppEvent>();
        let app = App {
            state: AppState::Loaded,
            current_period: Period::FourWeeks,
            stats: OverallStats {
                games_played: 0,
                est_sessions: 0,
                achievements: None,
                new_games: 0,
                partial_data_note: None,
            },
            top_games: vec![], // <-- empty + Loaded
            recent_games: vec![],
            achievement_cache: HashMap::new(),
            tx,
            force_achievement_refresh: false,
            status_message: None,
        };

        let buf = render_app(&app, 80, 24);
        let content = buf_string(&buf);
        assert!(
            content.contains("No activity in this period"),
            "buffer should contain 'No activity in this period'"
        );
    }

    // --- Loading state does NOT show "No activity" ---

    #[test]
    fn loading_state_does_not_show_no_activity_message() {
        let (tx, _rx) = mpsc::unbounded_channel::<AppEvent>();
        let app = App {
            state: AppState::Loading, // <-- Loading, not Loaded
            current_period: Period::FourWeeks,
            stats: OverallStats {
                games_played: 0,
                est_sessions: 0,
                achievements: None,
                new_games: 0,
                partial_data_note: None,
            },
            top_games: vec![],
            recent_games: vec![],
            achievement_cache: HashMap::new(),
            tx,
            force_achievement_refresh: false,
            status_message: None,
        };

        let buf = render_app(&app, 80, 24);
        let content = buf_string(&buf);
        assert!(
            !content.contains("No activity in this period"),
            "Loading state should not show the 'No activity' message"
        );
    }

    // --- Loaded app renders game names ---

    #[test]
    fn loaded_app_renders_game_names() {
        let app = make_loaded_app();
        let buf = render_app(&app, 80, 24);
        let content = buf_string(&buf);
        assert!(
            content.contains("Counter-Strike"),
            "buffer should contain game name"
        );
    }
}
