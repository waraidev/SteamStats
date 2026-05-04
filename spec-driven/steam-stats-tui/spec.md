# Specification: Steam Stats TUI

> Date: 2026-05-04
> Version: 1.0
> Location: spec-driven/steam-stats-tui/spec.md
> Tracking: N/A
> Source: Interactive elicitation

> **Provenance Key**: Content sources are marked inline:
> - **[User]** — Directly stated by the user
> - **[Inferred]** — Synthesized by the agent from available context
> - **[Default]** — Standard default applied
> - **[Codebase]** — Derived from codebase analysis

## Project Context

**Parent Project**: SteamStats repository. An iOS app was previously scaffolded here but will not be developed further — this Rust TUI replaces it as the primary artifact. See `CLAUDE.md` for project-wide principles. **[Codebase]**

**Scope**: A personal-use Rust terminal UI app that displays Steam gaming statistics across multiple timeframes using the Steam Web API and a local snapshot cache for historical data. **[User]**

## Overview

Steam Stats TUI is a Rust terminal application that displays personal Steam gaming statistics in a Stats.fm-inspired dashboard layout. It shows key metrics — games played, estimated sessions, achievements unlocked, and new games — alongside a ranked top-games list with percentage progress bars. **[User]**

The app is designed for personal use, not distribution to general audiences. It runs from the command line and renders entirely in the terminal using a dashboard UI (single-page, period selector at top). **[User]**

The Steam Web API does not expose per-session data or historical month-by-month breakdowns. To support 4-week, 6-month, and year-to-date timeframes, the app maintains a local snapshot cache: each run records current playtime data, and later runs compute deltas to approximate historical views. Sessions are estimated by counting distinct play-days observed across snapshots. **[Inferred]**

### Current State

A Swift iOS app was scaffolded in this repository (see `SteamStats_iOS/`) with the same data model and API client. That effort was abandoned in favor of a simpler personal tool. The Swift code can serve as a reference for Steam API models and endpoint patterns. **[Codebase]**

## Goals

### Primary Goal
Provide a fast, personal terminal dashboard for viewing Steam gaming statistics without the overhead of building and maintaining a mobile app. **[User]**

### Secondary Goals
1. Support multiple meaningful timeframes (4 weeks, 6 months, current year, lifetime) via local snapshot caching. **[User]**
2. Be distributable on macOS via Homebrew and Windows via Scoop so the binary is easy to install and update. **[User]**

### Non-Goals (Explicitly Out of Scope)
- Social or friends-comparison features **[User]**
- Web or mobile interface **[User]**
- Non-Steam game tracking **[Inferred]**
- Real-time notifications or background monitoring **[Inferred]**
- Writing or modifying any Steam data **[Inferred]**
- Exact (non-estimated) session counts — Valve's internal session data is not in the public API **[Inferred]**

## Users

### Primary Users
| User Type | Description | Goals | Pain Points |
|-----------|-------------|-------|-------------|
| Personal user (developer) | Single user running the tool locally for their own Steam account | Quick terminal access to personal Steam stats across timeframes | iOS app too heavy for personal use; Steam's own Replay is annual-only | **[User]** |

## Functional Requirements

### FR-1: Dashboard View

**Description**: The terminal renders a single-page dashboard showing a header stats row (Games Played, Est. Sessions, Achievements, New Games) and a scrollable top-games list with playtime percentage bars and hours. The selected period is shown prominently. **[User]**

**User Story**: As a personal user, I want a at-a-glance dashboard showing my key gaming stats and top games so that I can review my activity without leaving the terminal.

**Acceptance Criteria**:

