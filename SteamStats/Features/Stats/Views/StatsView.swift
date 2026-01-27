//
//  StatsView.swift
//  SteamStats
//
//  Created on 2026-01-27.
//

import SwiftUI

struct StatsView: View {
    @State private var viewModel = StatsViewModel()

    var body: some View {
        NavigationStack {
            ZStack {
                // Background gradient similar to Steam Replay
                LinearGradient(
                    colors: [Color(red: 0.1, green: 0.1, blue: 0.2), Color(red: 0.2, green: 0.1, blue: 0.2)],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
                .ignoresSafeArea()

                if viewModel.isLoading {
                    ProgressView("Loading stats...")
                        .foregroundStyle(.white)
                } else if let errorMessage = viewModel.errorMessage {
                    ErrorView(message: errorMessage) {
                        Task {
                            await viewModel.loadStats()
                        }
                    }
                } else if viewModel.allGames.isEmpty {
                    SetupView(steamID: $viewModel.steamID) {
                        Task {
                            await viewModel.loadStats()
                        }
                    }
                } else {
                    ScrollView {
                        VStack(spacing: 20) {
                            // Date Range Picker
                            DateRangePicker(selectedRange: $viewModel.selectedDateRange)
                                .padding(.horizontal)

                            // Overall Stats Cards
                            OverallStatsView(stats: viewModel.overallStats)
                                .padding(.horizontal)

                            // Top Games
                            TopGamesView(games: viewModel.topGames)
                                .padding(.horizontal)
                        }
                        .padding(.vertical)
                    }
                }
            }
            .navigationTitle("Steam Stats")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button {
                        Task {
                            await viewModel.refreshStats()
                        }
                    } label: {
                        Image(systemName: "arrow.clockwise")
                            .foregroundStyle(.white)
                    }
                }
            }
        }
        .task {
            // Auto-load if Steam ID is available
            if !viewModel.steamID.isEmpty {
                await viewModel.loadStats()
            }
        }
    }
}

// MARK: - Setup View

struct SetupView: View {
    @Binding var steamID: String
    let onContinue: () -> Void

    var body: some View {
        VStack(spacing: 20) {
            Text("Enter Your Steam ID")
                .font(.title)
                .fontWeight(.bold)
                .foregroundStyle(.white)

            Text("You can find your Steam ID in your profile URL")
                .font(.subheadline)
                .foregroundStyle(.white.opacity(0.7))
                .multilineTextAlignment(.center)

            TextField("Steam ID", text: $steamID)
                .textFieldStyle(.roundedBorder)
                .padding(.horizontal, 40)

            Button("Continue") {
                onContinue()
            }
            .buttonStyle(.borderedProminent)
            .disabled(steamID.isEmpty)
        }
        .padding()
    }
}

// MARK: - Error View

struct ErrorView: View {
    let message: String
    let onRetry: () -> Void

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "exclamationmark.triangle")
                .font(.system(size: 50))
                .foregroundStyle(.red)

            Text(message)
                .foregroundStyle(.white)
                .multilineTextAlignment(.center)
                .padding(.horizontal)

            Button("Retry") {
                onRetry()
            }
            .buttonStyle(.borderedProminent)
        }
    }
}

// MARK: - Date Range Picker

struct DateRangePicker: View {
    @Binding var selectedRange: DateRange

    var body: some View {
        Menu {
            ForEach(DateRange.allCases, id: \.self) { range in
                Button {
                    selectedRange = range
                } label: {
                    HStack {
                        Text(range.displayName)
                        if selectedRange == range {
                            Image(systemName: "checkmark")
                        }
                    }
                }
            }
        } label: {
            HStack {
                Text(selectedRange.displayName)
                    .foregroundStyle(.white)
                Image(systemName: "chevron.down")
                    .foregroundStyle(.white.opacity(0.7))
            }
            .padding()
            .background(Color.white.opacity(0.1))
            .cornerRadius(10)
        }
    }
}

// MARK: - Overall Stats View

struct OverallStatsView: View {
    let stats: OverallStats

    var body: some View {
        VStack(spacing: 16) {
            HStack(spacing: 16) {
                StatCard(
                    value: "\(stats.totalGames)",
                    label: "games played"
                )

                StatCard(
                    value: "\(Int(stats.totalHours))",
                    label: "hours played"
                )
            }

            HStack(spacing: 16) {
                StatCard(
                    value: "\(stats.minutesPlayed)",
                    label: "minutes played"
                )

                StatCard(
                    value: "\(stats.recentGames)",
                    label: "recent games"
                )
            }
        }
    }
}

struct StatCard: View {
    let value: String
    let label: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(value)
                .font(.system(size: 36, weight: .bold))
                .foregroundStyle(.white)

            Text(label)
                .font(.subheadline)
                .foregroundStyle(.white.opacity(0.7))
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding()
        .background(Color.white.opacity(0.1))
        .cornerRadius(12)
    }
}

// MARK: - Top Games View

struct TopGamesView: View {
    let games: [GameStats]

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Top Games")
                .font(.title2)
                .fontWeight(.bold)
                .foregroundStyle(.white)
                .padding(.bottom, 8)

            ForEach(Array(games.enumerated()), id: \.element.game.id) { index, gameStats in
                GameStatsCard(
                    rank: index + 1,
                    gameStats: gameStats
                )
            }
        }
    }
}

struct GameStatsCard: View {
    let rank: Int
    let gameStats: GameStats

    var body: some View {
        HStack(spacing: 12) {
            // Rank
            Text("\(rank)")
                .font(.title3)
                .fontWeight(.bold)
                .foregroundStyle(.white.opacity(0.5))
                .frame(width: 30)

            // Game Info
            VStack(alignment: .leading, spacing: 4) {
                Text(gameStats.game.name ?? "Unknown Game")
                    .font(.headline)
                    .foregroundStyle(.white)

                HStack(spacing: 16) {
                    Text("\(Int(gameStats.percentageOfTotalTime))% of play time")
                        .font(.caption)
                        .foregroundStyle(.white.opacity(0.7))

                    Text("\(gameStats.sessions) sessions")
                        .font(.caption)
                        .foregroundStyle(.white.opacity(0.7))
                }
            }

            Spacer()

            // Hours
            Text("\(Int(gameStats.hoursPlayed))h")
                .font(.title3)
                .fontWeight(.semibold)
                .foregroundStyle(.white)
        }
        .padding()
        .background(Color.white.opacity(0.1))
        .cornerRadius(12)
    }
}

#Preview {
    StatsView()
}
