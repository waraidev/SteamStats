//
//  SteamModels.swift
//  SteamStats
//
//  Created on 2026-01-27.
//

import Foundation

// MARK: - Owned Games Response

struct OwnedGamesResponse: Codable {
    let response: OwnedGamesData
}

struct OwnedGamesData: Codable {
    let gameCount: Int
    let games: [Game]
}

struct Game: Codable, Identifiable {
    let appid: Int
    let name: String?
    let playtimeForever: Int // in minutes
    let imgIconUrl: String?
    let imgLogoUrl: String?
    let playtime2Weeks: Int? // in minutes
    let playtimeWindowsForever: Int?
    let playtimeMacForever: Int?
    let playtimeLinuxForever: Int?

    var id: Int { appid }

    var hoursPlayed: Double {
        Double(playtimeForever) / 60.0
    }

    var iconURL: URL? {
        guard let imgIconUrl = imgIconUrl else { return nil }
        return URL(string: "https://media.steampowered.com/steamcommunity/public/images/apps/\(appid)/\(imgIconUrl).jpg")
    }

    var logoURL: URL? {
        guard let imgLogoUrl = imgLogoUrl else { return nil }
        return URL(string: "https://media.steampowered.com/steamcommunity/public/images/apps/\(appid)/\(imgLogoUrl).jpg")
    }
}

// MARK: - Recent Games Response

struct RecentGamesResponse: Codable {
    let response: RecentGamesData
}

struct RecentGamesData: Codable {
    let totalCount: Int?
    let games: [Game]?
}

// MARK: - Player Summaries Response

struct PlayerSummariesResponse: Codable {
    let response: PlayerSummariesData
}

struct PlayerSummariesData: Codable {
    let players: [Player]
}

struct Player: Codable, Identifiable {
    let steamid: String
    let personaname: String
    let profileurl: String
    let avatar: String
    let avatarmedium: String
    let avatarfull: String
    let personastate: Int
    let communityvisibilitystate: Int
    let timecreated: Int?

    var id: String { steamid }
}

// MARK: - Player Achievements Response

struct PlayerAchievementsResponse: Codable {
    let playerstats: PlayerStats
}

struct PlayerStats: Codable {
    let steamID: String
    let gameName: String
    let achievements: [Achievement]?
    let success: Bool
}

struct Achievement: Codable, Identifiable {
    let apiname: String
    let achieved: Int
    let unlocktime: Int

    var id: String { apiname }
    var isUnlocked: Bool { achieved == 1 }
}
