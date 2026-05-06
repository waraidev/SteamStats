# Remediation Brief: Steam Stats TUI

> Source: `spec-driven/steam-stats-tui/verify-report.md` | Date: 2026-05-06
> Findings selected: VF-1 through VF-19 (2 CRITICAL, 6 HIGH, 11 MEDIUM)

---

## VF-1 — CRITICAL: Period Switching Does Not Recompute Stats

**Dimension**: AC/NFR Completeness
**Affected ACs**: AC-2.2, AC-2.3, AC-3.4
**Complexity**: High — requires threading SnapshotStore into the event loop and calling `delta_for_period` on each period change

**What to fix**:
`src/tui/app.rs` — `handle_key()` on Left/Right/Tab only updates `app.current_period`. It must also recompute `top_games` and `stats` for the new period.

**Approach**:
1. Add a `snapshots: Vec<Snapshot>` field to `App` (or pass it in via `run_tui`)
2. Create a helper `fn recompute_for_period(&mut self, now_ts: i64)` that calls `delta_for_period(&self.current_period, now_ts)` and updates `self.stats`, `self.top_games`
3. Call `recompute_for_period` in `handle_key()` after updating `current_period`
4. Also call it in the `DataLoaded` handler after setting `self.snapshots`

**Files to change**:
- `src/tui/app.rs` — add `snapshots` field, `recompute_for_period()`, call on period change and DataLoaded
- `src/main.rs` — pass `snapshots` or the loaded snapshot vec into `App::new()`
- `src/tui/app.rs` tests — update `App::new()` call signatures and add a period-switch + stats-change test

---

## VF-2 — CRITICAL: AC-5.4 Steam ID Validation Has Zero Test Coverage

**Dimension**: Test Quality
**Affected ACs**: AC-5.4
**Complexity**: Low — extract predicate, add unit tests

**What to fix**:
`src/config/mod.rs` — extract Steam ID validation into a testable function.

**Approach**:
1. Extract: `fn is_valid_steam_id(s: &str) -> bool { s.len() == 17 && s.chars().all(|c| c.is_ascii_digit()) }`
2. Replace inline check in `prompt_and_save` loop with `is_valid_steam_id`
3. Add tests in `#[cfg(test)]`:
   - `test_valid_steam_id_accepts_17_digits`
   - `test_invalid_steam_id_rejects_empty`
   - `test_invalid_steam_id_rejects_non_numeric`
   - `test_invalid_steam_id_rejects_too_short`
   - `test_invalid_steam_id_rejects_too_long`

---

## VF-3 — HIGH: Non-Step Commit `777939e` Lacks Traceability

**Dimension**: Traceability
**Complexity**: Trivial — update progress file

**What to fix**:
Add a Session Log entry to `spec-driven/steam-stats-tui/progress-bundle-7.md` (or a new `progress-bundle-refactor.md` note) documenting commit `777939e` and what it changed.

**Approach**:
Add to `progress-bundle-7.md` Session Log section:
```
| 2026-05-06 | 777939e | Post-execution refactor — HTTP 500 handling for Steam achievement schema errors, Cargo.lock committed, formatting cleanup. Not a STEP commit; additive behavioral change. |
```

---

## VF-4 — HIGH: `.gitignore` Does Not Protect Against Accidental API Key Commit

**Dimension**: Traceability / Security
**Affected ACs**: NFR-2
**Complexity**: Trivial — add one line to .gitignore

**What to fix**:
`.gitignore` — add an entry to prevent accidental commit of a config.toml in the repo root.

**Approach**:
Add to `.gitignore`:
```
# Local config (API key lives in XDG config dir, not repo root — safety net)
config.toml
steam-stats.toml
```

---

## VF-5 — HIGH: Error State Has No Retry Mechanism

**Dimension**: AC/NFR Completeness
**Affected ACs**: AC-3.3
**Complexity**: Medium — new AppEvent variant, key handler, hint bar update

**What to fix**:
`src/tui/app.rs` and `src/tui/dashboard.rs` — add retry capability when in `AppState::Error`.

**Approach**:
1. Add `AppEvent::Retry` variant (or reuse the background-fetch spawn logic)
2. In `handle_key()`, when `AppState::Error`, handle `KeyCode::Char('r')` → re-spawn background fetch task via `app.tx`; set state back to `AppState::Loading`
3. In `dashboard.rs` hint bar: when `AppState::Error`, render `"r retry  q quit"` instead of period-switch hint

---

## VF-6 — HIGH: AC-3.4 Not Tested at App-Level (No-Refetch on Period Switch)

**Dimension**: Test Quality
**Affected ACs**: AC-3.4
**Complexity**: Low — one new unit test

**What to fix**:
`src/tui/app.rs` tests — add a test verifying period cycling does not enqueue API requests.

**Approach**:
```rust
#[test]
fn test_period_switch_does_not_trigger_refetch() {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut app = App::with_tx(tx, OverallStats::default(), vec![]);
    app.state = AppState::Loaded;
    app.handle_key(KeyCode::Right);
    // Channel should be empty — no DataLoaded or fetch event enqueued
    assert!(rx.try_recv().is_err(), "period switch must not enqueue an event");
    assert!(!app.force_achievement_refresh, "period switch must not trigger achievement refresh");
}
```

---

## VF-7 — HIGH: Non-2xx Error Handling Untested for `get_owned_games` / `get_recently_played`

**Dimension**: Test Quality
**Affected ACs**: AC-3.3
**Complexity**: Low — two new wiremock tests, parallel to existing pattern

**What to fix**:
`src/api/client.rs` tests — add two error-path wiremock tests.

