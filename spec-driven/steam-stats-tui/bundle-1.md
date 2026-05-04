### Bundle 1: Shared Infrastructure
> Stage: skeleton | Parallel: no (foundation — all depth bundles depend on this) | Files: Cargo.toml, src/models/steam.rs, src/models/stats.rs, src/models/mod.rs

**Bundle Verify**: Shared project scaffold and all data model types compile cleanly.
- **Level**: inspection
- **Given**: Cargo.toml and all model files created
- **Action**: `cargo check`
- **Outcome**: Project compiles with zero errors; `OwnedGame`, `GameStats`, `OverallStats`, `Period` types are all importable with no "unresolved import" errors

> **Context**
>
> **Applicable ACs**
> - **AC-3.1**: Given: Valid Steam ID and API key configured / When: App launches / Then: GetOwnedGames is called with include_appinfo=1 and results are stored
> - **AC-3.2**: Given: Valid Steam ID configured / When: App launches / Then: GetRecentlyPlayedGames is called and results are stored
> - **AC-1.1**: Given: App has loaded data for the selected period / When: The dashboard renders / Then: Four stat blocks are shown: Games Played, Est. Sessions, Achievements, New Games — each as a large number with a label
> - **AC-2.3**: Given: Any state / When: User selects Lifetime / Then: Stats are shown using all-time playtime_forever values — no cache required
> - **AC-4.2**: Given: At least two snapshots exist within the requested period window / When: User selects a period / Then: Stats reflect the playtime delta between the oldest and newest snapshot within the window
> - **AC-4.3**: Given: Multiple snapshots exist showing playtime increases / When: Dashboard renders / Then: Est. Sessions is computed as the count of distinct calendar days where at least one game's playtime increased
> - **AC-7.1**: Given: Cache has snapshots spanning the selected period / When: Dashboard renders / Then: New Games count reflects games whose first cached playtime entry falls within the period window
>
> **Architecture Decisions**
> - **AD-1: Adopt rally-tui tech stack verbatim** — Decision: Use ratatui 0.29, crossterm 0.28, tokio 1.x (full), reqwest 0.12 (json + rustls-tls, no default-features), serde 1 (derive), serde_json 1, dirs 6, thiserror 2, backon 1.3, chrono 0.4 (serde feature), clap 4 (derive), toml 0.8. Rationale: F-1 confirms this stack is working in rally-tui — avoids version discovery and incompatibility debugging.
> - **AD-4: NDJSON snapshot store with atomic writes** — Decision: NDJSON at XDG data dir, append-only, atomic tmp+rename writes. Rationale: F-9 confirms linear scan is fast enough; no SQLite C dependency.
>
> **Findings**
> - **F-6: Steam JSON envelopes** — All Steam API responses nest data under a top-level "response" key; playtime in minutes (integer); snake_case keys — serde structs must mirror this shape exactly or silent deserialization failures occur.
> - **F-7: Swift structs as serde reference** — SteamModels.swift has canonical field names: playtime_forever, playtime_2weeks, img_icon_url, appid — direct 1:1 mapping to Rust serde structs.
> - **F-9: NDJSON vs SQLite** — For our query pattern (filter-by-date, group-by-appid), linear scan of Vec<Snapshot> is O(n × m) — fast enough for years of daily data; SQLite overkill.
>
> **Standards**
> - **S-1**: Use `#[serde(default)]` on all config struct fields (Domain: api-design | File Type: .rs)
>
> **Constraints**
> - `reqwest` must use `rustls-tls` feature with `default-features = false` — prevents OpenSSL linking on Linux musl targets needed for cross-compilation (Category: compatibility | Source: codebase)
> - Session count must be displayed as "Est. Sessions" in the UI — this must be a constant, not a free string (Category: other | Source: spec Constraints)
> - `dirs::data_dir()` may return None on unusual platforms — must have a fallback path (Category: infrastructure | Source: technical)

---

#### STEP-1: Create Cargo.toml
MANUAL -> Project scaffold with all AD-1 dependencies | create `Cargo.toml` | Effort: S

> **Intent**: N/A — structural step. Note: `reqwest` MUST use `rustls-tls` with `default-features = false` — this prevents linking against OpenSSL on Linux musl targets required for the cross-compilation matrix in FR-8. Omitting this flag will cause CI failures on `x86_64-unknown-linux-musl`.

