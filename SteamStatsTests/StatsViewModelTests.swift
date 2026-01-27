//
//  StatsViewModelTests.swift
//  SteamStatsTests
//
//  Created on 2026-01-27.
//

import XCTest
@testable import SteamStats

@MainActor
final class StatsViewModelTests: XCTestCase {

    func test_filteredGames_lifetime_returnsAllGames() {
        // Given
        let viewModel = StatsViewModel()
        viewModel.selectedDateRange = .lifetime

        // When
        let filtered = viewModel.filteredGames

        // Then
        XCTAssertEqual(filtered.count, viewModel.allGames.count)
    }

    func test_overallStats_noGames_returnsZero() {
        // Given
        let viewModel = StatsViewModel()

        // When
        let stats = viewModel.overallStats

        // Then
        XCTAssertEqual(stats.totalGames, 0)
        XCTAssertEqual(stats.totalHours, 0)
    }

    func test_topGames_sortsbyPlaytime() {
        // Given
        let viewModel = StatsViewModel()

        // When
        let topGames = viewModel.topGames

        // Then
        // Verify sorted in descending order
        for i in 0..<(topGames.count - 1) {
            XCTAssertGreaterThanOrEqual(
                topGames[i].hoursPlayed,
                topGames[i + 1].hoursPlayed
            )
        }
    }
}
