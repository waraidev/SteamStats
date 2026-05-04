### Bundle 6: TUI Core
> Stage: depth | Parallel: yes (file-disjoint with Bundles 2/3/4; start after Bundle 5 completes) | Files: src/tui/dashboard.rs, src/tui/app.rs

**Bundle Verify**: Dashboard renders correct content and event loop handles state transitions.
- **Level**: unit
- **Given**: TestBackend (200×50) + App structs with known test data (None achievements, empty recent games, partial data note)
- **Action**: `cargo test tui::dashboard:: tui::app::`
- **Outcome**: Achievement stat shows "—" for None; "No recent activity" shown for empty recent list; DataLoaded event transitions App from Loading to Loaded; Left/Right key cycles periods

> **Context**
>
> **Applicable ACs**
> - **AC-1.1**: Given: App has loaded data / When: Dashboard renders / Then: Four stat blocks: Games Played, Est. Sessions, Achievements, New Games
> - **AC-1.2**: Given: ≥1 game with playtime / When: Dashboard renders / Then: Games listed rank, name, %, bar, hours
> - **AC-1.3**: Given: Data fetch in progress / When: Before data ready / Then: Loading indicator shown; no partial content
> - **AC-1.4**: Given: No data for period / When: Dashboard renders / Then: "No activity in this period" message
> - **AC-2.1**: Given: Dashboard visible / When: Any state / Then: Period selector at top, current highlighted
> - **AC-2.2**: Given: Dashboard loaded / When: Left/right/Tab / Then: Period changes, stats update
> - **AC-2.3**: Given: User selects Lifetime / When: Any state / Then: playtime_forever used, no cache required
> - **AC-2.4**: Given: Insufficient cache / When: 6 months or year selected / Then: "Based on N days of data" note shown
> - **AC-6.1**: Given: GetRecentlyPlayedGames returned results / When: Dashboard renders / Then: Up to 5 recent games with name and 2-week playtime
> - **AC-6.2**: Given: No recent activity / When: Dashboard renders / Then: "No recent activity" shown
> - **AC-9.2**: Given: Achievement fetch skipped or loading / When: Dashboard renders / Then: Shows "—" or "loading…", not 0 or blank
>
> **Architecture Decisions**
> - **AD-2: Mirror rally-tui event loop exactly** — `AppEvent` enum + tokio `mpsc::unbounded_channel` + `process_events()` drain + 250ms `crossterm::event::poll`. `try_recv()` in a while loop drains rapid event bursts without blocking render cycle.
> - **AD-3: Single-page dashboard layout with period tab bar** — Three vertical chunks: body (main), hint bar (1 line), status bar (1 line). Within body: period bar row top, two-column stats header, scrollable top-games table, recently played section.
>
> **Findings**
> - **F-2: Event loop pattern** — rally-tui run_tui(): enable_raw_mode + EnterAlternateScreen → loop: process_events() → terminal.draw() → event::poll(250ms) → handle_key() → cleanup on exit.
> - **F-1: rally-tui exact stack** — ratatui 0.29 `Frame::render_widget()` API; `Table` widget for games list.
>
> **Standards**
> - **S-5**: Terminal event poll interval: 250ms — `Duration::from_millis(250)` (Domain: other | File Type: .rs)
> - **S-7**: TUI rendering tests: use ratatui::backend::TestBackend (Domain: testing | File Type: .rs)
>
> **Constraints**
> - Session count must be displayed as "Est. Sessions" — use `LABEL_EST_SESSIONS` constant (Category: other | Source: spec)

---

#### STEP-19: Create src/tui/dashboard.rs
[FR-1 -> AC-1.1, AC-1.2, AC-1.4 | FR-2 -> AC-2.1, AC-2.2, AC-2.3, AC-2.4 | FR-6 -> AC-6.1, AC-6.2 | FR-9 -> AC-9.2] | create `src/tui/dashboard.rs` | Effort: L

> **Intent**: `achievements: Option<u64>` in `OverallStats` must render as `"—"` when `None` (AC-9.2) — not `"0"`, not empty string. `partial_data_note: Option<String>` must render in the status bar when `Some(...)` (AC-2.4). Recently played section (FR-6): render below the top-games table; show "No recent activity" when `app.recent_games.is_empty()` (AC-6.2). Empty state (AC-1.4): when `app.stats.games_played == 0` and app is Loaded state, show the "No activity in this period" message instead of the games table.

- `pub fn render_dashboard(f: &mut Frame, area: Rect, app: &App)` — main render entry point
- Layout: `Layout::vertical([Length(1), Min(0), Length(1), Length(1)])` → [period_bar_area, body_area, hint_bar_area, status_bar_area]
- Period bar: render `PeriodBar { selected: app.current_period }` in period_bar_area
- Body: within body_area, `Layout::vertical([Length(4), Min(0), Length(8)])` → [stats_header_area, games_area, recent_area]
- Stats header: 4-column horizontal layout with `StatCard` widgets; achievements value = `app.stats.achievements.map(|n| n.to_string()).unwrap_or("—".to_string())`; use `LABEL_EST_SESSIONS` for sessions card
- Games table: `Table` widget with `GameRow`-style rows; empty state: replace with "No activity in this period — run the app more frequently to build up history" paragraph when `app.top_games.is_empty()`
- Recently played: simple list block; "No recent activity" paragraph when `app.recent_games.is_empty()`; otherwise up to 5 games with name and `playtime_2weeks` minutes → hours
- Status bar: render `app.stats.partial_data_note` if Some; else empty

**Pattern reference**: `../rally-tui/src/tui/dashboard.rs`