| ID | Criterion | Given | When | Then |
|----|-----------|-------|------|------|
| AC-1.1 | Header stats displayed | The app has loaded data for the selected period | The dashboard renders | Four stat blocks are shown: Games Played, Est. Sessions, Achievements, New Games — each as a large number with a label |
| AC-1.2 | Top games list with bars | At least one game has playtime in the selected period | The dashboard renders | Games are listed in order from highest to lowest playtime %, each row showing rank, game name, %, a filled progress bar, and hours played |
| AC-1.3 | Loading state | The app starts and data fetch is in progress | Before data is ready | A loading indicator is shown and the dashboard layout is not partially rendered |
| AC-1.4 | Empty state | No data exists for the selected period | The dashboard renders | A clear message is shown (e.g., "No activity in this period — run the app more frequently to build up history") |

**Priority**: Must Have

**Goal**: Primary

**Dependencies**: FR-3 (data), FR-4 (cache for non-lifetime periods)

---

### FR-2: Period Selector

**Description**: The user can switch between four timeframes — 4 weeks, 6 months, current calendar year, and lifetime. Switching recomputes all stats and rerenders the dashboard without a full refetch. **[User]**

**User Story**: As a personal user, I want to switch between 4-week, 6-month, year-to-date, and lifetime views so that I can see my stats across different windows of time.

**Acceptance Criteria**:

| ID | Criterion | Given | When | Then |
|----|-----------|-------|------|------|
| AC-2.1 | Selector rendered | Dashboard is visible | Any state | A period selector (e.g., tab bar or dropdown label) is shown at the top of the dashboard with the current period highlighted |
| AC-2.2 | Period switch works | Dashboard is loaded | User presses left/right arrow keys or Tab/Shift-Tab | The selected period changes and all stats update to reflect the new window |
| AC-2.3 | Lifetime always available | Any state | User selects Lifetime | Stats are shown using all-time `playtime_forever` values from `GetOwnedGames` — no cache required |
| AC-2.4 | Insufficient cache data labeled | Cache has fewer snapshots than needed for the selected period | User selects 6 months or year | A note is shown indicating the period is partially estimated (e.g., "Based on 12 days of data") |

**Priority**: Must Have

**Goal**: Primary

**Dependencies**: FR-4 (cache for non-lifetime periods)

---

### FR-3: Steam API Data Fetch

**Description**: On launch, the app fetches the user's owned games, recently played games, and (optionally) achievement counts from the Steam Web API. All requests use the stored Steam ID and API key. **[User]** **[Codebase]**

**User Story**: As a personal user, I want the app to fetch my Steam data automatically on launch so that I always see up-to-date stats.

**Acceptance Criteria**:

| ID | Criterion | Given | When | Then |
|----|-----------|-------|------|------|
| AC-3.1 | Owned games fetched | Valid Steam ID and API key are configured | App launches | `GetOwnedGames` is called with `include_appinfo=1` and results are stored to cache |
| AC-3.2 | Recently played fetched | Valid Steam ID configured | App launches | `GetRecentlyPlayedGames` is called and results are stored |
| AC-3.3 | API error handled | The Steam API returns a non-2xx response or is unreachable | App launches | An error message is shown in the dashboard with a retry option; app does not crash |
| AC-3.4 | Rate limit respected | App has already fetched `GetOwnedGames` in the current run | A second call to load the dashboard is triggered (e.g., period switch) | No new network request is made; data is served from the in-memory result of the initial fetch |

**Priority**: Must Have

**Goal**: Primary

**Dependencies**: FR-5 (Steam ID config)

---

### FR-4: Local Snapshot Cache

**Description**: Each run, the app saves a timestamped snapshot of per-game playtime to a local file (SQLite or JSON). Subsequent runs compute deltas between snapshots to derive timeframe-specific stats. Session estimates are derived by counting distinct days a game's playtime increased. **[User]** **[Agent Decision]**

**User Story**: As a personal user, I want the app to build up a local history of my playtime so that 4-week, 6-month, and year-to-date views become more accurate over time.

**Acceptance Criteria**:

