use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Config struct
// ---------------------------------------------------------------------------

/// Application configuration persisted to `~/.config/steam-stats/config.toml`.
///
/// All fields carry `#[serde(default)]` for forward-compatibility — unknown
/// fields in the file are silently ignored; missing fields fall back to the
/// `Default` impl (empty string).
///
/// **Debug is implemented manually** to redact `steam_api_key` so it is never
/// printed in log output or error messages.
#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub steam_id: String,

    #[serde(default)]
    pub steam_api_key: String,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("steam_id", &self.steam_id)
            .field("steam_api_key", &"[REDACTED]")
            .finish()
    }
}

// ---------------------------------------------------------------------------
// ConfigError
// ---------------------------------------------------------------------------

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("config file not found")]
    NotFound,

    #[error("config parse error: {0}")]
    ParseError(String),

    #[error("config I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Returns the canonical config file path: `~/.config/steam-stats/config.toml`.
///
/// Uses `dirs::config_dir()` (XDG-aware) with a fallback to
/// `dirs::home_dir().join(".config")` so the path is correct on both
/// macOS and Linux.
pub fn default_config_path() -> PathBuf {
    let base =
        dirs::config_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"));
    base.join("steam-stats").join("config.toml")
}

// ---------------------------------------------------------------------------
// Config impl
// ---------------------------------------------------------------------------

fn is_valid_steam_id(s: &str) -> bool {
    s.len() == 17 && s.chars().all(|c| c.is_ascii_digit())
}

impl Config {
    /// Load config from the default path (`~/.config/steam-stats/config.toml`).
    ///
    /// Returns `ConfigError::NotFound` if the file does not exist — the caller
    /// should respond by invoking `Config::prompt_and_save`.
    pub fn load() -> Result<Config, ConfigError> {
        Self::load_from(&default_config_path())
    }

    /// Load config from an explicit path.  Accepts a `&Path` so tests can pass
    /// a `NamedTempFile` path without touching `~/.config/steam-stats/`.
    pub fn load_from(path: &Path) -> Result<Config, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::NotFound);
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config =
            toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))?;
        Ok(config)
    }

    /// Interactive first-run prompt.
    ///
    /// Prompts the user for their Steam ID and API key on `stdin`, validates
    /// the Steam ID (must be a 17-digit numeric string), then writes the config
    /// to `path`, creating parent directories as needed.
    ///
    /// This function **must** be called before `enable_raw_mode()` is invoked
    /// for ratatui — raw mode swallows terminal input and produces garbage.
    pub fn prompt_and_save(path: &Path) -> Result<Config, ConfigError> {
        let steam_id = loop {
            print!("Enter your SteamID64 (17-digit number): ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let trimmed = input.trim().to_string();
            if is_valid_steam_id(&trimmed) {
                break trimmed;
            }
            eprintln!(
                "Invalid Steam ID — must be exactly 17 digits (got {:?}). Please try again.",
                trimmed
            );
        };

        print!("Enter your Steam Web API key: ");
        io::stdout().flush()?;
        let mut api_key_input = String::new();
        io::stdin().read_line(&mut api_key_input)?;
        let steam_api_key = api_key_input.trim().to_string();

        let config = Config {
            steam_id,
            steam_api_key,
        };

        // Persist to disk — create parent directories if necessary.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let toml_content =
            toml::to_string(&config).map_err(|e| ConfigError::ParseError(e.to_string()))?;
        std::fs::write(path, toml_content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }

        Ok(config)
    }
}

// ---------------------------------------------------------------------------
// Tests (STEP-8)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn write_toml(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("could not create tempfile");
        f.write_all(content.as_bytes())
            .expect("could not write tempfile");
        f
    }

    #[test]
    fn test_load_from_valid_toml_returns_ok() {
        let f = write_toml(
            r#"
steam_id = "76561198012345678"
steam_api_key = "AAAA1111BBBB2222CCCC3333DDDD4444"
"#,
        );
        let config = Config::load_from(f.path()).expect("should succeed");
        assert_eq!(config.steam_id, "76561198012345678");
        assert_eq!(config.steam_api_key, "AAAA1111BBBB2222CCCC3333DDDD4444");
    }

    #[test]
    fn test_load_from_missing_file_returns_not_found() {
        let tmp = std::env::temp_dir().join("steam_stats_nonexistent_config_xyz.toml");
        // Ensure the file does not exist.
        let _ = std::fs::remove_file(&tmp);
        let err = Config::load_from(&tmp).expect_err("should fail with NotFound");
        assert!(
            matches!(err, ConfigError::NotFound),
            "expected NotFound, got {err:?}"
        );
    }

    #[test]
    fn test_load_from_extra_fields_succeeds() {
        // serde(default) means unknown keys are ignored — backward compat.
        let f = write_toml(
            r#"
steam_id = "76561198012345678"
steam_api_key = "key123"
unknown_future_field = "some_value"
"#,
        );
        let config = Config::load_from(f.path()).expect("extra fields should not cause failure");
        assert_eq!(config.steam_id, "76561198012345678");
    }

    // ── is_valid_steam_id (VF-2) ──────────────────────────────────────────────

    #[test]
    fn test_valid_steam_id_accepts_17_digits() {
        assert!(is_valid_steam_id("76561198012345678"));
    }

    #[test]
    fn test_invalid_steam_id_rejects_empty() {
        assert!(!is_valid_steam_id(""));
    }

    #[test]
    fn test_invalid_steam_id_rejects_non_numeric() {
        assert!(!is_valid_steam_id("7656119801234567A"));
    }

    #[test]
    fn test_invalid_steam_id_rejects_too_short() {
        assert!(!is_valid_steam_id("7656119801234567")); // 16 digits
    }

    #[test]
    fn test_invalid_steam_id_rejects_too_long() {
        assert!(!is_valid_steam_id("765611980123456789")); // 18 digits
    }

    // ── Config::load() discovery (VF-17) ─────────────────────────────────────

    #[test]
    fn test_config_load_returns_not_found_when_no_config_file() {
        // Use load_from() with a path in a temp dir to verify Config discovery
        // returns NotFound when the file is absent (AC-5.1, AC-5.2).
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("steam-stats").join("config.toml");
        let err = Config::load_from(&absent).expect_err("should be NotFound");
        assert!(
            matches!(err, ConfigError::NotFound),
            "expected NotFound, got {err:?}"
        );
    }

    #[test]
    fn test_debug_redacts_api_key() {
        let config = Config {
            steam_id: "76561198012345678".to_string(),
            steam_api_key: "real-key-secret-value".to_string(),
        };
        let debug_str = format!("{config:?}");
        assert!(
            debug_str.contains("[REDACTED]"),
            "Debug output should contain [REDACTED]: {debug_str}"
        );
        assert!(
            !debug_str.contains("real-key"),
            "Debug output must not contain actual key: {debug_str}"
        );
    }
}
