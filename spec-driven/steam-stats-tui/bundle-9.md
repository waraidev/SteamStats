### Bundle 9: iOS Cleanup
> Stage: integration | Parallel: no (depends on Bundle 8 — README.md must exist before SETUP.md deleted) | Files: deletes SteamStats_iOS/, SteamStats.xcodeproj/, project.yml, PROJECT_SUMMARY.md, SETUP.md; modifies CLAUDE.md

**Bundle Verify**: Repository contains no iOS artifacts; only Rust source, spec-driven/, and GitHub workflow remain.
- **Level**: inspection
- **Given**: Cleanup completed
- **Action**: `git status` + `find . -name "*.swift" ! -path "./.git/*"`
- **Outcome**: No Swift files, no Xcode project, no iOS directories in working tree; `git status` shows only expected Rust project files

> **Context**
>
> **Architecture Decisions**
> - **AD-8: iOS cleanup deferred to post-TUI task** — Decision: Delete all iOS/Swift artifacts after TUI is functional. Rationale: Swift code in SteamStats_iOS/ is useful as a reference for Steam API response shapes during TUI implementation (particularly models/steam.rs, STEP-2). Deleting it first removes the API shape reference.
>
> **Constraints**
> - iOS cleanup must run AFTER README.md is created (Bundle 8 / STEP-27) — deleting SETUP.md before a replacement README exists leaves the repo with no documentation (Category: other | Source: design AD-8)

---

#### STEP-28: iOS Cleanup
MANUAL -> AD-8: Delete all iOS/Swift artifacts after TUI implementation complete | delete `SteamStats_iOS/`, `SteamStats.xcodeproj/`, `project.yml`, `PROJECT_SUMMARY.md`, `SETUP.md`; modify `CLAUDE.md` | Effort: XS

> **Intent**: The iOS files (SteamStats_iOS/, SteamStats.xcodeproj/) are useful as a reference for Steam API field names during STEP-2 and STEP-9 (SteamModels.swift has canonical field names). Deleting them before those steps removes the reference corpus. This STEP is ordered last intentionally (AD-8). Verify that `README.md` from STEP-27 exists before deleting `SETUP.md`.

- `git rm -r SteamStats_iOS/ SteamStats.xcodeproj/`
- `git rm project.yml PROJECT_SUMMARY.md SETUP.md`
- Update `CLAUDE.md` to remove Swift/iOS build commands and replace with Rust equivalents (`cargo build`, `cargo test`, `cargo check`)
- Verify `README.md` exists (from STEP-27) before deleting SETUP.md

**Verify**:
- Level: inspection | Given: Cleanup complete | Action: `git status` and `find . -name "*.swift" ! -path "./.git/*"` | Outcome: No Swift files in working tree; no Xcode project; `README.md` exists; `CLAUDE.md` reflects Rust project

> Depends on: STEP-27 | Enables: — | Parallel with: —
