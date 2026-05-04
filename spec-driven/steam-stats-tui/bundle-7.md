### Bundle 7: App Wiring + Achievement Fan-out
> Stage: integration | Parallel: yes (file-disjoint with Bundle 8; start after Bundles 2/3/4/6 all complete) | Files: src/main.rs (create), src/tui/app.rs (modify)

**Bundle Verify**: Application starts with config, loads cached stats, spawns background API fetch, and updates dashboard on data arrival.
- **Level**: integration
- **Given**: Config file exists at tempfile path; wiremock server running with mocked Steam endpoints
- **Action**: `cargo test` (integration tests in main or separate test file)
- **Outcome**: App starts in Loading state; DataLoaded event arrives and transitions to Loaded state; achievement cache merges correctly from AchievementsPartial events

> **Context**
>
> **Applicable ACs**
> - **AC-1.3**: Given: App starts, data fetch in progress / When: Before data ready / Then: Loading indicator shown
> - **AC-3.1**: Given: Valid credentials / When: App launches / Then: GetOwnedGames called
> - **AC-5.3**: Given: Config file exists / When: App launches / Then: Loaded automatically, no prompt
> - **AC-9.1**: Given: Achievement data available / When: Dashboard renders / Then: Total achievement count displayed
> - **AC-9.2**: Given: Fetch skipped or loading / When: Dashboard renders / Then: Shows "—" or "loading…"
>
> **Architecture Decisions**
> - **AD-2: Mirror rally-tui event loop** — `tokio::spawn` for background API fetch task; `mpsc::unbounded_channel` connects background tasks to UI; `AppEvent::DataLoaded` arrives when fetch completes.
> - **AD-5: Smart-incremental achievement fetching** — On launch, determine recently-active games (in GetRecentlyPlayedGames OR playtime increased since last snapshot). Re-fetch GetPlayerAchievements only for those. First launch: fetch all games in batches of 10. Results stream as `AppEvent::AchievementsPartial`.
>
> **Findings**
> - **F-1: rally-tui exact stack** — `tokio::spawn` + `mpsc::unbounded_channel` for background-to-UI event passing.
> - **F-8: Achievement fan-out cost** — GetPlayerAchievements requires one API call per game; users with 500+ games cannot fetch all on every launch — smart-incremental strategy is required.
>
> **Standards**
> - **S-4**: Use dirs crate for config/data paths (Domain: other | File Type: .rs)
>
> **Constraints**
> - Achievement fan-out must be bounded (batch of 10) to avoid rate limits (Category: performance | Source: technical)
> - Config first-run prompt runs BEFORE enable_raw_mode() (Category: other | Source: design)
>
> **Assumptions**
> - tokio::spawn for background achievement fetch doesn't cause data races — all shared state goes through mpsc channel (Affects: FR-9, FR-1)
>
> **Risks**
> - Steam API returns 403 on achievement calls for private DLC or hidden stats (Impact: Low | Mitigation: SteamApiError::NotAvailable → skip that game's achievements, continue fan-out)

---

#### STEP-23: Create src/main.rs
[FR-1 -> AC-1.3 | FR-3 -> AC-3.1 | FR-5 -> AC-5.3] | create `src/main.rs` | Effort: M

> **Intent**: `main.rs` is the composition root — component wiring order matters. Config loading and first-run prompt MUST happen before `run_tui()` calls `enable_raw_mode()`, otherwise the prompt renders inside raw mode and is unreadable (AD-6 / config constraint). NFR-1 (<500ms to first render): snapshot loading is synchronous and fast (sub-ms for expected file sizes); the API fetch runs in a `tokio::spawn` task so `run_tui()` starts immediately with cached data.

- `#[tokio::main] async fn main() -> anyhow::Result<()>`
- Config: `match Config::load() { Err(ConfigError::NotFound) => Config::prompt_and_save(&default_config_path())?, Ok(c) => c, Err(e) => return Err(e.into()) }`
- Snapshot store: `let store = SnapshotStore::new(); let snapshots = store.load();`
- Initial stats: `let (top_games, initial_stats) = compute_delta(&snapshots, &Period::Lifetime, unix_now())` — Lifetime is always available (AC-2.3)
- Channel: `let (tx, rx) = mpsc::unbounded_channel::<AppEvent>();`
- Background fetch: `tokio::spawn` closure that calls `client.get_owned_games().await`, `client.get_recently_played().await`, builds and calls `store.append(snapshot)`, then `tx.send(AppEvent::DataLoaded { owned, recent })`
- `run_tui(App::new(tx, initial_stats, top_games), rx)`

**Pattern reference**: `../rally-tui/src/main.rs`

**Verify**:
- Level: inspection | Given: main.rs code | Action: Read code | Outcome: `Config::load()` (or `prompt_and_save`) appears before `run_tui()` with no `enable_raw_mode()` between them
- Level: integration | Given: Config tempfile + wiremock server | Action: Integration test mocking the app startup flow | Outcome: `AppState` transitions from Loading to Loaded after DataLoaded event

> **Standards**:
> - S-4: dirs crate for data path

> Depends on: STEP-7, STEP-9, STEP-11, STEP-21 | Enables: STEP-24 | Parallel with: STEP-26

---

#### STEP-24: Add achievement background fetch
[FR-9 -> AC-9.1, AC-9.2] | modify `src/tui/app.rs`, modify `src/main.rs` | Effort: M

> **Intent**: Smart-incremental logic (AD-5): "recently active" games = those in `GetRecentlyPlayedGames` response OR those whose `playtime_forever` increased vs the last snapshot's value. Only these games get re-fetched — this keeps per-launch API calls bounded even for libraries with 500+ games. First launch (no prior snapshot in store): fetch all games in batches of 10 using `tokio::task::JoinSet` — bounded concurrency prevents rate limiting (F-8). `AppEvent::AchievementsPartial(HashMap<u32, u64>)` events stream results as batches complete, triggering incremental dashboard updates. The `achievement_cache` on App merges each partial result — old values are preserved for games not in the current batch.

- In `main.rs`: after `DataLoaded` is sent, spawn a second `tokio::task` that: (a) determines recently-active appids (intersection of owned games + recently played), (b) chunks into batches of 10, (c) for each batch calls `get_player_achievements(appid)` for each game in the batch concurrently (JoinSet), (d) sends `AppEvent::AchievementsPartial(batch_result_map)` per completed batch
- Add 'r' keybind in `handle_key()` → `app.force_achievement_refresh = true` flag to trigger full re-fetch on next cycle
- In `process_events()`: on `AchievementsPartial(map)` → merge `map` into `app.achievement_cache` (existing entries not in map are preserved), recompute `app.stats.achievements` as sum of `achievement_cache` values that are `Some(_)`
- `app.stats.achievements` remains `None` until the first partial result arrives → dashboard renders "—" (AC-9.2)

**Pattern reference**: `../rally-tui/src/tui/app.rs` (tokio::spawn + AppEvent pattern)

**Verify**:
- Level: unit | Given: App with `achievement_cache = {}` + `AchievementsPartial({ 730: 50, 440: 100 })` event | Action: `process_events()` | Outcome: `achievement_cache == { 730: Some(50), 440: Some(100) }`; `stats.achievements == Some(150)`
- Level: unit | Given: App with `achievement_cache = { 730: Some(50) }` + `AchievementsPartial({ 440: 100 })` event | Action: `process_events()` | Outcome: `achievement_cache == { 730: Some(50), 440: Some(100) }` — old entry preserved

> **Standards**:
> - S-6: wiremock for testing API calls in tests

> Depends on: STEP-21, STEP-23, STEP-9 | Enables: STEP-25 | Parallel with: STEP-26

---

#### STEP-25: Test achievement fan-out
MANUAL -> Test for STEP-24 (test-after) | modify `src/tui/app.rs` | Effort: S

> **Intent**: The fan-out batch size of 10 must be tested — a regression that removes batching would send 500 concurrent requests to Steam and trigger rate limiting. The `AchievementsPartial` merge must preserve old cache entries — a test with a pre-existing entry verifies this.

- Add to `src/tui/app.rs` tests
- Test `AchievementsPartial` merge: existing entry preserved when new partial doesn't include it
- Test `stats.achievements` is `None` when `achievement_cache` is empty
- Test `stats.achievements` sums all `Some(_)` values in cache (ignoring `None` entries)
- Test 'r' keybind sets `force_achievement_refresh = true`

**Verify**:
- Level: unit | Given: achievement_cache states as described in STEP-24 intents | Action: `cargo test tui::app::achievement` | Outcome: Merge correctness, None propagation, and sum computation all verified

> Depends on: STEP-24 | Enables: — | Parallel with: STEP-27
