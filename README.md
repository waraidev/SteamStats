# steam-stats

A terminal UI (TUI) for viewing your personal Steam statistics — top games, playtime, session counts, and more — right in your terminal.

```
┌─ steam-stats ─────────────────────────────────────────────────────────────┐
│ Player: YourSteamName              Games owned: 312   Total playtime: 2847h│
├───────────────────────────────────────────────────────────────────────────┤
│ # Game                              Playtime   Est. Sessions  Last played  │
│ 1 Counter-Strike 2                  843h       2,109          3 days ago   │
│ 2 Elden Ring                        214h       428            2 weeks ago  │
│ 3 Hollow Knight                      98h       196            1 month ago  │
└───────────────────────────────────────────────────────────────────────────┘
  [q] Quit  [r] Refresh  [↑↓] Scroll  [Tab] Switch view
```

## Install

### macOS / Linux (curl | sh)

```sh
curl -fsSL https://raw.githubusercontent.com/ericwiley/SteamStats/main/install.sh | sh
```

This detects your architecture (Apple Silicon or Intel) and installs the correct binary to `/usr/local/bin/steam-stats`.

### Windows (Scoop)

```powershell
scoop bucket add steam-stats https://github.com/ericwiley/SteamStats
scoop install steam-stats
```

## Quick Start

On first run, `steam-stats` will prompt you to create a config file:

```sh
steam-stats
# → No config found at ~/.config/steam-stats/config.toml
# → Creating config template...
# → Edit ~/.config/steam-stats/config.toml and re-run.
```

Edit `~/.config/steam-stats/config.toml` (Windows: `%APPDATA%\steam-stats\config.toml`):

```toml
steam_api_key = "YOUR_STEAM_API_KEY_HERE"
steam_id = "76561198000000000"
```

Then run `steam-stats` again.

## Configuration

### Getting a Steam API Key

1. Visit https://steamcommunity.com/dev/apikey
2. Register for a Steam Web API Key
3. Copy the key into `config.toml` under `steam_api_key`

**Important:** Store your API key only in `~/.config/steam-stats/config.toml`. Do not set it as an environment variable or hardcode it in any script.

### Finding Your Steam ID

Your Steam ID is the 17-digit number in your profile URL:

```
https://steamcommunity.com/profiles/76561198000000000
                                     ↑ this is your Steam ID
```

If you use a custom URL, use https://steamid.io/ to convert it to the numeric ID.

### Config file location

| Platform | Path |
|----------|------|
| macOS / Linux | `~/.config/steam-stats/config.toml` |
| Windows | `%APPDATA%\steam-stats\config.toml` |

## Usage

```
steam-stats          # Launch the TUI
steam-stats --help   # Show help
steam-stats --version
```

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `q` / `Ctrl-C` | Quit |
| `r` | Refresh data from Steam API |
| `↑` / `↓` | Scroll game list |
| `Tab` | Switch between views |

## Requirements

> **Your Steam profile must be set to Public** for the API to return game data. Check your privacy settings at https://steamcommunity.com/my/edit/settings.

## Limitations

- **Est. Sessions**: Session counts are *estimated* from playtime (roughly 30-minute average sessions). Steam does not expose actual session-level data through its public API. The count in the "Est. Sessions" column is an approximation.
- **Snapshot history**: Some trend features (e.g., playtime changes over time) require multiple snapshots. Stats improve after the tool has been run on several separate days.
- **Date range precision**: Steam's public API does not provide per-day playtime breakdowns. Weekly/monthly ranges use the available data (last 2 weeks from `GetRecentlyPlayedGames`) combined with lifetime totals.

## How it works

`steam-stats` fetches data from the Steam Web API on launch, caches a snapshot locally, and renders the TUI. Subsequent launches show the cached data immediately while a background refresh runs.

Cache location: `~/.local/share/steam-stats/` (macOS/Linux) or `%LOCALAPPDATA%\steam-stats\` (Windows).

## Privacy

- Your Steam profile must be **public** to return game data
- The tool only reads data — it never modifies your Steam account
- Your API key and Steam ID are stored locally in `config.toml`
- No data is sent anywhere except Steam's official API

## Building from source

```sh
git clone https://github.com/ericwiley/SteamStats
cd SteamStats
cargo build --release
./target/release/steam-stats
```

Requires Rust 1.75+ and an internet connection for dependencies.

## License

See [LICENSE](LICENSE) for details.
