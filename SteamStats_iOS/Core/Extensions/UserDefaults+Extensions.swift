//
//  UserDefaults+Extensions.swift
//  SteamStats
//
//  Created on 2026-01-27.
//

import Foundation

extension UserDefaults {
    private enum Keys {
        static let steamID = "steamID"
    }

    var savedSteamID: String? {
        get { string(forKey: Keys.steamID) }
        set { set(newValue, forKey: Keys.steamID) }
    }
}
