### Bundle 3: API Client
> Stage: depth | Parallel: yes (file-disjoint — src/api/ only; start after Bundle 1) | Files: src/api/client.rs, src/api/mod.rs

**Bundle Verify**: SteamClient calls correct endpoints with correct query params against a mock server.
- **Level**: integration
- **Given**: wiremock MockServer running with mocked Steam API endpoints
- **Action**: `cargo test api::`
- **Outcome**: All API tests pass; wiremock asserts confirm `steamid` and `key` query params are present in requests; GetPlayerAchievements 403 returns SteamApiError::NotAvailable (not panic)

> **Context**
>
> **Applicable ACs**
> - **AC-3.1**: Given: Valid Steam ID and API key configured / When: App launches / Then: GetOwnedGames called with include_appinfo=1 and results stored
> - **AC-3.2**: Given: Valid Steam ID configured / When: App launches / Then: GetRecentlyPlayedGames called and results stored
> - **AC-3.3**: Given: Non-2xx or unreachable / When: App launches / Then: Error message shown with retry option; app does not crash
> - **AC-3.4**: Given: GetOwnedGames already fetched in current run / When: Period switch triggered / Then: No new network request — served from in-memory result
>
> **Architecture Decisions**
> - **AD-1: Adopt rally-tui tech stack verbatim** — reqwest 0.12 with rustls-tls. No OpenSSL dependency.
> - **AD-5: Smart-incremental achievement fetching** — GetPlayerAchievements is called only for recently-active games. Per-game 403/400 responses (private stats, no achievements) must return `SteamApiError::NotAvailable`, not a fatal error, so the fan-out continues for other games.
>
> **Findings**
> - **F-6: Steam JSON envelopes** — All Steam API responses nest under a top-level "response" key. Deserializing without the envelope wrapper silently produces empty data.
> - **F-5: wiremock pattern** — rally-tui uses `wiremock 0.6` with `SteamClient::with_base_url(mock_url)` for HTTP testing. `MockServer::start().await` in `#[tokio::test]` async tests.
>
> **Standards**
> - **S-6**: HTTP API mocking in tests: use wiremock (Domain: testing | File Type: .rs)
> - **S-3**: API keys must not appear in source code, git history, or log output (Domain: security)
>
> **Constraints**
> - reqwest must use rustls-tls feature with default-features = false (Category: compatibility | Source: codebase)
> - Achievement fan-out must be bounded (batch of N, not all at once) to avoid rate limiting (Category: performance | Source: technical)
>
> **Risks**
> - Steam API returns 403 on achievement calls for private DLC or hidden stats (Impact: Low | Mitigation: Treat non-2xx per-game achievement responses as SteamApiError::NotAvailable; continue fan-out)

---

#### STEP-9: Create src/api/client.rs + src/api/mod.rs
[FR-3 -> AC-3.1, AC-3.2, AC-3.3, AC-3.4] | create `src/api/client.rs`, create `src/api/mod.rs` | Effort: M

> **Intent**: Steam API wraps all data under `"response"` envelope (F-6) — the client deserializes `OwnedGamesResponse` (the full envelope), not a bare games array. AC-3.4's "no duplicate request" is NOT HTTP-level rate limiting — it's an in-memory flag `fetched_owned: AtomicBool` on the client that prevents redundant `get_owned_games()` calls in the same process run. `SteamClient::with_base_url(url, steam_id, api_key)` is required for wiremock testing — without it, tests hit the real Steam API and are flaky (S-6 requirement from rally-tui F-5).

- `SteamClient { client: reqwest::Client, steam_id: String, api_key: String, base_url: String }` — `new(steam_id, api_key) -> Self` uses `"https://api.steampowered.com"`; `with_base_url(base: &str, steam_id: &str, api_key: &str) -> Self` for testing
- `async fn get_owned_games(&self) -> Result<Vec<OwnedGame>, SteamApiError>` — `GET {base}/IPlayerService/GetOwnedGames/v1?steamid=...&key=...&include_appinfo=1&include_played_free_games=1`; deserializes `OwnedGamesResponse`, returns `.response.games`
- `async fn get_recently_played(&self) -> Result<Vec<RecentGame>, SteamApiError>` — `GET {base}/IPlayerService/GetRecentlyPlayedGames/v1?steamid=...&key=...`
- `async fn get_player_achievements(&self, appid: u32) -> Result<u32, SteamApiError>` — `GET {base}/ISteamUserStats/GetPlayerAchievements/v1?steamid=...&key=...&appid=...`; count stats where `achieved == 1`; return `SteamApiError::NotAvailable` on 403/400 (private/no-achievement games)
- `SteamApiError` via thiserror: `Http(#[from] reqwest::Error)`, `Parse(String)`, `NotAvailable`, `ApiError(String)`; `mod.rs`: `pub mod client; pub use client::{SteamClient, SteamApiError};`

**Pattern reference**: `../rally-tui/src/api/client.rs`

**Verify**:
- Level: integration | Given: wiremock server mocking GetOwnedGames with valid Steam JSON envelope | Action: `SteamClient::with_base_url(mock_url, ...).get_owned_games().await` | Outcome: Returns `Ok(vec)` with correct game appids
- Level: unit | Given: Mock server returns HTTP 403 on GetPlayerAchievements | Action: `.get_player_achievements(730)` | Outcome: Returns `Err(SteamApiError::NotAvailable)` — does not panic or return `Err(Http(...))`

> **Standards**:
> - S-6: wiremock `with_base_url` pattern for testability
> - S-3: api_key used only as query param, never logged

> Depends on: STEP-6 | Enables: STEP-10, STEP-23, STEP-24 | Parallel with: STEP-7, STEP-11, STEP-14

---

#### STEP-10: Test API Client with wiremock
MANUAL -> Test for STEP-9 (test-after) | modify `src/api/client.rs` | Effort: M

> **Intent**: wiremock tests must assert request shape (query params) in addition to response parsing — a client that omits the `steamid` query param would pass a response-only test. Including a security check that `key` appears in the request params also confirms the API key is actually sent to the server (not silently dropped).

- Add `#[cfg(test)] mod tests` with `#[tokio::test]` async tests
- Use `MockServer::start().await` + `Mock::given(method("GET")).and(path_contains("GetOwnedGames")).and(query_param("steamid", ...))` to assert request structure
- Test `get_owned_games`: mock returns valid Steam JSON envelope; assert returned `Vec<OwnedGame>` length and first game's appid match expected
- Test `get_recently_played`: similar pattern
- Test `get_player_achievements` with 403 response → `SteamApiError::NotAvailable`
- Test `get_player_achievements` with valid response → returns correct achievement count (count of `achieved == 1` entries)

**Verify**:
- Level: integration | Given: wiremock server with mocked Steam endpoints | Action: `cargo test api::` | Outcome: All tests pass; wiremock asserts confirm request params include `steamid` and `key`

> **Standards**:
> - S-6: wiremock for HTTP testing

> Depends on: STEP-9 | Enables: — | Parallel with: STEP-8, STEP-13, STEP-18