**Verify**:
- Level: unit | Given: TestBackend 200×50 + App with `stats.achievements = None` | Action: `render_dashboard` | Outcome: Buffer contains `"—"` in the Achievements stat position; does NOT contain `"0"` in that position
- Level: unit | Given: App with empty `recent_games: vec![]` | Action: `render_dashboard` | Outcome: Buffer contains `"No recent activity"`

> **Standards**:
> - S-7: TestBackend for tests

> Depends on: STEP-17, STEP-4, STEP-1 | Enables: STEP-20, STEP-21 | Parallel with: STEP-7, STEP-9, STEP-11

---

#### STEP-20: Test Dashboard Rendering
MANUAL -> Test for STEP-19 (test-after) | modify `src/tui/dashboard.rs` | Effort: S

> **Intent**: Dashboard rendering tests catch content-correctness bugs that would only be noticed visually during use. Focus on the three AC-specific content cases: achievement "—" display, recently played empty state, and partial data note visibility.

- Add `#[cfg(test)] mod tests`; construct `App` with known test state
- Test: `achievements = None` → buffer contains `"—"`, not `"0"`
- Test: `recent_games = vec![]` → buffer contains `"No recent activity"`
- Test: `stats.partial_data_note = Some("Based on 5 days of data")` → text appears in status bar area
- Test: `top_games = vec![]` + `AppState::Loaded` → "No activity in this period" message appears

**Verify**:
- Level: unit | Given: TestBackend + App with known test data for each case | Action: `cargo test tui::dashboard::` | Outcome: All four content-correctness tests pass

> **Standards**:
> - S-7: TestBackend

> Depends on: STEP-19 | Enables: — | Parallel with: —

---

#### STEP-21: Create src/tui/app.rs
[FR-1 -> AC-1.3 | FR-2 -> AC-2.2] | create `src/tui/app.rs` | Effort: M

> **Intent**: `process_events()` drains `mpsc::UnboundedReceiver<AppEvent>` with `try_recv()` in a while loop — `recv().await` would block the render cycle (F-2). The 250ms poll interval (S-5) controls both input responsiveness and render rate — changing it breaks the tradeoff between CPU usage and perceived responsiveness. Loading state (AC-1.3): `AppState::Loading` is the initial state; `DataLoaded` transitions to `AppState::Loaded`; `AppState::Error(String)` on API failure. The render function checks this to show appropriate content.

- `#[derive(Clone, Debug)] pub enum AppState { Loading, Loaded, Error(String) }`
- `#[derive(Debug)] pub enum AppEvent { DataLoaded { owned: Vec<OwnedGame>, recent: Vec<RecentGame> }, AchievementsPartial(HashMap<u32, u64>), ApiError(String), Quit }`
- `pub struct App { pub state: AppState, pub current_period: Period, pub stats: OverallStats, pub top_games: Vec<GameStats>, pub recent_games: Vec<RecentGame>, pub achievement_cache: HashMap<u32, Option<u64>>, pub tx: UnboundedSender<AppEvent> }`
- `pub fn run_tui(mut app: App, mut rx: UnboundedReceiver<AppEvent>) -> Result<()>` — `enable_raw_mode()` + `EnterAlternateScreen`; loop: `process_events(&mut app, &mut rx)` → `terminal.draw(|f| render_dashboard(f, f.size(), &app))` → `if event::poll(Duration::from_millis(250))? { handle_key(&mut app, event::read()?) }`; `disable_raw_mode()` + `LeaveAlternateScreen` on exit
- `fn process_events(app: &mut App, rx: &mut UnboundedReceiver<AppEvent>)` — `while let Ok(ev) = rx.try_recv() { match ev { DataLoaded → update app state + stats, AchievementsPartial → merge into achievement_cache, ApiError → set AppState::Error, Quit → break } }`
- `fn handle_key(app: &mut App, ev: Event)` — `Key(Right | Tab) → cycle period forward`, `Key(Left | BackTab) → cycle backward`, `Key('q') | Key(Esc) → app.tx.send(AppEvent::Quit)`

**Pattern reference**: `../rally-tui/src/tui/app.rs:run_tui()`

**Verify**:
- Level: unit | Given: App in `AppState::Loading` + `AppEvent::DataLoaded` sent to channel | Action: `process_events()` | Outcome: `app.state == AppState::Loaded`
- Level: unit | Given: App with `current_period = Period::FourWeeks` | Action: `handle_key(Right)` | Outcome: `app.current_period == Period::SixMonths`

> **Standards**:
> - S-5: 250ms poll interval — `Duration::from_millis(250)`
> - S-7: TestBackend for render tests

> Depends on: STEP-19, STEP-4, STEP-1 | Enables: STEP-22, STEP-23, STEP-24 | Parallel with: STEP-7, STEP-9, STEP-11

---

#### STEP-22: Test Event Loop
MANUAL -> Test for STEP-21 (test-after) | modify `src/tui/app.rs` | Effort: S

> **Intent**: State transition tests verify the event loop handles the full `Loading → Loaded → (period switch) → Loaded` cycle. Period cycling must wrap at both ends (Lifetime → FourWeeks on Right, FourWeeks → Lifetime on Left) — an off-by-one or missing modulo produces a wrong period on wrap.

- Add `#[cfg(test)] mod tests`
- Test `DataLoaded` event → `AppState::Loaded`; stats updated from event data
- Test `ApiError("msg")` event → `AppState::Error("msg")`
- Test `handle_key(Right)` cycles: `FourWeeks → SixMonths → ThisYear → Lifetime → FourWeeks`
- Test `handle_key(Left)` reverse cycle: `FourWeeks → Lifetime`

**Verify**:
- Level: unit | Given: App at each period | Action: `cargo test tui::app::` | Outcome: State transitions verified; period wraps in both directions are correct

> Depends on: STEP-21 | Enables: — | Parallel with: —
