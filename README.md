# SteamStats

An iOS app for viewing personal Steam statistics with customizable date ranges, inspired by Steam Replay and Stats.fm.

## Features

- View gaming stats across multiple time ranges (today, this week, 6 months, lifetime)
- See top games with playtime percentages
- Track total hours, games played, and achievements
- Beautiful card-based UI inspired by Steam Replay
- Real-time data from Steam Web API

## Requirements

- iOS 17.0+
- Xcode 15.0+
- Steam API Key (get one at https://steamcommunity.com/dev/apikey)
- Your Steam ID (find it in your Steam profile URL)

## Setup

### 1. Get Your Steam API Key

1. Visit https://steamcommunity.com/dev/apikey
2. Register for a Steam Web API Key
3. Copy your API key

### 2. Find Your Steam ID

Your Steam ID is the long number in your profile URL:
- Profile URL: `https://steamcommunity.com/profiles/76561198000000000`
- Steam ID: `76561198000000000`

Alternatively, use a service like https://steamid.io/ to convert your custom URL.

### 3. Open the Project in Xcode

You have two options:

**Option A: Use Xcode Directly**
1. Open Xcode
2. File → New → Project
3. Choose "iOS" → "App"
4. Name it "SteamStats" with organization identifier "com.steamstats"
5. Choose SwiftUI interface and Swift language
6. Replace the generated files with the files from this repository

**Option B: Use XcodeGen (if installed)**
```bash
# Install XcodeGen if not already installed
brew install xcodegen

# Generate the Xcode project
xcodegen generate

# Open the project
open SteamStats.xcodeproj
```

### 4. Configure Your API Key

The API key is already configured in `Info.plist`, but you can change it:

1. Open `SteamStats/Info.plist`
2. Find the `STEAM_API_KEY` entry
3. Replace with your API key

### 5. Enter Your Steam ID

When you first launch the app, you'll be prompted to enter your Steam ID.

## Building and Running

### Using Xcode
1. Select a simulator or device
2. Press Cmd+R to build and run

### Using Command Line
```bash
# Build
xcodebuild -scheme SteamStats -destination 'platform=iOS Simulator,name=iPhone 15'

# Run tests
xcodebuild test -scheme SteamStats -destination 'platform=iOS Simulator,name=iPhone 15'
```

## Project Structure

```
SteamStats/
├── App/
│   └── SteamStatsApp.swift           # App entry point
├── Features/
│   └── Stats/
│       ├── Views/
│       │   └── StatsView.swift       # Main stats screen
│       ├── ViewModels/
│       │   └── StatsViewModel.swift  # Stats business logic
│       └── Models/
│           └── StatsModels.swift     # Stats data models
├── Core/
│   ├── Network/
│   │   └── SteamAPIClient.swift      # API client
│   └── Models/
│       └── SteamModels.swift         # Steam API models
└── Resources/
    └── Assets.xcassets               # App assets
```

## Steam API Endpoints Used

- `IPlayerService/GetOwnedGames` - Fetch all owned games with playtime
- `IPlayerService/GetRecentlyPlayedGames` - Recent activity (last 2 weeks)
- `ISteamUser/GetPlayerSummaries` - Player profile information
- `ISteamUserStats/GetPlayerAchievements` - Achievement data per game

## Privacy Notes

- Your Steam profile must be public for the API to return data
- The app only reads data; it never modifies your Steam account
- Your API key and Steam ID are stored locally on your device
- No data is sent to any server other than Steam's official API

## Known Limitations

- Steam API doesn't provide exact daily/weekly breakdowns, so some date ranges use approximations
- The "today" and "this week" ranges show games played in the last 2 weeks (Steam API limitation)
- Session counts are estimated based on playtime

## Sources

- [Steam Web API Documentation](https://developer.valvesoftware.com/wiki/Steam_Web_API)
- [Better Steam Web API Documentation](https://steamwebapi.azurewebsites.net/)
- [IPlayerService Interface](https://partner.steamgames.com/doc/webapi/iplayerservice)

## License

See LICENSE file for details.
