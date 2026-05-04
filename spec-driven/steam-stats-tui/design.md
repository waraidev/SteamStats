---
slug: steam-stats-tui
status: final
spec_source: spec-driven/steam-stats-tui/spec.md
spec_tier: 1
spec_hash: sha256:f372ea7d9e528f824db6395a2250339dc715571a5f77374aa3d070a207cd49a5
adaptive_flow: partial
test_approach: test-after
test_capabilities:
  unit: "rust #[test]"
  integration: wiremock
  e2e: null
created_date: 2026-05-04T00:00:00Z
last_updated: 2026-05-04T00:00:00Z
---

# Architectural Design: steam-stats-tui

## Overview

- **Spec**: Steam Stats TUI (9 FRs, 4 NFRs)
- **Architecture**: Greenfield Rust binary — no existing Rust code in the repo
- **Reference**: `../rally-tui` — a production Rust TUI in this workspace, exact same tech stack. Primary architecture reference throughout this design.
- **Test approach**: test-after `[no existing test framework]`
- **Test capabilities**: unit=`rust #[test]`, integration=`wiremock`, e2e=`null`
  - Note: `wiremock` is not yet a dependency — add to `[dev-dependencies]` to enable HTTP mocking for FR-3 (Steam API client) tests. Rally-tui proves this pattern works well.
  - No e2e framework needed — the app has no user-facing flows beyond "launch and view"; integration + unit coverage is sufficient.

## Technical Approach

### App Entry Point and Event Loop (FR-1, FR-2)

No Rust code exists yet. The project starts with a `Cargo.toml` scaffold. The entry point (`src/main.rs`) loads config (FR-5), initializes the snapshot store (FR-4), kicks off a background tokio task that fetches Steam API data (FR-3), and launches the TUI via `tui::run_tui(app)`.

**NFR-1 (startup <500ms)**: The app displays cached lifetime data immediately from the snapshot store before any network call completes. API fetch happens in a background tokio task; the dashboard renders with cached data first, then updates when fresh data arrives via `AppEvent::DataLoaded`. This satisfies the <500ms to first render target since file I/O (snapshot load) is sub-millisecond for expected file sizes.

The event loop follows rally-tui's proven pattern exactly: a tokio `mpsc::unbounded_channel<AppEvent>()` connects background tasks to the UI, `process_events()` drains the channel on each render tick, and `crossterm::event::poll(250ms)` handles keyboard input. See `../rally-tui/src/tui/app.rs` for the full pattern.

The dashboard is a single-page view (no list/board toggle needed). Layout: three vertical chunks — dashboard body (main area), hint bar (1 line), status bar (1 line). Within the body: period selector row at top, two-column stats header (games/sessions/achievements/new-games), then scrollable top-games list with progress bars (AD-3).

### Steam API Client (FR-3, FR-6, FR-9)

A `SteamClient` struct wraps `reqwest::Client` with the Steam ID and API key from config. All calls are async. On each launch, two calls run: `GetOwnedGames` (for lifetime playtime data) and `GetRecentlyPlayedGames` (for 2-week activity). Both results are saved to the snapshot store immediately.

Achievement fetching is **smart-incremental** (AD-5): on launch, determine which games are "recently active" (their `appid` appears in `GetRecentlyPlayedGames` results, or their `playtime_forever` increased since the previous snapshot). Re-fetch `GetPlayerAchievements` only for those games via background `tokio::spawn` fan-out. All other games serve cached achievement counts from the snapshot store. First-run fetches achievements for all owned games in batches of 10, respecting rate limits. Results stream back as `AppEvent::AchievementsPartial(HashMap<u32, u32>)` events.

JSON response shapes are derived from the Swift `SteamModels.swift` reference (F-7). All fields use `#[serde(rename_all = "snake_case")]` or explicit `#[serde(rename)]` since Steam's API uses snake_case field names and nested `response` envelopes (F-6).

### Snapshot Store (FR-4, FR-7)

The snapshot store lives at `~/.local/share/steam-stats/snapshots.ndjson` (macOS/Linux) or `%APPDATA%\steam-stats\snapshots.ndjson` (Windows), resolved via the `dirs` crate (AD-4).

Each run appends one JSON line with a Unix timestamp and a map of `appid → {playtime_forever, achievement_count}`:
```json
{"ts": 1746360000, "games": {"730": {"pt": 12345, "ach": 42}, "4000": {"pt": 567, "ach": 0}}}
```

