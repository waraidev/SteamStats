# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SteamStats is an iPhone app for viewing personal Steam statistics. Built with Swift for iOS.

## Build and Development Commands

```bash
# Build the project
xcodebuild -scheme SteamStats -destination 'platform=iOS Simulator,name=iPhone 15'

# Run all tests
xcodebuild test -scheme SteamStats -destination 'platform=iOS Simulator,name=iPhone 15'

# Run a single test class
xcodebuild test -scheme SteamStats -destination 'platform=iOS Simulator,name=iPhone 15' -only-testing:SteamStatsTests/TestClassName

# Run a single test method
xcodebuild test -scheme SteamStats -destination 'platform=iOS Simulator,name=iPhone 15' -only-testing:SteamStatsTests/TestClassName/testMethodName

# Lint with SwiftLint (if installed)
swiftlint

# Format with SwiftFormat (if installed)
swiftformat .
```

## Swift Best Practices

### Code Organization

- Use extensions to organize code by protocol conformance
- Group related functionality in separate files within feature folders
- Prefer composition over inheritance
- Use `// MARK: -` comments to organize large files by section

### SwiftUI Conventions

- Extract reusable views into separate structs
- Keep views small and focused on presentation
- Move business logic to ViewModels or dedicated services
- Use `@StateObject` for owned observable objects, `@ObservedObject` for injected ones
- Prefer `@Environment` for dependency injection over initializer parameters for shared services

### Concurrency

- Use Swift's structured concurrency (`async/await`) over completion handlers
- Mark `@MainActor` for UI-related code
- Use `Task` for bridging sync to async contexts
- Handle cancellation appropriately in long-running operations

### Error Handling

- Define custom error types conforming to `Error` for domain-specific failures
- Use `Result` type when callbacks are necessary
- Prefer throwing functions over optional returns when failure is meaningful

### Naming Conventions

- Types and protocols: UpperCamelCase
- Properties, methods, variables: lowerCamelCase
- Use descriptive names; avoid abbreviations except for common ones (URL, ID)
- Boolean properties: use `is`, `has`, `should` prefixes (e.g., `isLoading`, `hasError`)

### API Integration

- Use `Codable` for JSON parsing
- Create dedicated service classes for API calls
- Define models in separate files from networking code
- Use `URLSession` with async/await for network requests

### Testing

- Name tests descriptively: `test_methodName_condition_expectedResult`
- Use `XCTest` for unit tests
- Mock network calls and external dependencies
- Test ViewModels independently from views
