# Research Results: steam-stats-tui

## Aspect 1: TUI Framework & Event Loop

**Question**: What TUI framework and event loop pattern should we use for a Rust terminal dashboard with background API calls?

**Source**: Direct analysis of `../rally-tui` — a production Rust TUI in the same workspace.

### Findings

**F-1**: rally-tui uses exactly `ratatui 0.29 + crossterm 0.28 + tokio 1.x (full)`. This is a confirmed working stack for a Rust TUI with:
- Background HTTP calls via reqwest
- Terminal raw mode + alternate screen
- Keyboard event handling
- Spinner animation during loading
- Multiple view modes

Confidence: high. Directly observed in `../rally-tui/Cargo.toml`.

**F-2**: The event loop pattern in `../rally-tui/src/tui/app.rs:run_tui()`:
1. `enable_raw_mode()` + `EnterAlternateScreen`
2. `CrosstermBackend::new(stdout)` → `Terminal::new(backend)`
3. Loop: `process_events()` → `terminal.draw()` → `event::poll(250ms)` → `handle_key()`
4. `disable_raw_mode()` + `LeaveAlternateScreen` on exit

`process_events()` drains a `mpsc::UnboundedReceiver<AppEvent>` using `try_recv()` in a while loop. This handles rapid event bursts without blocking the render cycle.

Confidence: high. Directly observed, well-tested.

**F-5 (ratatui testing)**: rally-tui uses `ratatui::backend::TestBackend::new(width, height)` → `Terminal::new(backend)` → `terminal.draw()` → `terminal.backend().buffer().content` string extraction. This enables widget rendering assertions in unit tests without a real terminal.

### Approaches Evaluated

| Approach | Fit | Tradeoffs |
|---|---|---|
| ratatui + crossterm + tokio (rally-tui stack) | ✅ Recommended | Proven locally, active community, direct copy-adapt |
| cursive | ❌ | Different abstraction model, no tokio integration, less active |
| tui-rs | ❌ | Archived; ratatui is the active fork |
| EventStream async (crossterm) | ❌ | Adds complexity vs poll(), no benefit for single-view |

### Resolved Uncertainties

- TUI crate: `ratatui` (confirmed by F-1)
- Event loop: copy rally-tui pattern exactly (F-2)

---

## Aspect 2: Local Snapshot Cache

**Question**: How should the local snapshot store be designed to support timeframe delta computation and session estimation?

**Source**: rally-tui cache module analysis + training knowledge on NDJSON/SQLite tradeoffs.

### Findings

**F-4**: rally-tui's `CacheManager` (`src/cache/mod.rs`) uses a TTL-based file cache (Hit/Stale/Miss) with separate `.json` and `.meta` sidecar files. The TTL semantics are wrong for steam-stats which needs append-only history — but the file I/O patterns (create_dir_all, write, temp sidecar) are directly reusable.

**F-9**: Query requirements for our delta computation:
- Filter snapshots by timestamp range: `snapshots.iter().filter(|s| s.ts >= window_start)`
- Group by game: `HashMap<u32, Vec<u64>>` (appid → playtime values)
- Find first appearance of a game: `snapshots.iter().find(|s| s.games.contains_key(appid))`

These are all linear-scan O(n) operations on `Vec<Snapshot>`. For 3 years of daily data (≈1000 snapshots), even a full scan is <10ms. SQLite's query power is not needed.

### NDJSON Schema

Each line in `snapshots.ndjson`:
```json
{"ts": 1746360000, "games": {"730": {"pt": 12345, "ach": 42}, "4000": {"pt": 567, "ach": null}}}
```

- `ts`: Unix timestamp (seconds) — written at end of each successful run
- `games`: object keyed by appid string (Steam uses numeric appids; stringify for JSON keys)
- `pt`: playtime_forever in minutes
- `ach`: achievement count, or `null` if not yet fetched

### Delta Computation Algorithm

```
fn delta_for_period(snapshots, start_ts, end_ts) -> Vec<GameDelta>:
  window = snapshots.filter(ts >= start_ts && ts <= end_ts)
  if window.len() < 2: return []
  
  for each appid:
    first = earliest snapshot in window where games[appid] exists
    last = latest snapshot in window where games[appid] exists
    delta_minutes = last.pt - first.pt
    
  session_estimate = count distinct calendar days where
    consecutive snapshots show any game's pt increased
```

### Atomic Write Pattern

```rust
let tmp_path = path.with_extension("ndjson.tmp");
let line = serde_json::to_string(&snapshot)?;
// append to tmp (copy existing + new line)
std::fs::rename(&tmp_path, &path)?;  // atomic on POSIX and Windows
```

For append-only (most common case), just `OpenOptions::new().append(true).open(&path)` and write one line — no copy needed.

### Approaches Evaluated

