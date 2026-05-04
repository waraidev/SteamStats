# Standards: steam-stats-tui

Extracted from `../rally-tui/CLAUDE.md` (project reference) and `CLAUDE.md` (this project).

---

### S-1: Use `#[serde(default)]` on all config struct fields

- **Domain**: api-design
- **File Type**: .rs
- **Action Type**: create
- **Source**: `../rally-tui/CLAUDE.md` — "All config fields have `#[serde(default)]` so missing/partial configs work"
- **Rationale**: Ensures backward compatibility when new config fields are added; users who haven't updated their config file don't get deserialization errors.

---

### S-2: Implement custom `Debug` that redacts `steam_api_key` to `[REDACTED]`

- **Domain**: security
- **File Type**: .rs
- **Action Type**: create
- **Source**: `../rally-tui/CLAUDE.md` — "API key redaction: Config's Debug impl redacts rally_api_key to prevent credential leakage in logs"
- **Rationale**: Prevents accidental API key exposure in debug output, error messages, or log files.

---

### S-3: API keys must not appear in source code, git history, or log output

- **Domain**: security
- **File Type**: *
- **Action Type**: *
- **Source**: spec NFR-2, `../rally-tui/CLAUDE.md`
- **Rationale**: Keys in source code or logs are a common credential leak vector. Config file with 600 permissions is the only acceptable storage.

---

### S-4: Use `dirs` crate for config/data paths — never hardcode `~/` paths or platform strings

- **Domain**: other
- **File Type**: .rs
- **Action Type**: create
- **Source**: `../rally-tui/src/config/mod.rs` — uses `dirs::home_dir()` for config path
- **Rationale**: `dirs` handles platform differences (macOS, Linux, Windows) correctly. Hardcoded paths fail on Windows or when HOME is non-standard.
- **Specific paths**: Config: `dirs::home_dir().join(".config/steam-stats/config.toml")`. Data (snapshots): `dirs::data_dir().join("steam-stats/snapshots.ndjson")`.

---

### S-5: Terminal event poll interval: 250ms

- **Domain**: other
- **File Type**: .rs
- **Action Type**: create
- **Source**: `../rally-tui/src/tui/app.rs:1072` — `event::poll(Duration::from_millis(250))`
- **Rationale**: 250ms is the sweet spot between responsive keyboard handling and unnecessary CPU wake-ups. Spinner animations at this rate look smooth without burning CPU.

---

### S-6: HTTP API mocking in tests: use `wiremock` (dev-dependency)

- **Domain**: testing
- **File Type**: .rs
- **Action Type**: create
- **Source**: `../rally-tui/Cargo.toml` — `wiremock = "0.6"` in dev-dependencies; `../rally-tui/src/api/test_helpers.rs` — `test_client(&MockServer)` pattern
- **Rationale**: Avoids real HTTP calls in tests. wiremock spins up a local HTTP server, allows asserting request counts and shapes. `SteamClient::with_base_url()` injects the mock server URL.

---

### S-7: TUI rendering tests: use `ratatui::backend::TestBackend`

- **Domain**: testing
- **File Type**: .rs
- **Action Type**: create
- **Source**: `../rally-tui/src/tui/app.rs` — `use ratatui::backend::TestBackend; let backend = TestBackend::new(80, 30); let mut terminal = Terminal::new(backend).unwrap()`
- **Rationale**: Enables widget rendering assertions without a real terminal. Buffer content can be extracted as a string and matched against expected output patterns.

---

### S-8: Temp files in tests: use `tempfile` crate (dev-dependency)

- **Domain**: testing
- **File Type**: .rs
- **Action Type**: create
- **Source**: `../rally-tui/Cargo.toml` — `tempfile = "3"` in dev-dependencies; used in `config/mod.rs` tests for `NamedTempFile`
- **Rationale**: `tempfile::tempdir()` and `NamedTempFile` clean up automatically on drop. Prevents test pollution and makes config/cache tests hermetic.
