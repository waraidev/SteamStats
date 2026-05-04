//
//  SteamAPIClient.swift
//  SteamStats
//
//  Created on 2026-01-27.
//

import Foundation

enum SteamAPIError: Error {
    case invalidURL
    case invalidResponse
    case httpError(Int)
    case decodingError(Error)
    case missingAPIKey
}

@MainActor
final class SteamAPIClient {
    static let shared = SteamAPIClient()

    private let apiKey: String
    private let baseURL = "https://api.steampowered.com"

    private init() {
        // Load API key from bundle or environment
        if let key = Bundle.main.object(forInfoDictionaryKey: "STEAM_API_KEY") as? String {
            self.apiKey = key
        } else if let key = ProcessInfo.processInfo.environment["STEAM_API_KEY"] {
            self.apiKey = key
        } else {
            self.apiKey = "FA0E3976795C943FFC225E6492CF9A18" // Fallback to hardcoded
        }
    }

    // MARK: - API Methods

    func getOwnedGames(steamID: String, includeAppInfo: Bool = true, includePlayedFree: Bool = true) async throws -> OwnedGamesResponse {
        let endpoint = "/IPlayerService/GetOwnedGames/v0001/"
        var components = URLComponents(string: baseURL + endpoint)
        components?.queryItems = [
            URLQueryItem(name: "key", value: apiKey),
            URLQueryItem(name: "steamid", value: steamID),
            URLQueryItem(name: "include_appinfo", value: includeAppInfo ? "1" : "0"),
            URLQueryItem(name: "include_played_free_games", value: includePlayedFree ? "1" : "0"),
            URLQueryItem(name: "format", value: "json")
        ]

        guard let url = components?.url else {
            throw SteamAPIError.invalidURL
        }

        return try await performRequest(url: url)
    }

    func getRecentlyPlayedGames(steamID: String, count: Int = 20) async throws -> RecentGamesResponse {
        let endpoint = "/IPlayerService/GetRecentlyPlayedGames/v0001/"
        var components = URLComponents(string: baseURL + endpoint)
        components?.queryItems = [
            URLQueryItem(name: "key", value: apiKey),
            URLQueryItem(name: "steamid", value: steamID),
            URLQueryItem(name: "count", value: "\(count)"),
            URLQueryItem(name: "format", value: "json")
        ]

        guard let url = components?.url else {
            throw SteamAPIError.invalidURL
        }

        return try await performRequest(url: url)
    }

    func getPlayerSummaries(steamIDs: [String]) async throws -> PlayerSummariesResponse {
        let endpoint = "/ISteamUser/GetPlayerSummaries/v0002/"
        var components = URLComponents(string: baseURL + endpoint)
        components?.queryItems = [
            URLQueryItem(name: "key", value: apiKey),
            URLQueryItem(name: "steamids", value: steamIDs.joined(separator: ",")),
            URLQueryItem(name: "format", value: "json")
        ]

        guard let url = components?.url else {
            throw SteamAPIError.invalidURL
        }

        return try await performRequest(url: url)
    }

    func getPlayerAchievements(steamID: String, appID: Int) async throws -> PlayerAchievementsResponse {
        let endpoint = "/ISteamUserStats/GetPlayerAchievements/v0001/"
        var components = URLComponents(string: baseURL + endpoint)
        components?.queryItems = [
            URLQueryItem(name: "key", value: apiKey),
            URLQueryItem(name: "steamid", value: steamID),
            URLQueryItem(name: "appid", value: "\(appID)"),
            URLQueryItem(name: "format", value: "json")
        ]

        guard let url = components?.url else {
            throw SteamAPIError.invalidURL
        }

        return try await performRequest(url: url)
    }

    // MARK: - Private Methods

    private func performRequest<T: Decodable>(url: URL) async throws -> T {
        let (data, response) = try await URLSession.shared.data(from: url)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw SteamAPIError.invalidResponse
        }

        guard (200...299).contains(httpResponse.statusCode) else {
            throw SteamAPIError.httpError(httpResponse.statusCode)
        }

        do {
            let decoder = JSONDecoder()
            decoder.keyDecodingStrategy = .convertFromSnakeCase
            return try decoder.decode(T.self, from: data)
        } catch {
            throw SteamAPIError.decodingError(error)
        }
    }
}
