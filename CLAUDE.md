# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SteamStats is a terminal UI (TUI) app for viewing personal Steam statistics. Built with Rust.

## Build and Development Commands

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run the app
cargo run

# Run with arguments
cargo run -- --steam-id <STEAM_ID>

# Run all tests
cargo test

# Run a single test module
cargo test <module_name>

# Run a single test
cargo test <test_name>

# Check for compile errors without building
cargo check

# Lint with Clippy
cargo clippy

# Format code
cargo fmt

# Format check (CI)
cargo fmt -- --check
```

## Rust Best Practices

### Code Organization

- Organize code into modules by feature/domain (e.g., `steam/`, `ui/`, `config/`)
- Use `mod.rs` or module files at the same level for public interfaces
- Prefer composition over inheritance (use traits and structs)
- Keep modules focused on a single responsibility

### TUI Conventions (ratatui)

- Separate app state (`App` struct) from rendering logic
- Use an event loop with `crossterm` for input handling
- Keep `draw()` functions pure — pass state in, render out
- Use `StatefulWidget` for interactive components (lists, tables)
- Handle terminal cleanup on panic via `color_eyre` or a panic hook

### Concurrency (tokio)

- Use `tokio` for async runtime; prefer `async/await` over raw futures
- Use `tokio::spawn` for background tasks; keep the main thread for rendering
- Use `tokio::sync::mpsc` channels to communicate between async tasks and the TUI event loop
- Avoid blocking the async runtime with sync I/O — use `tokio::task::spawn_blocking` when needed

### Error Handling

- Use `thiserror` for defining domain-specific error types
- Use `anyhow` for application-level error propagation
- Prefer `Result<T, E>` over panicking in library/service code
- Surface errors to the TUI as user-readable messages, not raw debug output

### API Integration (reqwest + serde)

- Use `reqwest` with `async/await` for HTTP requests to the Steam Web API
- Use `serde` with `#[derive(Deserialize)]` for JSON parsing
- Define response models in a dedicated `models/` or `steam/` module
- Handle rate limiting and network errors gracefully with `thiserror`/`anyhow`

### Naming Conventions

- Types, traits, enums: `UpperCamelCase`
- Functions, methods, variables, modules: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Use descriptive names; avoid abbreviations except for common ones (URL, ID, TUI)
- Boolean fields/variables: use `is_`, `has_`, `should_` prefixes (e.g., `is_loading`, `has_error`)

### Testing

- Name tests descriptively: `test_function_name_condition_expected_result`
- Use `#[cfg(test)]` modules in the same file for unit tests
- Mock HTTP calls with `mockito` or `wiremock` for integration tests
- Test business logic (state transitions, data transforms) independently from TUI rendering