**Approach**:
```rust
#[tokio::test]
async fn test_get_owned_games_non2xx_returns_error() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(503)).mount(&mock_server).await;
    let client = SteamClient::with_base_url("key", "76561198000000000", &mock_server.uri());
    let result = client.get_owned_games().await;
    assert!(matches!(result, Err(SteamApiError::ApiError(_))));
}

#[tokio::test]
async fn test_get_recently_played_non2xx_returns_error() { /* mirror above with 401 */ }
```

---

## VF-8 — HIGH: Game Ranking Sort Order Not Directly Tested

**Dimension**: Test Quality
**Affected ACs**: AC-2.2, AC-2.3
**Complexity**: Low — one new snapshot test

**What to fix**:
`src/cache/snapshot.rs` tests — add a test asserting top_games are sorted descending by playtime delta.

**Approach**:
Build 3 snapshots for 3 games with different playtime values. Call `delta_for_period`. Assert returned `top_games` is sorted descending by `playtime_delta_minutes` and rank values are 1-based (1, 2, 3).

---

## VF-9 through VF-13 — MEDIUM: Stale Progress File Statuses

**Dimension**: Traceability
**Complexity**: Trivial — file edits only

| Finding | File | Change |
|---------|------|--------|
| VF-9 | progress-bundle-1.md | STEP-1..6: `in-progress` → `completed` |
| VF-10 | progress-bundle-2.md | STEP-8: commit `—` → `f43a7fe` |
| VF-11 | progress-bundle-5.md | STEP-14..17: `pending` → `completed`; record SHAs 459801c, 594d76f, ad0520e, c1eb39b |
| VF-12 | progress-bundle-5.md | STEP-18: `pending` → `completed`; commit `bfce333` |
| VF-13 | progress-bundle-7.md, progress-bundle-8.md | STEP-25 SHA → `4cfcdaa`; STEP-26 `pending`→`completed`, SHA `a839748`; STEP-27 `pending`→`completed`, SHA `12c66e6` |

---

## VF-14 — MEDIUM: AC-6.1 Missing "2-Week" Label on Recently Played

**Dimension**: AC/NFR Completeness
**Affected ACs**: AC-6.1
**Complexity**: Trivial — add a label string to `src/tui/dashboard.rs`

**What to fix**:
Add `"(2 wks)"` suffix or column label to the recently played section playtime display in `render_dashboard`.

---

## VF-15 — MEDIUM: API Key Query Param — Add Acknowledgment Comment

**Dimension**: Security
**Affected ACs**: NFR-2
**Complexity**: Trivial — one-line comment

**What to fix**:
`src/api/client.rs` — add a comment near the `.query(&[("key", ...)])` call:
```rust
// Steam API requires the API key as a `key=` query param (their design, not ours).
// The key does NOT appear in any log output — error paths only capture response body.
```

---

## VF-16 — MEDIUM: Config File Not Written with 0600 Permissions

**Dimension**: Security
**Affected ACs**: NFR-2
**Complexity**: Low — one-liner fix

**What to fix**:
`src/config/mod.rs` — after `std::fs::write(path, toml_content)?;`, set permissions:
```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
}
```

---

## VF-17 — MEDIUM: AC-5.1/5.2 Config Discovery Untested

**Dimension**: Test Quality
**Affected ACs**: AC-5.1, AC-5.2
**Complexity**: Low-Medium — test requires path isolation

**What to fix**:
`src/config/mod.rs` tests — add a test that `Config::load()` returns `ConfigError::NotFound` when the XDG config file doesn't exist. Use `HOME` env var override to isolate from the real user config.

---

## VF-18 — MEDIUM: AC-4.1 Background Task Write Not Verified

**Dimension**: Test Quality
**Affected ACs**: AC-4.1
**Complexity**: Low — add a comment or minimal structural test

**What to fix**:
Add a `// MANUAL: background task calls store.append() — verified by integration test or code inspection` comment in the test module, OR create a structural test that injects a mock store and confirms `append` is called after `DataLoaded` wiring.

---

## VF-19 — MEDIUM: Rank Number and Bar Width Not Asserted in Dashboard Tests

**Dimension**: Test Quality
**Affected ACs**: AC-1.1, AC-1.2
**Complexity**: Low — add buffer assertions to existing dashboard tests

**What to fix**:
`src/tui/dashboard.rs` tests — in the existing test that provides `top_games` with multiple games, assert:
- Buffer contains `"1."` or `"1 "` (rank prefix for first game)
- Buffer contains the top game's name

---

## Remediability Summary

| VF | Complexity | Mode |
|----|-----------|------|
| VF-1 | High | Plan first |
| VF-5 | Medium | Plan first |
| VF-2, VF-6, VF-7, VF-8, VF-14, VF-16, VF-17, VF-19 | Low | Fix now |
| VF-3, VF-4, VF-9..13, VF-15, VF-18 | Trivial | Fix now |

---

## Build Checks

Run these after each significant change to confirm no regressions:

```bash
cargo check          # must exit 0
cargo test           # must exit 0, 67+ tests passing
cargo clippy         # review any new warnings
cargo fmt            # fix formatting drift (especially src/api/client.rs)
```

---

## Post-Implementation (Required)

After all VF-1..19 fixes are implemented and build checks pass, re-verify the failed dimensions:

```
/sds.verify steam-stats-tui --focus completeness,testing
```

This re-runs the AC/NFR Completeness and Test Quality agents only — the two dimensions that produced FAIL verdicts. If both come back PASS or PASS WITH CAVEATS, the overall verdict upgrades.
