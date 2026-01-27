//
//  StatsModels.swift
//  SteamStats
//
//  Created on 2026-01-27.
//

import Foundation

enum DateRange: String, CaseIterable {
    case today = "today"
    case thisWeek = "this week"
    case fourWeeks = "4 weeks"
    case sixMonths = "6 months"
    case year2026 = "2026"
    case lifetime = "lifetime"

    var displayName: String { rawValue }

    func filterGames(_ games: [Game]) -> [Game] {
        let now = Date()
        let calendar = Calendar.current

        switch self {
        case .today:
            // Note: Steam API doesn't provide today's data, use 2 weeks as proxy
            return games.filter { $0.playtime2Weeks ?? 0 > 0 }
        case .thisWeek:
            return games.filter { $0.playtime2Weeks ?? 0 > 0 }
        case .fourWeeks:
            return games.filter { $0.playtime2Weeks ?? 0 > 0 }
        case .sixMonths:
            // Use all games with recent playtime as proxy
            return games.filter { $0.playtimeForever > 0 }
        case .year2026:
            // Since we can't get exact 2026 data, show all recent games
            return games.filter { $0.playtimeForever > 0 }
        case .lifetime:
            return games
        }
    }
}

struct GameStats {
    let game: Game
    let percentageOfTotalTime: Double
    let sessions: Int // Approximation

    var hoursPlayed: Double {
        game.hoursPlayed
    }
}

struct OverallStats {
    let totalGames: Int
    let totalHours: Double
    let totalAchievements: Int
    let recentGames: Int

    var minutesPlayed: Int {
        Int(totalHours * 60)
    }
}