Delta computation for a period window: load all snapshot lines, filter to those within the window, take the earliest and latest entries per game, compute `playtime_delta = latest.pt - earliest.pt`. Session estimation: count distinct calendar days where at least one game's playtime increased between consecutive snapshots. New games: games whose `appid` first appears in a snapshot within the period window.

Writes are atomic: write to a `.tmp` file, then `std::fs::rename()` atomically replaces the old file. This satisfies NFR-4 (AD-6).

### Configuration (FR-5)

Config lives at `~/.config/steam-stats/config.toml` (XDG-style, same convention as rally-tui's `~/.config/rally-cli/config.json`). On first run, if the file is absent, the app prompts inline (not via TUI — a plain `stdin` prompt before entering the alternate screen) for Steam ID and API key, then writes the config file.

Config struct uses `serde::Deserialize` with `#[serde(default)]` on all fields (S-1). Custom `Debug` impl redacts the `steam_api_key` field (S-2). The `Config::load()` / `Config::load_from(path)` split from rally-tui enables clean testing (S-7).

### Distribution and iOS Cleanup (FR-8)

The Homebrew tap is a separate `homebrew-steam-stats` repo containing `Formula/steam-stats.rb`. Scoop does NOT require a separate repo — a `scoop/steam-stats.json` manifest lives directly in this repo, and users add this repo as a Scoop bucket. The release workflow auto-updates both by computing SHA256 of each release artifact.

iOS cleanup (Swift files, Xcode project, iOS-specific docs) is deferred until after the TUI is functional. See File Inventory for the full delete list.

## Findings

| ID | Title | Source | Confidence | Related FRs | Summary |
|---|---|---|---|---|---|
| F-1 | rally-tui exact stack | codebase | high | FR-1, FR-2, FR-3 | rally-tui uses ratatui 0.29 + crossterm 0.28 + tokio + reqwest 0.12 — proven working stack for a Rust TUI with background HTTP calls |
| F-2 | Event loop pattern | codebase | high | FR-1, FR-2 | AppEvent mpsc channel + process_events() drain + 250ms crossterm poll is battle-tested in rally-tui; copy directly |
| F-3 | Config pattern | codebase | high | FR-5 | rally-tui Config struct: serde + #[serde(default)], dirs::home_dir() XDG path, custom Debug redacting API key, load_from(path) for tests |
| F-4 | File-based cache | codebase | high | FR-4 | rally-tui CacheManager uses file JSON + .meta sidecar — adequate for TTL caching; steam-stats needs different semantics (append-only history) |
| F-5 | wiremock for HTTP tests | codebase | high | FR-3 | rally-tui uses wiremock 0.6 in dev-dependencies for mocking Steam-like API calls; test_client(&MockServer) pattern reusable |
| F-6 | Steam JSON envelopes | codebase | high | FR-3, FR-6, FR-9 | All Steam API responses nest data under top-level "response" key; playtime in minutes (integer); snake_case keys |
| F-7 | Swift structs as serde reference | codebase | high | FR-3 | SteamModels.swift has canonical field names: playtime_forever, playtime_2weeks, img_icon_url, appid — direct 1:1 mapping to Rust serde structs |
| F-8 | Achievement fan-out cost | spec | high | FR-9 | GetPlayerAchievements requires one API call per game; users with 500+ games cannot fetch all on every launch |
| F-9 | NDJSON vs SQLite | training_knowledge | medium | FR-4, FR-7 | For our query pattern (filter-by-date, group-by-appid), linear scan of Vec<Snapshot> is O(n snapshots × m games) — fast enough for years of daily data |

See `references/research.md` for full research results.

## Architecture Decisions

### AD-1: Adopt rally-tui tech stack verbatim

- **Context**: This is a greenfield Rust TUI with no existing code. Choosing dependencies from scratch risks wheel-reinvention.
- **Decision**: We will use exactly `ratatui 0.29`, `crossterm 0.28`, `tokio 1.x (full)`, `reqwest 0.12 (json + rustls-tls, no default-features)`, `serde 1 (derive)`, `serde_json 1`, `dirs 6`, `thiserror 2`, `backon 1.3`, `chrono 0.4 (serde feature)`, `clap 4 (derive)`.
- **Rationale**: F-1 confirms this stack is working in a nearby project the user already ships. Avoids version discovery, incompatibility debugging, and learning new abstractions. Alignment with rally-tui makes future copy/adapt easy.
- **Alternatives Considered**: `cursive`, `tui-rs` (archived) — neither has the active community or the local reference. `ureq` instead of `reqwest` — sync-only, incompatible with tokio event loop.

---

### AD-2: Mirror rally-tui event loop exactly

- **Context**: TUI event loops require careful interleaving of terminal input polling and background data arrival. Getting this wrong causes UI freezes or missed key events.
- **Decision**: We will implement `AppEvent` enum + tokio `mpsc::unbounded_channel` + `process_events()` drain method + 250ms `crossterm::event::poll` — identical structure to `../rally-tui/src/tui/app.rs:run_tui()`.
- **Rationale**: F-2 confirms this pattern handles rapid key events, background data arrival, and error display without modification needed. Directly reusable.
- **Alternatives Considered**: `crossterm::event::EventStream` (async stream) — adds complexity with no benefit for our single-view dashboard.

---

### AD-3: Single-page dashboard layout with period tab bar

- **Context**: The spec shows one dashboard view; no list/board toggle needed. Period selector must be always visible and keyboard-navigable.
- **Decision**: We will render a period tab bar at the top using a custom widget (left/right arrows cycle through [4 Weeks | 6 Months | This Year | Lifetime]), then a two-column stats header (Games | Sessions | Achievements | New Games), then a scrollable `Table` widget for top games with inline `LineGauge` progress bars.
- **Rationale**: Simpler app state than rally-tui (no ViewMode enum needed). Period selector via Tab/Left/Right matches spec AC-2.2. ratatui's `LineGauge` or `Gauge` widget provides progress bars without custom widget code.
- **Alternatives Considered**: Full custom period picker dialog — unnecessary complexity; tab bar at top is always visible and immediate.

---

### AD-4: NDJSON snapshot store with atomic writes

- **Context**: FR-4 needs append-only history for delta computation. rally-tui's TTL cache (Hit/Stale/Miss) is the wrong abstraction. Open question from spec: SQLite vs JSON.
- **Decision**: We will use NDJSON (JSON Lines) stored at the XDG data dir (`dirs::data_dir().join("steam-stats/snapshots.ndjson")`). Each run appends one line. Writes use a `.tmp` rename for atomicity (NFR-4). Schema: `{"ts": <unix_secs>, "games": {"<appid>": {"pt": <minutes>, "ach": <count_or_null>}}}`.
- **Rationale**: F-9 confirms our query complexity doesn't require SQL. No C dependency (vs `rusqlite` + `libsqlite3-sys`). Append-only means low corruption risk. Rename-atomic write satisfies NFR-4. File readable/debuggable with `jq`.
- **Alternatives Considered**: SQLite — powerful but adds ~2MB and a C build dependency. Overkill for a flat time-series with one record per day. Plain JSON (single array) — requires rewriting entire file on each run; unsafe under crash (partial write corrupts history).

---

### AD-5: Smart-incremental achievement fetching

- **Context**: FR-9 requires achievement totals but GetPlayerAchievements is O(n games). Can't fetch all on every launch (too slow). Spec notes this is opt-in/background.
- **Decision**: We will fetch achievements only for "recently active" games on each launch. A game is recently active if: (a) it appears in `GetRecentlyPlayedGames`, or (b) its `playtime_forever` increased since the last snapshot. All other games use `ach` values from the snapshot store. First launch fetches all games in background batches of 10 (yielding `AppEvent::AchievementsPartial` events as each batch completes). A `r` keybind forces a full achievement re-fetch.
- **Rationale**: F-8 confirms the fan-out cost. This strategy keeps achievements current for actively played games (the ones that matter) while cold-starting gracefully on a large library. Matches spec AC-9.2 (show "—" until data arrives).
- **Alternatives Considered**: Fetch all achievements on every launch — too slow (10–60s for large libraries). Never refresh cached achievements — stale counts for games you're actively playing.

---

### AD-6: TOML config format (not JSON)

- **Context**: rally-tui uses JSON for config due to Python compat. Steam-stats has no legacy constraint.
- **Decision**: We will use TOML for `~/.config/steam-stats/config.toml` via the `toml` crate. Fields: `steam_id`, `steam_api_key`. First-run writes a minimal file; unknown fields are silently ignored on future reads.
- **Rationale**: TOML is more ergonomic for hand-editing (comments, no trailing-comma issues). The `toml` crate is pure Rust with no extra deps. Config is small (2 fields) so no tooling advantage to JSON.
- **Alternatives Considered**: JSON (no compelling reason without legacy compat). `confy` crate — adds an abstraction that hides the file path from the user; since the user needs to know where to find their API key, an explicit known path is preferable.

---

### AD-7: All distribution from the main repo — no separate tap/bucket repos

- **Context**: FR-8 requires Mac and Windows distribution. Original plan assumed a separate Homebrew tap repo for Mac. User prefers everything in the main repo.
- **Decision**: We will ship two distribution mechanisms, both living in this repo:
  1. **Mac/Linux**: An `install.sh` shell script in the repo root that auto-detects architecture (`uname -m`), downloads the correct pre-built binary from the latest GitHub Release, and installs to `/usr/local/bin/steam-stats`. Users run `curl -sSf https://raw.githubusercontent.com/<user>/SteamStats/main/install.sh | sh`. Also supports `cargo install --git` for users with Rust installed.
  2. **Windows**: A `scoop/steam-stats.json` manifest in the repo. Users run `scoop bucket add steam-stats https://github.com/<user>/SteamStats && scoop install steam-stats`.
  The release workflow updates `scoop/steam-stats.json` (SHA256 + download URL) and the `install.sh` `VERSION` variable automatically on each tagged release.
- **Rationale**: No separate repos to maintain. install.sh is a universal pattern (rustup, Homebrew itself, etc.) that works without requiring Homebrew or any package manager. Scoop-from-main-repo is already planned.
- **Alternatives Considered**: Homebrew tap (separate repo) — unnecessary complexity for personal use. Homebrew cask (separate repo + PR to homebrew-cask) — heavyweight for a personal tool.

---

### AD-8: iOS cleanup deferred to post-TUI task

- **Context**: User wants to delete all iOS/Swift artifacts from the repo, but after the TUI is functional (confirmed at Research Scope gate).
- **Decision**: We will include a dedicated cleanup task in the task decomposition that deletes all iOS-related files: `SteamStats_iOS/`, `SteamStats.xcodeproj/`, `SteamStatsTests/`, `project.yml`, `PROJECT_SUMMARY.md`, `SETUP.md`, and the current `README.md` (replaced by a Rust-specific one). This task is ordered last in the execution plan.
- **Rationale**: The Swift code is useful as a reference for Steam API response shapes during TUI implementation. Deleting it first removes the reference corpus. Deleting it last keeps the repo clean at release time.
- **Alternatives Considered**: Delete iOS files first — loses the API shape reference mid-build.

## Resolved Uncertainties

| # | Question | Answer |
|---|---|---|
| 1 | TUI crate choice (spec OQ-1) | `ratatui` 0.29 — confirmed by rally-tui reference (F-1) |
| 2 | Cache format (spec OQ-2) | NDJSON (AD-4) — SQLite overkill for our query patterns |
| 3 | Achievement fetch trigger (spec OQ-3) | Smart-incremental background fetch on each launch (AD-5); `r` keybind for force-refresh |
| 4 | Homebrew tap / Scoop bucket naming (spec OQ-4) | No separate repos needed — Mac uses `install.sh` in main repo; Scoop uses `scoop/` dir in main repo (AD-7) |

## Standards

| ID | Rule | Domain | File Type | Action Type | Source |
|---|---|---|---|---|---|
| S-1 | Use `#[serde(default)]` on all config struct fields | api-design | .rs | create | `../rally-tui/CLAUDE.md` |
| S-2 | Implement custom `Debug` that redacts `steam_api_key` to `[REDACTED]` | security | .rs | create | `../rally-tui/CLAUDE.md` |
| S-3 | API keys must not appear in source code, git history, or log output | security | * | * | spec NFR-2 |
| S-4 | Use `dirs` crate for config/data paths — never hardcode `~/` paths or platform strings | other | .rs | create | `../rally-tui/src/config/mod.rs` |
| S-5 | Terminal event poll interval: 250ms (`Duration::from_millis(250)`) | other | .rs | create | `../rally-tui/src/tui/app.rs:1072` |
| S-6 | HTTP API mocking in tests: use `wiremock` (dev-dependency) | testing | .rs | create | `../rally-tui/Cargo.toml` |
| S-7 | TUI rendering tests: use `ratatui::backend::TestBackend` | testing | .rs | create | `../rally-tui/src/tui/app.rs` |
| S-8 | Temp files in tests: use `tempfile` crate (dev-dependency) | testing | .rs | create | `../rally-tui/Cargo.toml` |

See `references/standards.md` for full standards inventory with complete applicability metadata.

## File Inventory

| Action | Path | Related FRs | Rationale |
|---|---|---|---|
| create | `Cargo.toml` | all | Project manifest — dependencies from AD-1 |
| create | `src/main.rs` | FR-1, FR-3, FR-5 | Entry point: load config, init snapshot store, spawn API fetch task, run TUI |
| create | `src/config/mod.rs` | FR-5 | Config struct (steam_id, steam_api_key), load/load_from, first-run prompt |
| create | `src/models/steam.rs` | FR-3, FR-6, FR-9 | Serde structs for Steam API responses (from SteamModels.swift reference) |
| create | `src/models/stats.rs` | FR-1, FR-2, FR-4, FR-7 | Computed stats: Period enum, GameStats, OverallStats, delta computation functions |
| create | `src/models/mod.rs` | all | Re-exports |
| create | `src/api/client.rs` | FR-3, FR-6, FR-9 | SteamClient: GetOwnedGames, GetRecentlyPlayedGames, GetPlayerAchievements |
| create | `src/api/mod.rs` | FR-3 | Re-exports, SteamApiError |
| create | `src/cache/snapshot.rs` | FR-4, FR-7 | SnapshotStore: load, append, delta_for_period, session_estimate, new_games_in_period |
| create | `src/cache/mod.rs` | FR-4 | Re-exports |
| create | `src/tui/app.rs` | FR-1, FR-2 | App state, AppEvent enum, process_events(), handle_key(), run_tui() |
| create | `src/tui/dashboard.rs` | FR-1, FR-2, FR-6 | Main dashboard render: layout, period bar, stats header, top games table, recently played |
| create | `src/tui/widgets/period_bar.rs` | FR-2 | Period selector tab bar widget |
| create | `src/tui/widgets/stat_card.rs` | FR-1 | Individual stat card (number + label) |
| create | `src/tui/widgets/game_row.rs` | FR-1 | Game row with rank, name, %, LineGauge, hours |
| create | `src/tui/mod.rs` | FR-1 | Re-exports, run_tui() |
| create | `.github/workflows/release.yml` | FR-8 | Cross-compile matrix (x86_64-apple-darwin, aarch64-apple-darwin, x86_64-pc-windows-msvc), GitHub Release, auto-update scoop/steam-stats.json and install.sh VERSION |
| create | `install.sh` | FR-8 | Mac/Linux installer: detects arch, downloads correct binary from latest release, installs to /usr/local/bin |
| create | `scoop/steam-stats.json` | FR-8 | Scoop bucket manifest (auto-updated by release workflow) |
| create | `README.md` | FR-8 | Rust-specific README (replaces iOS README) |
| delete | `SteamStats_iOS/` | — | iOS Swift source (post-TUI cleanup, AD-8) |
| delete | `SteamStats.xcodeproj/` | — | Xcode project (post-TUI cleanup, AD-8) |
| delete | `SteamStatsTests/` | — | iOS tests (post-TUI cleanup, AD-8) |
| delete | `project.yml` | — | XcodeGen config (post-TUI cleanup, AD-8) |
| delete | `PROJECT_SUMMARY.md` | — | iOS project summary (post-TUI cleanup, AD-8) |
| delete | `SETUP.md` | — | iOS setup guide (post-TUI cleanup, AD-8) |

## Dependencies and Coupling

| Feature Area | Shared Files | Recommendation |
|---|---|---|
| FR-3 (API) + FR-4 (Cache) | `src/models/steam.rs` | Walking skeleton: models first, then API client, then snapshot store. Both consume the same Game struct. |
| FR-4 (Cache) + FR-1, FR-2 (Dashboard) | `src/models/stats.rs` | OverallStats and GameStats are computed by snapshot.rs and consumed by dashboard rendering. Hoist into models early. |
| FR-4 (Cache) + FR-7 (New Games) | `src/cache/snapshot.rs` | `new_games_in_period()` lives in SnapshotStore — not a separate module. FR-7 is zero extra coupling. |
| FR-9 (Achievements) + FR-4 (Cache) | `src/cache/snapshot.rs` | Achievement counts stored in snapshot alongside playtime. The `ach` field is nullable in the schema — FR-9 is gracefully optional. |

**Suggested bundle order for task decomposition:**
1. Cargo.toml + models (walking skeleton)
2. Config + first-run prompt (FR-5)
3. API client + tests (FR-3)
4. Snapshot store + delta computation (FR-4, FR-7)
5. TUI event loop + dashboard rendering (FR-1, FR-2)
6. Recently played section (FR-6)
7. Achievement background fetch (FR-9)
8. CI release workflow + Scoop manifest + Homebrew tap (FR-8)
9. iOS cleanup (AD-8)

## Spec Deviations

| Spec Value | Location | Design Value | Rationale |
|---|---|---|---|
| Cache path `~/.local/share/steam-stats/` (macOS/Linux) | AC-4.4 | `dirs::data_dir().join("steam-stats/")` — resolves to `~/Library/Application Support/steam-stats/` on macOS, `~/.local/share/steam-stats/` on Linux | `dirs::data_dir()` follows platform conventions correctly. macOS XDG-style (`~/.local/share/`) is non-standard — proper macOS data dir is `~/Library/Application Support/`. Using `dirs` handles this automatically. |

## Open Questions

All four spec open questions are resolved (see Resolved Uncertainties). No new questions emerged during research.

## Constraints (Technical)

| Constraint | Category | Source | Rationale |
|---|---|---|---|
| `reqwest` must use `rustls-tls` feature with `default-features = false` | compatibility | codebase | Prevents linking against OpenSSL on Linux, which causes build failures in musl targets needed for cross-compilation |
| Snapshot file writes must be atomic (temp + rename) | infrastructure | technical | Non-atomic writes leave partial JSON lines that corrupt the NDJSON format on crash |
| Achievement fan-out must be bounded (batch of N, not all at once) | performance | technical | Unbounded concurrent requests to Steam API risk triggering rate limits (429) |
| `dirs::data_dir()` may return `None` on unusual platforms | infrastructure | technical | Must have a fallback path (e.g., current dir or error message) to avoid panic |
| Session count must be displayed as "Est. Sessions" (not "Sessions") in the UI | other | spec (Scope > Constraints) | Spec constraint: "Sessions are estimated, not exact — this must be labeled clearly in the UI." Dashboard stat card label must say "Est. Sessions." |

## Assumptions

| Assumption | Source | Affects |
|---|---|---|
| `tokio::spawn` for background achievement fetch doesn't cause data races — all shared state goes through mpsc channel | design | FR-9, FR-1 |
| The user runs steam-stats at least a few times per week, so snapshot history accumulates quickly enough for meaningful 4-week views | design | FR-4, FR-2 |
| Scoop's git-repo-as-bucket feature is stable and will remain available | research | FR-8 |
| The `homebrew-steam-stats` tap repo will be created by the user separately (not auto-created by CI) | design | FR-8 |

## Risks (Technical)

| Risk | Impact | Probability | Mitigation | Affects |
|---|---|---|---|---|
| Steam API returns 403 on achievement calls for private DLC or hidden stats | Low | Medium | Treat non-2xx per-game achievement responses as "unknown" (store `null`), continue fan-out | FR-9 |
| NDJSON file grows large after years of daily use (each snapshot ~50KB for 500-game library) | Low | Low | Prune snapshots older than 2 years on load; one-liner for users to clear manually | FR-4 |
| Cross-compilation for Windows fails in CI due to missing MSVC target | Medium | Low | Use `cross` tool or GitHub-hosted Windows runner; rally-tui has no Windows CI to reference | FR-8 |
| `dirs::data_dir()` returns different paths between macOS versions or when HOME is unset | Low | Low | Fallback to `PathBuf::from(".")` with a warning; document in README | FR-4, FR-5 |

## References

- See `references/research.md` for full research results per aspect
- See `references/standards.md` for complete standards inventory (8 standards)
- Reference project: `../rally-tui` — full working Rust TUI; primary architecture reference for all AD-1 through AD-3 decisions
