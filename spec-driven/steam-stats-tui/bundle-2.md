### Bundle 2: Config
> Stage: depth | Parallel: yes (file-disjoint — src/config/ only; start after Bundle 1) | Files: src/config/mod.rs

**Bundle Verify**: Config loading, first-run prompt, and API key redaction work correctly.
- **Level**: unit
- **Given**: A NamedTempFile containing valid TOML, and a Config instance with a real API key value
- **Action**: `cargo test config::`
- **Outcome**: `load_from(path)` returns Ok(Config); `format!("{:?}", config)` contains `[REDACTED]` and does NOT contain the actual key string

> **Context**
>
> **Applicable ACs**
> - **AC-5.1**: Given: No config file exists / When: App launches / Then: User is prompted to enter their Steam ID (17-digit SteamID64) and Steam API key
> - **AC-5.2**: Given: User completes first-run prompt / When: Prompt is submitted / Then: Config is saved to `~/.config/steam-stats/config.toml`
> - **AC-5.3**: Given: Config file exists / When: App launches / Then: Steam ID and API key are loaded; no prompt shown
> - **AC-5.4**: Given: User enters non-numeric or wrong-length Steam ID / When: Prompt submitted / Then: Error shown, user re-prompted
>
> **Architecture Decisions**
> - **AD-6: TOML config format** — Decision: `~/.config/steam-stats/config.toml` via `toml` crate. Rationale: TOML is ergonomic for hand-editing (comments, no trailing commas). No legacy JSON constraint. Config is small (2 fields).
>
> **Findings**
> - **F-3: Config pattern from rally-tui** — `Config::load_from(path)` split from `Config::load()` enables clean testing without touching `~/.config/`; custom Debug redacting API key prevents credential leakage in logs.
>
> **Standards**
> - **S-1**: Use `#[serde(default)]` on all config struct fields (Domain: api-design | File Type: .rs)
> - **S-2**: Implement custom Debug that redacts `steam_api_key` to `[REDACTED]` (Domain: security | File Type: .rs)
> - **S-3**: API keys must not appear in source code, git history, or log output (Domain: security | File Type: *)
> - **S-4**: Use `dirs` crate for config/data paths — never hardcode `~/` paths (Domain: other | File Type: .rs)
> - **S-8**: Temp files in tests: use `tempfile` crate (Domain: testing | File Type: .rs)
>
> **Constraints**
> - Config first-run prompt must run on stdin BEFORE entering TUI alternate screen — prompting inside raw mode swallows input (Category: other | Source: design AD-6)

---

#### STEP-7: Create src/config/mod.rs
[FR-5 -> AC-5.1, AC-5.2, AC-5.3, AC-5.4] | create `src/config/mod.rs` | Effort: M

> **Intent**: The first-run prompt must run on `stdin` BEFORE `enable_raw_mode()` is called in `run_tui()` — prompting inside ratatui's raw mode swallows terminal input and produces garbage output. Steam ID validation: SteamID64 is a 17-digit numeric string — a 15-digit or non-numeric input must re-prompt, not accept. `Config::load_from(path)` takes an explicit path to enable tests without touching `~/.config/steam-stats/`. Custom `Debug` impl MUST redact `steam_api_key` — deriving `Debug` will leak it in any log or error output.

- `Config { steam_id: String, steam_api_key: String }` with `#[serde(default)]` on both fields; do NOT `#[derive(Debug)]` — implement manually to show `steam_api_key: "[REDACTED]"`
- `pub fn default_config_path() -> PathBuf` — `dirs::home_dir().join(".config/steam-stats/config.toml")` with `dirs::config_dir()` as the preferred variant
- `Config::load() -> Result<Config, ConfigError>` — calls `load_from(&default_config_path())`
- `Config::load_from(path: &Path) -> Result<Config, ConfigError>` — reads and parses TOML; returns `ConfigError::NotFound` if file missing (caller triggers first-run prompt)
- `Config::prompt_and_save(path: &Path) -> Result<Config, ConfigError>` — `print!` prompts, `io::stdin().read_line()`, validates Steam ID (17-digit numeric, re-prompt on invalid), creates parent dirs, writes TOML
- `#[derive(thiserror::Error, Debug)] enum ConfigError`: `NotFound`, `ParseError(String)`, `IoError(#[from] std::io::Error)`

**Pattern reference**: `../rally-tui/src/config/mod.rs`

**Verify**:
- Level: unit | Given: Valid TOML at tempfile path | Action: `Config::load_from(path)` | Outcome: Returns `Ok(Config)` with correct `steam_id` and `steam_api_key`
- Level: unit | Given: Config with api_key = "real-secret-abc123" | Action: `format!("{:?}", config)` | Outcome: Output contains `[REDACTED]`; does NOT contain `"real-secret-abc123"`

> **Standards**:
> - S-1: `#[serde(default)]` on config fields
> - S-2: Custom Debug impl redacting steam_api_key
> - S-3: No key in source
> - S-4: `dirs::config_dir()` for path

> Depends on: STEP-6 | Enables: STEP-8, STEP-23 | Parallel with: STEP-9, STEP-11, STEP-14

---

#### STEP-8: Test Config
MANUAL -> Test for STEP-7 (test-after) | modify `src/config/mod.rs` | Effort: S

> **Intent**: Config tests MUST use `tempfile::NamedTempFile` to avoid polluting `~/.config/steam-stats/` during CI runs. Testing `Debug` redaction is critical — a regression would expose the API key in CI logs or error reports without the author noticing.

- Add `#[cfg(test)] mod tests`
- Test `load_from` with valid TOML tempfile → returns `Ok(Config)` with expected field values
- Test `load_from` with missing file → returns `Err(ConfigError::NotFound)`
- Test `load_from` with TOML file that has extra unknown fields → succeeds (backward compat via `#[serde(default)]`)
- Test `format!("{:?}", config)` does NOT contain the actual api key string (use `assert!(!debug_str.contains("real-key"))`)

**Verify**:
- Level: unit | Given: NamedTempFile + Config instances | Action: `cargo test config::tests` | Outcome: All tests pass; Debug test confirms `[REDACTED]` appears and actual key does not

> **Standards**:
> - S-8: `tempfile::NamedTempFile` in tests

> Depends on: STEP-7 | Enables: — | Parallel with: STEP-10, STEP-13, STEP-18
