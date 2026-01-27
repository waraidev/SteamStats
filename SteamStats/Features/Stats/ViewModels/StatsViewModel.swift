//
//  StatsViewModel.swift
//  SteamStats
//
//  Created on 2026-01-27.
//

import Foundation
import SwiftUI

@MainActor
@Observable
final class StatsViewModel {

    // MARK: - Published Properties

    var selectedDateRange: DateRange = .sixMonths
    var isLoading = false
    var errorMessage: String?
    var steamID: String = "" {
        didSet {
            // Save Steam ID when it changes
            UserDefaults.standard.savedSteamID = steamID
        }
    }

    private(set) var allGames: [Game] = []
    private(set) var player: Player?

    // MARK: - Computed Properties

    var filteredGames: [Game] {
        selectedDateRange.filterGames(allGames)
    }

    var overallStats: OverallStats {
        let games = filteredGames
        let totalMinutes = games.reduce(0) { $0 + $1.playtimeForever }
        let totalHours = Double(totalMinutes) / 60.0

        return OverallStats(
            totalGames: games.count,
            totalHours: totalHours,
            totalAchievements: 0, // TODO: Fetch actual achievements
            recentGames: games.filter { $0.playtime2Weeks ?? 0 > 0 }.count
        )
    }

    var topGames: [GameStats] {
        let games = filteredGames
        let totalMinutes = games.reduce(0) { $0 + $1.playtimeForever }

        guard totalMinutes > 0 else { return [] }

        return games
            .sorted { $0.playtimeForever > $1.playtimeForever }
            .prefix(10)
            .map { game in
                let percentage = (Double(game.playtimeForever) / Double(totalMinutes)) * 100
                let sessions = estimateSessions(for: game)
                return GameStats(game: game, percentageOfTotalTime: percentage, sessions: sessions)
            }
    }

    // MARK: - Dependencies

    private let apiClient: SteamAPIClient

    init(apiClient: SteamAPIClient = .shared) {
        self.apiClient = apiClient
        // Load saved Steam ID
        if let savedID = UserDefaults.standard.savedSteamID {
            self.steamID = savedID
        }
    }

    // MARK: - Actions

    func loadStats() async {
        guard !steamID.isEmpty else {
            errorMessage = "Please enter your Steam ID"
            return
        }

        isLoading = true
        errorMessage = nil

        do {
            // Fetch owned games
            let ownedGamesResponse = try await apiClient.getOwnedGames(steamID: steamID)
            allGames = ownedGamesResponse.response.games

            // Fetch player info
            let playerResponse = try await apiClient.getPlayerSummaries(steamIDs: [steamID])
            player = playerResponse.response.players.first

            isLoading = false
        } catch {
            errorMessage = "Failed to load stats: \(error.localizedDescription)"
            isLoading = false
        }
    }

    func refreshStats() async {
        await loadStats()
    }

    // MARK: - Private Methods

    private func estimateSessions(for game: Game) -> Int {
        // Rough estimate: 1 session per 2 hours of playtime
        let hours = game.hoursPlayed
        return max(1, Int(hours / 2))
    }
}
