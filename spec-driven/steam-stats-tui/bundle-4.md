### Bundle 4: Snapshot Store
> Stage: depth | Parallel: yes (file-disjoint — src/cache/ only; start after Bundle 1) | Files: src/cache/snapshot.rs, src/cache/mod.rs

**Bundle Verify**: SnapshotStore reads and writes atomically with correct delta computation and session estimation.
- **Level**: unit
- **Given**: tempdir with crafted snapshot NDJSON files
- **Action**: `cargo test cache::`
- **Outcome**: All cache tests pass; `delta_for_period` returns correct playtime delta; `session_estimate` correctly deduplicates same-calendar-day increases; corrupted NDJSON lines are skipped without error

> **Context**
>
> **Applicable ACs**
> - **AC-4.1**: Given: Successful data fetch / When: End of run / Then: Snapshot written with timestamp and per-game playtime values
> - **AC-4.2**: Given: At least two snapshots in period / When: User selects period / Then: Stats reflect playtime delta between oldest and newest snapshot in window
> - **AC-4.3**: Given: Multiple snapshots showing playtime increases / When: Dashboard renders / Then: Est. Sessions = count of distinct calendar days where ≥1 game's playtime increased
> - **AC-4.4**: Given: User reinstalls / When: App launches / Then: Cache at stable user-data path, data preserved
> - **AC-4.5**: Given: Cache file malformed / When: App launches / Then: Falls back to live API data; logs warning; does not crash
> - **AC-7.1**: Given: Cache has snapshots spanning period / When: Dashboard renders / Then: New Games count = games whose first cached playtime entry falls within period window
> - **AC-7.2**: Given: No new games in period / When: Dashboard renders / Then: New Games shows 0, not blank
>
> **Architecture Decisions**
> - **AD-4: NDJSON snapshot store with atomic writes** — Decision: NDJSON at `dirs::data_dir().join("steam-stats/snapshots.ndjson")`; each run appends one line; writes via `.tmp` + `std::fs::rename()` for atomicity. Schema: `{"ts": u64, "games": {"<appid>": {"pt": u64, "ach": u64|null}}}`. Rationale: F-9 confirms O(n) linear scan is fast enough for years of daily data; no SQLite C dependency; atomic rename prevents partial-write corruption.
>
> **Findings**
> - **F-4: rally-tui cache I/O patterns** — File I/O patterns (create_dir_all, write to temp, rename) are directly reusable from rally-tui CacheManager; TTL semantics are different but file operations are the same.
> - **F-9: NDJSON vs SQLite query performance** — Linear scan of Vec<Snapshot> is O(n × m) — fast enough for 3 years of daily data (<10ms); SQLite overkill.
>
> **Standards**
> - **S-4**: Use dirs crate for config/data paths (Domain: other | File Type: .rs)
> - **S-8**: Temp files in tests: use tempfile crate (Domain: testing | File Type: .rs)
>
> **Constraints**
> - Snapshot file writes must be atomic (temp + rename) — non-atomic writes leave partial JSON lines that corrupt the NDJSON format on crash (Category: infrastructure | Source: technical)
> - `dirs::data_dir()` may return None on unusual platforms — must have a fallback path to avoid panic (Category: infrastructure | Source: technical)
>
> **Risks**
> - NDJSON file grows large after years of daily use (~50KB per snapshot for 500-game library) (Impact: Low | Mitigation: Prune snapshots older than 2 years on load; one-liner for users to clear manually)

---

#### STEP-11: Create src/cache/snapshot.rs
[FR-4 -> AC-4.1, AC-4.2, AC-4.3, AC-4.4, AC-4.5 | FR-7 -> AC-7.1, AC-7.2] | create `src/cache/snapshot.rs` | Effort: L

> **Intent**: NDJSON atomic write (AD-4): read existing file content + append new line in memory → write to `.tmp` → `std::fs::rename(.tmp, path)`. A crash between write and rename leaves `.tmp` orphaned — the main file is untouched. AC-4.5: each NDJSON line is parsed independently; a `serde_json::from_str` error on one line must log a warning and continue, never `?`-propagate and discard the entire history. `dirs::data_dir()` may return `None` — fallback to `PathBuf::from(".")` with a warning. Session estimation uses `chrono::NaiveDate::from_timestamp_opt` to convert Unix timestamps to calendar dates — same calendar day in the user's local timezone counts as one session (use UTC for consistency to avoid timezone complexity).