- Set `name = "steam-stats"`, `version = "0.1.0"`, `edition = "2021"` with a `[[bin]]` target
- Production dependencies from AD-1: `ratatui = "0.29"`, `crossterm = { version = "0.28", features = ["event"] }`, `tokio = { version = "1", features = ["full"] }`, `reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `dirs = "6"`, `thiserror = "2"`, `backon = "1.3"`, `chrono = { version = "0.4", features = ["serde"] }`, `clap = { version = "4", features = ["derive"] }`, `toml = "0.8"`
- Dev dependencies: `wiremock = "0.6"`, `tempfile = "3"`
- No workspace members; standalone binary crate

**Verify**:
- Level: inspection | Given: Cargo.toml created | Action: `cargo check` | Outcome: All dependencies resolve; no version conflict errors

> **Standards**:
> - S-6: wiremock in dev-dependencies
> - S-8: tempfile in dev-dependencies

> Depends on: — | Enables: STEP-2, STEP-4, STEP-7, STEP-9, STEP-11, STEP-14 | Parallel with: —

---

#### STEP-2: Create src/models/steam.rs
[FR-3 -> AC-3.1, AC-3.2] | create `src/models/steam.rs` | Effort: S

> **Intent**: Steam API JSON responses wrap all data under a top-level `"response"` key (F-6) — a serde struct that flattens this envelope will silently return empty `Vec` instead of erroring. All playtime values are minutes (not hours). Field names must exactly match F-7's Swift reference (`playtime_forever`, `playtime_2weeks`, `img_icon_url`, `appid`) — a typo in `#[serde(rename)]` silently deserializes as `Default::default()`.

- Define `OwnedGamesResponse { response: OwnedGamesBody }` and `OwnedGamesBody { game_count: u32, games: Vec<OwnedGame> }` — the `response` field uses `#[serde(rename = "response")]` to match Steam's envelope
- `OwnedGame`: `appid: u32`, `name: String`, `playtime_forever: u64` (minutes), `playtime_2weeks: Option<u64>`, `img_icon_url: Option<String>` — use `#[serde(rename_all = "snake_case")]`
- Define `RecentlyPlayedResponse { response: RecentlyPlayedBody }` and `RecentlyPlayedBody { total_count: u32, games: Vec<RecentGame> }`
- `RecentGame`: `appid: u32`, `name: String`, `playtime_2weeks: u64`, `playtime_forever: u64`
- Add `#[derive(Debug, Clone, Serialize, Deserialize)]` to all types; use `#[serde(default)]` on `Option` fields

**Pattern reference**: `SteamStats_iOS/` (in git history) — field names are canonical

**Verify**:
- Level: unit | Given: JSON string with Steam GetOwnedGames "response" envelope | Action: `serde_json::from_str::<OwnedGamesResponse>` | Outcome: Deserialization succeeds and `games[0].playtime_forever` equals the expected minutes value
- Level: unit | Given: OwnedGame JSON missing `playtime_2weeks` field | Action: deserialize | Outcome: `playtime_2weeks` is `None`, not a deserialization error

> **Standards**:
> - S-1: `#[serde(default)]` on all optional fields

> Depends on: STEP-1 | Enables: STEP-3, STEP-9, STEP-11 | Parallel with: —

---

#### STEP-3: Test steam.rs serde deserialization
MANUAL -> Test for STEP-2 (test-after) | modify `src/models/steam.rs` | Effort: S

> **Intent**: Serde field mapping errors are silent — a typo in a rename attribute returns `Default::default()` instead of an error. Tests must assert actual field values, not just that deserialization succeeds. The "response" envelope is especially error-prone — flattening it vs nesting it produces different JSON shapes.

- Add `#[cfg(test)] mod tests` at the bottom of steam.rs
- Test `OwnedGamesResponse`: construct JSON string with `"response": { "game_count": 2, "games": [...] }` envelope; assert `game_count`, `games.len()`, and `games[0].playtime_forever` equals expected minutes value (not hours)
- Test `RecentlyPlayedResponse`: similar envelope + `playtime_2weeks` value assertion
- Test optional field: game JSON without `playtime_2weeks` → deserializes as `None`
- Test `appid` as `u32` (not string) — Steam returns numeric appids

**Verify**:
- Level: unit | Given: Inline test JSON strings matching Steam API response format | Action: `cargo test models::steam` | Outcome: All tests pass; field values match expected values (not just non-null)

> Depends on: STEP-2 | Enables: — | Parallel with: —

