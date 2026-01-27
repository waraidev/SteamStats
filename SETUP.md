# SteamStats - Quick Setup Guide

## Next Steps to Get Your App Running

### 1. Create the Xcode Project

Since we can't create `.xcodeproj` files directly, you need to create the Xcode project:

**Option A: Using Xcode GUI (Recommended)**

1. Open Xcode
2. File → New → Project
3. Select **iOS** → **App**
4. Configure:
   - Product Name: `SteamStats`
   - Team: Your team
   - Organization Identifier: `com.steamstats`
   - Interface: **SwiftUI**
   - Language: **Swift**
   - Storage: None
   - Include Tests: Yes
5. Save in the root directory `/Users/eric.wiley/Documents/GitHub/SteamStats`
6. Delete the auto-generated `ContentView.swift` and `SteamStatsApp.swift` files
7. In Xcode, right-click the project navigator and select "Add Files to SteamStats"
8. Add the existing folder structure:
   - `SteamStats/App`
   - `SteamStats/Core`
   - `SteamStats/Features`
   - `SteamStats/Resources`
   - `SteamStatsTests`

**Option B: Using XcodeGen**

If you have XcodeGen installed:

```bash
xcodegen generate
open SteamStats.xcodeproj
```

### 2. Configure Info.plist

The Info.plist is already configured with your API key. If you need to change it:

1. Open `SteamStats/Info.plist`
2. Find `STEAM_API_KEY`
3. Replace with your key from https://steamcommunity.com/dev/apikey

### 3. Get Your Steam ID

You'll need your Steam ID to use the app:

1. Visit your Steam profile
2. Copy the number from the URL: `https://steamcommunity.com/profiles/[YOUR_ID]`
3. Alternatively, use https://steamid.io/ to convert a custom URL

### 4. Build and Run

**In Xcode:**
- Select iPhone 15 simulator (or any iOS 17+ simulator)
- Press Cmd+R

**From Terminal:**
```bash
xcodebuild -scheme SteamStats \
  -destination 'platform=iOS Simulator,name=iPhone 15' \
  clean build
```

### 5. First Launch

When you launch the app:
1. Enter your Steam ID in the setup screen
2. Tap "Continue"
3. The app will fetch your game stats

### Troubleshooting

**Build errors about missing files:**
- Make sure all Swift files are added to the SteamStats target
- Check Project Navigator → Select each file → File Inspector → Target Membership

**API returns no data:**
- Ensure your Steam profile is set to **Public**
- Visit Steam → Profile → Edit Profile → Privacy Settings
- Set "Game details" to Public

**"Invalid API Key" error:**
- Verify your API key in Info.plist
- Get a new key at https://steamcommunity.com/dev/apikey

## Project Architecture

```
SteamStats/
├── App/
│   └── SteamStatsApp.swift          # Entry point
│
├── Features/
│   └── Stats/                        # Stats feature
│       ├── Views/                    # SwiftUI views
│       ├── ViewModels/               # Business logic
│       └── Models/                   # Feature models
│
├── Core/
│   ├── Network/                      # API client
│   └── Models/                       # Shared models
│
└── Resources/
    └── Assets.xcassets               # Images, colors
```

## Key Files

- **SteamStatsApp.swift** - App entry point, displays StatsView
- **StatsView.swift** - Main UI with cards and date range picker
- **StatsViewModel.swift** - Handles data fetching and filtering
- **SteamAPIClient.swift** - Makes API calls to Steam
- **SteamModels.swift** - Codable structs for API responses

## What's Implemented

✅ Steam API integration (GetOwnedGames, GetRecentlyPlayedGames, GetPlayerSummaries)
✅ Date range filtering (today, week, 6 months, lifetime)
✅ Top games with playtime percentages
✅ Overall stats cards (games played, hours, etc.)
✅ Steam Replay-inspired UI with gradient background
✅ Error handling and loading states
✅ Unit tests structure

## What Could Be Added Later

- Pull to refresh
- Game detail view with achievements
- Charts/graphs for playtime over time
- Export stats as image
- Multiple Steam account support
- Dark/light theme toggle
- Custom date range picker

## API Rate Limits

Steam Web API allows **100,000 requests per 24 hours**. This app makes:
- 1 request on app launch (GetOwnedGames)
- 1 request for player info (GetPlayerSummaries)

So you can refresh about 50,000 times per day (you're safe!).

## Support

For Steam API issues: https://steamcommunity.com/dev
For app issues: Check the README.md