| Approach | Fit | Tradeoffs |
|---|---|---|
| NDJSON (JSON Lines) | ✅ Recommended | No C dep, O(1) append, readable, good enough query perf |
| SQLite (rusqlite) | ❌ | ~2MB + libsqlite3-sys C dep, overkill for flat time-series |
| Plain JSON array | ❌ | Must rewrite entire file on each run, partial-write corruption |
| Bincode | ❌ | Not human-readable, no tooling for debug |

---

## Aspect 3: Steam API Client

**Question**: How should the Rust Steam API client be structured, and how should achievement fetching be handled?

**Source**: Swift SteamAPIClient.swift reference + rally-tui API client analysis.

### Findings

**F-6**: Steam API response envelope — all responses wrap data under `{"response": {...}}`. Field names are snake_case. From Swift reference:
- `GetOwnedGames`: `response.game_count: Int`, `response.games: [Game]`
- `GetRecentlyPlayedGames`: `response.total_count: Int?`, `response.games: [Game]?`
- `GetPlayerSummaries`: `response.players: [Player]`
- `GetPlayerAchievements`: `playerstats.achievements: [Achievement]?`, `playerstats.success: bool`

Note: `GetPlayerAchievements` is the only endpoint that doesn't use `response` as the top-level key.

**F-7**: Game struct fields (from SteamModels.swift):
- `appid: u32`
- `name: Option<String>`
- `playtime_forever: u32` (minutes)
- `playtime_2_weeks: Option<u32>` (minutes, absent if 0)
- `img_icon_url: Option<String>`

**F-8**: Achievement fan-out: for a user with 500 owned games, fetching all achievements requires 500 HTTP requests. At ~100ms/request sequential = 50 seconds. Parallel (10 concurrent) = ~5 seconds. Must be background-only.

### Smart-Incremental Strategy

On each launch:
1. Call `GetRecentlyPlayedGames` → set of recently active appids
2. Load previous snapshot → compare playtime_forever vs stored
3. Union of (recently played appids) + (appids with playtime increase) = "active games"
4. Background-spawn achievement fetches for active games only
5. All other games: serve `ach` from last snapshot that has a non-null value

First launch (no snapshot): fetch achievements for all games in batches of 10, background.

### API Client Structure (from rally-tui pattern)

```rust
pub struct SteamClient {
    client: reqwest::Client,
    api_key: String,
    steam_id: String,
    base_url: String,  // "https://api.steampowered.com" — overridable for wiremock tests
}

impl SteamClient {
    pub fn new(config: &Config) -> Self { ... }
    pub fn with_base_url(mut self, url: &str) -> Self { ... }  // for testing
    pub async fn get_owned_games(&self) -> Result<Vec<Game>, SteamApiError> { ... }
    pub async fn get_recently_played(&self) -> Result<Vec<Game>, SteamApiError> { ... }
    pub async fn get_achievements(&self, appid: u32) -> Result<u32, SteamApiError> { ... }
}
```

The `with_base_url()` builder follows rally-tui's `test_client(&MockServer)` pattern for wiremock integration.

---

## Aspect 4: Configuration and Distribution

**Question**: What config format and distribution approach to use?

### Config

From rally-tui `src/config/mod.rs`:
- `dirs::home_dir().join(".config/rally-cli/config.json")` — XDG-style, not macOS Library
- `serde_json::from_str` with `#[serde(default)]` on all fields
- `Config::load_from(PathBuf)` for test injection
- Custom `Debug` that redacts API key

Steam-stats differences:
- TOML instead of JSON (no legacy compat needed; `toml` crate is pure Rust)
- `dirs::config_dir()` instead of manual `home_dir().join(".config/")` — `dirs::config_dir()` returns `~/.config/` on Linux, `~/Library/Application Support/` on macOS (but we want `~/.config/` for XDG compatibility across platforms, so use `dirs::home_dir().join(".config/steam-stats/config.toml")` to be explicit)

### Distribution

Mac/Linux — `install.sh` pattern (same as rustup, GitHub CLI, etc.):
```sh
#!/usr/bin/env sh
set -e
REPO="https://github.com/<user>/SteamStats"
VERSION=$(curl -s "$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
curl -Lo /tmp/steam-stats "$REPO/releases/download/$VERSION/steam-stats-$ARCH-$OS"
chmod +x /tmp/steam-stats
sudo mv /tmp/steam-stats /usr/local/bin/steam-stats
```

Windows — Scoop from main repo:
```
scoop bucket add steam-stats https://github.com/<user>/SteamStats
scoop install steam-stats
```

GitHub Actions release workflow (`.github/workflows/release.yml`):
- Trigger: `push tags v*`
- Matrix: `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
- Use `dtolnay/rust-toolchain` + `actions/upload-artifact` + `softprops/action-gh-release`
- Post-release step: compute SHA256, update `scoop/steam-stats.json` and `install.sh VERSION`, commit back to main