---

#### STEP-4: Create src/models/stats.rs
[FR-1 -> AC-1.1 | FR-2 -> AC-2.3 | FR-4 -> AC-4.2, AC-4.3 | FR-7 -> AC-7.1] | create `src/models/stats.rs` | Effort: M

> **Intent**: `Period::Lifetime` is a special case that bypasses delta computation and uses `playtime_forever` directly (AC-2.3) — the delta functions must check for this explicitly, not fall through. The `est_sessions` field MUST be labeled "Est. Sessions" (not "Sessions") per the spec constraint — define this as a constant `LABEL_EST_SESSIONS: &str = "Est. Sessions"` here. `OverallStats.partial_data_note: Option<String>` carries the "Based on N days of data" message for AC-2.4 — None means no note shown.

- Define `Period` enum: `FourWeeks`, `SixMonths`, `ThisYear`, `Lifetime` — impl `window_start_ts(now: u64) -> Option<u64>` (None for Lifetime), `label(&self) -> &str`
- Define `GameStats { appid: u32, name: String, playtime_delta_minutes: u64, playtime_forever_minutes: u64, achievement_count: Option<u64>, rank: usize }`
- Define `OverallStats { games_played: u64, est_sessions: u64, achievements: Option<u64>, new_games: u64, partial_data_note: Option<String> }` — `achievements: Option<u64>` is None when not yet fetched
- Const `LABEL_EST_SESSIONS: &str = "Est. Sessions"` — reference this from all UI code
- Stub `compute_delta_for_period` function signature: `pub fn compute_delta(snapshots: &[crate::cache::snapshot::Snapshot], period: Period, now: u64) -> (Vec<GameStats>, OverallStats)` — full impl goes in cache/snapshot.rs; this file defines the return types

**Pattern reference**: `../rally-tui/src/models/`

**Verify**:
- Level: unit | Given: `Period::Lifetime` | Action: `period.window_start_ts(any_ts)` | Outcome: Returns `None` (Lifetime bypasses window)
- Level: unit | Given: `Period::FourWeeks` | Action: `period.window_start_ts(1746360000)` | Outcome: Returns `Some(ts)` where `ts == 1746360000 - 28*24*3600`

> **Standards**:
> - S-1: `#[serde(default)]` on any serde-derived types

> Depends on: STEP-1 | Enables: STEP-5, STEP-6, STEP-11, STEP-14, STEP-15, STEP-19, STEP-21 | Parallel with: —

---

#### STEP-5: Test stats.rs delta types
MANUAL -> Test for STEP-4 (test-after) | modify `src/models/stats.rs` | Effort: M

> **Intent**: `Period::window_start_ts` must compute the correct timestamp boundary for each period. An off-by-one (e.g., 27 days instead of 28 for FourWeeks) produces an incorrect window that silently excludes valid snapshots. Session count labeled "Est. Sessions" is a spec constraint — test it cannot be bypassed.

- Add `#[cfg(test)] mod tests`
- Test `Period::window_start_ts` for each variant: `FourWeeks` → 28 days ago, `SixMonths` → 182 days ago, `ThisYear` → Jan 1 of current year at UTC midnight, `Lifetime` → None
- Test `Period::label()` returns expected strings including "Lifetime" for `Period::Lifetime`
- Test that `LABEL_EST_SESSIONS` constant equals `"Est. Sessions"` (guards against accidental renaming)

**Verify**:
- Level: unit | Given: Known Unix timestamp for 2026-05-04 | Action: `cargo test models::stats` | Outcome: All window boundary tests pass; `LABEL_EST_SESSIONS == "Est. Sessions"`

> Depends on: STEP-4 | Enables: — | Parallel with: —

---

#### STEP-6: Create src/models/mod.rs
MANUAL -> Structural re-exports | create `src/models/mod.rs` | Effort: XS

> **Intent**: N/A — structural step.

- `pub mod steam; pub mod stats;`
- Re-export: `pub use steam::{OwnedGame, OwnedGamesResponse, RecentGame, RecentlyPlayedResponse}; pub use stats::{Period, GameStats, OverallStats, LABEL_EST_SESSIONS};`

**Verify**:
- Level: inspection | Given: models/mod.rs created | Action: `cargo check` | Outcome: No "use of unresolved import" errors anywhere in src/

> Depends on: STEP-4 | Enables: STEP-7, STEP-9, STEP-11, STEP-14 | Parallel with: —