| ID | Criterion | Given | When | Then |
|----|-----------|-------|------|------|
| AC-4.1 | Snapshot written on run | App successfully fetches data from Steam API | End of a run | A snapshot record is written to the local cache file with a timestamp and per-game playtime values |
| AC-4.2 | Delta computed for period | At least two snapshots exist within the requested period window | User selects a period | Stats reflect the playtime delta between the oldest and newest snapshot within the window |
| AC-4.3 | Session estimation | Multiple snapshots exist showing playtime increases | Dashboard renders | Est. Sessions is computed as the count of distinct calendar days where at least one game's playtime increased |
| AC-4.4 | Cache survives reinstall | User reinstalls or rebuilds the binary | App launches | Cache file is read from a stable user-data path (e.g., `~/.local/share/steam-stats/` on macOS/Linux, `%APPDATA%\steam-stats\` on Windows) and data is preserved |
| AC-4.5 | Cache corruption handled | Cache file is malformed | App launches | App falls back to live API data for the current run and logs a warning; it does not crash |

**Priority**: Must Have

**Goal**: Secondary-1

**Dependencies**: FR-3

---

### FR-5: Steam ID Configuration

**Description**: On first run, the app prompts for a Steam ID and API key and stores them in a local config file. On subsequent runs, config is loaded automatically. **[User]** **[Codebase]**

**User Story**: As a personal user, I want to enter my Steam ID once and have it remembered so that I don't have to provide it on every run.

**Acceptance Criteria**:

| ID | Criterion | Given | When | Then |
|----|-----------|-------|------|------|
| AC-5.1 | First-run prompt | No config file exists | App launches | User is prompted to enter their Steam ID (17-digit SteamID64) and Steam API key |
| AC-5.2 | Config persisted | User completes first-run prompt | Prompt is submitted | Config is saved to `~/.config/steam-stats/config.toml` (or platform equivalent) |
| AC-5.3 | Config auto-loaded | Config file exists | App launches | Steam ID and API key are loaded from config file; no prompt shown |
| AC-5.4 | Invalid Steam ID flagged | User enters a non-numeric or wrong-length Steam ID | Prompt submitted | Error is shown and user is prompted again |

**Priority**: Must Have

**Goal**: Primary

**Dependencies**: None

---

### FR-6: Recently Played Section

**Description**: Below the top-games list, the dashboard shows a short list of recently played games (last 2 weeks) sourced from the `GetRecentlyPlayedGames` API response. **[User]**

**User Story**: As a personal user, I want to see which games I've played recently so that I can track my short-term activity at a glance.

**Acceptance Criteria**:

| ID | Criterion | Given | When | Then |
|----|-----------|-------|------|------|
| AC-6.1 | Recent games displayed | `GetRecentlyPlayedGames` returned results | Dashboard renders | Up to 5 recently played games are listed with game name and 2-week playtime |
| AC-6.2 | Empty state handled | No recent activity in the past 2 weeks | Dashboard renders | Section shows "No recent activity" rather than being hidden or crashing |

**Priority**: Should Have

**Goal**: Primary

**Dependencies**: FR-3

---

### FR-7: New Games Detection

**Description**: The "New Games" stat reflects games first observed in the selected timeframe window — i.e., games whose first snapshot entry falls within the period. **[User]** **[Agent Decision]**

**User Story**: As a personal user, I want to know how many new games I started playing in the selected timeframe so that I can see how much I've branched out.

**Acceptance Criteria**:

| ID | Criterion | Given | When | Then |
|----|-----------|-------|------|------|
| AC-7.1 | New games counted | Cache has snapshots spanning the selected period | Dashboard renders | New Games count reflects games whose first cached playtime entry falls within the period window |
| AC-7.2 | Zero state | No new games in the period | Dashboard renders | New Games shows 0, not blank or hidden |

**Priority**: Should Have

**Goal**: Primary

**Dependencies**: FR-4

---

### FR-8: Distribution Packaging

**Description**: The project includes a CI workflow to build release binaries for macOS (arm64, x86_64) and Windows (x86_64) on tagged releases. A Homebrew tap formula and Scoop bucket manifest are maintained so users can install via `brew install` and `scoop install`. **[User]**

**User Story**: As a personal user, I want to install and update the tool via Homebrew on Mac and Scoop on Windows so that I don't have to manage the binary manually.

**Acceptance Criteria**:

| ID | Criterion | Given | When | Then |
|----|-----------|-------|------|------|
| AC-8.1 | CI release builds | A git tag matching `v*` is pushed | GitHub Actions workflow runs | Binaries are built for `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc` and attached to a GitHub Release |
| AC-8.2 | Homebrew formula | A GitHub Release exists | User runs `brew install <tap>/steam-stats` | The binary installs and `steam-stats --version` works |
| AC-8.3 | Scoop manifest | A GitHub Release exists | User runs `scoop install steam-stats` | The binary installs and `steam-stats --version` works |

**Priority**: Should Have

**Goal**: Secondary-2

**Dependencies**: None (independent of app FRs)

---

### FR-9: Achievement Count Display

**Description**: The dashboard shows total achievements unlocked across all games for the selected period. Because fetching achievements requires one API call per game, this is opt-in or runs in the background with a progress indicator. **[User]** **[Inferred]**

**User Story**: As a personal user, I want to see how many achievements I've unlocked in the selected timeframe so that I can track that metric alongside playtime.

**Acceptance Criteria**:

| ID | Criterion | Given | When | Then |
|----|-----------|-------|------|------|
| AC-9.1 | Achievement count shown | Achievement data is available (fetched or cached) | Dashboard renders | Total achievement count is displayed in the header stats row |
| AC-9.2 | Graceful absence | Achievement fetch was skipped or is still loading | Dashboard renders | Achievement stat shows "—" or "loading…" rather than 0 or blank |

**Priority**: Nice to Have

**Goal**: Primary

**Dependencies**: FR-3, FR-4

---

## Non-Functional Requirements

### NFR-1: Startup Performance

**Category**: Performance

**Description**: The app should display a usable dashboard quickly, even before all data is fetched.

**Metric**: Time from launch to first dashboard render

**Target**: < 500ms to display cached/lifetime data; live API fetch may take longer but should show progress

**Verification**: Manual timing on target hardware; startup profiling in CI

---

### NFR-2: API Key Security

**Category**: Security

**Description**: The Steam API key must not be stored in source code, committed to git, or logged. **[Codebase]**

**Metric**: No API key present in any committed file

**Target**: Key stored only in `~/.config/steam-stats/config.toml` (user-owned, 600 permissions on Unix)

**Verification**: `git grep` scan in CI; config file permission check on first write

---

### NFR-3: Cross-Platform Support

**Category**: Reliability

**Description**: The app must run on macOS (arm64 and x86_64) and Windows 10+. Linux is a bonus, not a hard requirement.

**Metric**: Binary builds and `steam-stats --help` executes on each target

**Target**: No platform-specific panics or missing features on macOS and Windows

**Verification**: CI matrix build; manual smoke test on each platform

---

### NFR-4: Cache Durability

**Category**: Reliability

**Description**: The local snapshot cache must survive app crashes and binary updates without data loss.

**Metric**: Cache file integrity after simulated crash (SIGKILL mid-write)

**Target**: Cache writes are atomic (write-then-rename); no partial write leaves the cache unreadable

**Verification**: Unit test simulating interrupted write; manual test after `cargo install` upgrade

---

## Scope

### In Scope
- Rust TUI dashboard (ratatui or equivalent)
- Steam Web API integration (GetOwnedGames, GetRecentlyPlayedGames, GetPlayerAchievements)
- Local snapshot cache for timeframe delta calculations
- Period selector: 4 weeks, 6 months, current year, lifetime
- Top games list ordered by % playtime with progress bars
- Header stats: games played, est. sessions, achievements, new games
- Recently played section (last 2 weeks)
- First-run Steam ID + API key configuration
- GitHub Actions CI release builds (macOS arm64/x86, Windows x86)
- Homebrew tap formula and Scoop bucket manifest

### Out of Scope
- Friends comparison or social features
- Web or mobile interface
- Non-Steam game tracking
- Real-time background monitoring or notifications
- Writing or modifying Steam data
- Exact session counts (Valve internal data not available via public API)
- Linux as a first-class supported platform (best-effort only)

### Constraints
- Steam Web API only — no Valve internal data, no scraping **[User]**
- Steam profile must be set to Public for the API to return data **[Inferred]**
- Steam API rate limit: ~100,000 requests per day (well within personal use) **[Codebase]**
- Sessions are estimated, not exact — this must be labeled clearly in the UI **[Agent Decision]**

### Assumptions
- User is running macOS or Windows as their primary platform
- User will run the app regularly enough (e.g., a few times per week) for the snapshot cache to provide meaningful timeframe data within a few weeks
- Steam ID64 format (17-digit numeric) is used for API calls
- The existing Swift `SteamModels.swift` can serve as a reference for API response shapes **[Codebase]**

### Risks
| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Steam API rate limiting or deprecation | High | Low | Cache API responses; design around minimal call count per run |
| Profile privacy change breaks data fetch | Medium | Low | Clear error message with instructions to set profile to Public |
| Session estimation is misleading if user plays multiple sessions/day | Low | Medium | Label clearly as "est." in UI; document limitation in README |
| Insufficient snapshot history makes timeframe views useless at first | Medium | High (inevitable for new installs) | Show "Based on N days of data" label; lifetime view always works |

## Success Metrics

### Primary Metrics
| Metric | Current Baseline | Target | Measurement Method |
|--------|------------------|--------|-------------------|
| App launches and shows dashboard | N/A (new tool) | Works on first run after config | Manual test |
| Timeframe views show meaningful data | N/A | After 2 weeks of daily use, 4-week view is accurate | Manual review |

### Secondary Metrics
| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Homebrew install works | `brew install` succeeds on first release tag | CI + manual test |
| Scoop install works | `scoop install` succeeds on first release tag | CI + manual test |

## Dependencies

### External Systems
- Steam Web API (`api.steampowered.com`) — `GetOwnedGames`, `GetRecentlyPlayedGames`, `GetPlayerAchievements`
- GitHub Actions — CI release pipeline
- Homebrew (custom tap) and Scoop (custom bucket) — distribution

### Internal Dependencies
- None — this is a standalone binary

### Data Dependencies
- Local snapshot cache file (created at first run, grows over time)
- Steam ID64 and API key (entered by user at first run)

## Open Questions

> Questions that need stakeholder input before implementation

1. What TUI crate to use — `ratatui` (active fork of tui-rs) is the clear choice, but worth confirming before planning begins.
2. What local cache format — SQLite (via `rusqlite`) vs. append-only JSON/NDJSON? SQLite is more queryable; JSON is simpler. Decision affects FR-4 implementation significantly.
3. Should achievement fetching happen on every run (slow) or only when explicitly triggered (e.g., a keybind)? Affects FR-9 UX.
4. What is the Homebrew tap name and Scoop bucket repo name? Needs to be decided before FR-8 work begins.

## Agent Decisions

> Decisions made by the agent during elicitation. Review these — they represent assumptions that may need validation.

| # | Decision | Context | Rationale | Affects |
|---|----------|---------|-----------|---------|
| 1 | Sessions estimated as distinct play-days (days where ≥1 game's playtime increased in cache snapshots) | Steam public API has no session data; user asked if a better heuristic exists | More grounded than hours÷2; leverages the local cache already required for timeframes | FR-1 (sessions display), FR-4 (cache schema) |
| 2 | New games defined as "first snapshot entry falls within the period window" | Steam API has no "first played" timestamp; new games metric requires a definition | Consistent with cache-based approach; natural definition given available data | FR-7 |
| 3 | Cache stored at `~/.local/share/steam-stats/` (macOS/Linux) and `%APPDATA%\steam-stats\` (Windows) | Cache must survive binary reinstalls; XDG base dir is the standard | Follows platform conventions; avoids data loss on `cargo install` upgrades | FR-4 (AC-4.4) |