- `Snapshot { ts: u64, games: HashMap<String, GameEntry> }` + `GameEntry { pt: u64, ach: Option<u64> }` — both serde Serialize/Deserialize
- `SnapshotStore { path: PathBuf }` — `new() -> Self` uses `dirs::data_dir().unwrap_or(PathBuf::from(".")).join("steam-stats/snapshots.ndjson")`; `with_path(path: PathBuf) -> Self` for testing
- `fn load(&self) -> Vec<Snapshot>` — reads file line-by-line; `serde_json::from_str()` each line; skip + `eprintln!("[warn]...")` on parse errors; return `vec![]` if file missing
- `fn append(&self, snapshot: &Snapshot) -> std::io::Result<()>` — atomic: read existing lines (or empty string), append new JSON line, write all to `.tmp` path, `std::fs::rename(&tmp, &self.path)`; call `std::fs::create_dir_all` on parent first
- `fn delta_for_period(&self, period: &Period, now_ts: u64) -> (Vec<GameStats>, OverallStats)` — calls `self.load()`, filters to window, computes per-game min/max pt, builds GameStats vec sorted by `playtime_delta_minutes` descending, computes `est_sessions` and `new_games`
- `fn session_estimate(snapshots: &[Snapshot]) -> u64` — for each consecutive pair, if any game's `pt` increased, record the calendar date of the later snapshot; count distinct dates using a `HashSet<NaiveDate>`
- `fn new_games_in_period(snapshots: &[Snapshot], window_start: u64) -> u64` — count appids whose first appearance (any snapshot) has `ts >= window_start`

**Pattern reference**: `../rally-tui/src/cache/mod.rs` (file I/O patterns); `references/research.md` (delta algorithm pseudocode)

**Verify**:
- Level: unit | Given: tempdir NDJSON with 3 snapshots spanning 2 calendar days, both showing playtime increases | Action: `session_estimate()` | Outcome: Returns 2 (not 3 — same-day increases deduplicated)
- Level: unit | Given: NDJSON file with one malformed JSON line between two valid snapshots | Action: `load()` | Outcome: Returns the 2 valid snapshots; does not return Err or panic
- Level: unit | Given: `append()` called on a new path (file doesn't exist yet) | Action: `load()` immediately after | Outcome: Returns the written snapshot; no `.tmp` file remains

> **Standards**:
> - S-4: `dirs::data_dir()` for data path
> - S-8: `tempfile::tempdir()` in tests

> Depends on: STEP-4, STEP-1 | Enables: STEP-12, STEP-13, STEP-23 | Parallel with: STEP-7, STEP-9, STEP-14

---

#### STEP-12: Create src/cache/mod.rs
MANUAL -> Structural re-exports | create `src/cache/mod.rs` | Effort: XS

> **Intent**: N/A — structural step.

- `pub mod snapshot;`
- `pub use snapshot::{SnapshotStore, Snapshot, GameEntry};`

**Verify**:
- Level: inspection | Given: cache/mod.rs created | Action: `cargo check` | Outcome: No import errors

> Depends on: STEP-11 | Enables: STEP-13, STEP-23 | Parallel with: —

---

#### STEP-13: Test SnapshotStore
MANUAL -> Test for STEP-11 (test-after) | modify `src/cache/snapshot.rs` | Effort: M

> **Intent**: The delta computation boundary condition: the "earliest" snapshot in the window may represent playtime accumulated before the window started. The delta is `latest.pt - earliest.pt` within the window — not `latest.pt - 0`. A test with a game that has 1000 minutes at window start and 1050 at window end should produce delta = 50, not 1050. This is the most likely regression point.

- Add `#[cfg(test)] mod tests` using `tempfile::tempdir()`
- Test `append` + `load` round-trip: written snapshot is readable and fields match
- Test `append` atomicity: no `.tmp` file left after successful write (use `path.with_extension("ndjson.tmp").exists()` assertion)
- Test `delta_for_period` with 2 snapshots: game starts with 1000pt, ends with 1050pt → `playtime_delta_minutes = 50`
- Test `session_estimate` calendar deduplication: 3 snapshots on 2 calendar days with increases → `est_sessions = 2`
- Test `new_games_in_period`: game first seen before window → not counted; game first seen within window → counted
- Test `load` with malformed line: valid snapshots returned, no panic

**Verify**:
- Level: unit | Given: tempdir + constructed snapshots with known playtime values | Action: `cargo test cache::` | Outcome: All tests pass; delta boundary test returns 50 not 1050

> **Standards**:
> - S-8: tempfile in tests

> Depends on: STEP-12 | Enables: — | Parallel with: STEP-8, STEP-10, STEP-18
