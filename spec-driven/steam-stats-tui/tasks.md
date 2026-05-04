---
title: "Tasks: Steam Stats TUI"
slug: steam-stats-tui
status: final
design_source: spec-driven/steam-stats-tui/design.md
design_hash: sha256:56635585fc8dcd90132be7eef880a2aa82aa7423ce5b16518567b5e8c23e6887
spec_source: spec-driven/steam-stats-tui/spec.md
spec_hash: sha256:f372ea7d9e528f824db6395a2250339dc715571a5f77374aa3d070a207cd49a5
strategy: max-parallelism
total_steps: 28
total_slices: 7
total_bundles: 9
validation: subagent
version: 2.0
date: 2026-05-04
---

# Tasks: Steam Stats TUI

> Design: spec-driven/steam-stats-tui/design.md | Spec: spec-driven/steam-stats-tui/spec.md | Strategy: max-parallelism | Generated: 2026-05-04 | Status: Final

> Do not edit this document after finalization. Track execution in `spec-driven/steam-stats-tui/progress-bundle-N.md` files.

---

## Traceability

### Functional Requirements

| FR | Priority | ACs Covered | STEPs | Slice | Bundle |
|----|----------|-------------|-------|-------|--------|
| FR-1 | Must Have | AC-1.1, AC-1.2, AC-1.3, AC-1.4 | STEP-4, STEP-15, STEP-16, STEP-19, STEP-21 | Slice 1, 2D-1, 2D-2 | 1, 5, 6 |
| FR-2 | Must Have | AC-2.1, AC-2.2, AC-2.3, AC-2.4 | STEP-4, STEP-14, STEP-19, STEP-21 | Slice 1, 2D-1, 2D-2 | 1, 5, 6 |
| FR-3 | Must Have | AC-3.1, AC-3.2, AC-3.3, AC-3.4 | STEP-9, STEP-23 | Slice 2B, 3A | 3, 7 |
| FR-4 | Must Have | AC-4.1, AC-4.2, AC-4.3, AC-4.4, AC-4.5 | STEP-11 | Slice 2C | 4 |
| FR-5 | Must Have | AC-5.1, AC-5.2, AC-5.3, AC-5.4 | STEP-7, STEP-23 | Slice 2A, 3A | 2, 7 |
| FR-6 | Should Have | AC-6.1, AC-6.2 | STEP-19 | Slice 2D-2 | 6 |
| FR-7 | Should Have | AC-7.1, AC-7.2 | STEP-11 | Slice 2C | 4 |
| FR-8 | Should Have | AC-8.1, AC-8.2, AC-8.3 | STEP-26, STEP-27 | Slice 3B | 8 |
| FR-9 | Nice to Have | AC-9.1, AC-9.2 | STEP-19, STEP-24 | Slice 2D-2, 3A | 6, 7 |

> STEP-1, STEP-3, STEP-5, STEP-6, STEP-8, STEP-10, STEP-12, STEP-13, STEP-17, STEP-18, STEP-20, STEP-22, STEP-25, STEP-28 use MANUAL trace — see bundle files for rationale.

### Non-Functional Requirements

| NFR | Disposition | STEP / Mechanism | Verification |
|-----|-------------|-----------------|--------------|
| NFR-1 (startup <500ms) | Implemented | STEP-23 | `SnapshotStore::load()` synchronous before `run_tui()`; API fetch in background tokio task; first render shows cached data immediately. Verify: app starts and shows Loading state within <500ms before DataLoaded arrives. |
| NFR-2 (API key security) | Implemented | STEP-7, STEP-8, STEP-26 | Custom Debug impl redacts `steam_api_key` to `[REDACTED]`. STEP-8 test asserts Debug output does not contain actual key. STEP-26 CI has no hardcoded secrets. |
| NFR-3 (cross-platform) | Implemented | STEP-26, S-4 throughout | CI matrix: x86_64-apple-darwin, aarch64-apple-darwin, x86_64-pc-windows-msvc. S-4 (dirs crate) ensures no hardcoded platform paths. |
| NFR-4 (cache durability) | Implemented | STEP-11, STEP-13 | Atomic write via `.tmp` + `std::fs::rename()`. STEP-13 tests that no `.tmp` file persists after write; valid data preserved after simulated interrupted write. |

---

## Bundle Overview

### Slice 1: Shared Infrastructure (Stage: skeleton)

**Bundle 1** — Shared Infrastructure
> Stage: skeleton | Parallel: no (foundation) | Files: Cargo.toml, src/models/steam.rs, src/models/stats.rs, src/models/mod.rs
> STEPs: 1–6 | See: [bundle-1.md](bundle-1.md)

---

### Slice 2A: Config (Stage: depth)

**Bundle 2** — Config
> Stage: depth | Parallel: yes (file-disjoint — src/config/ only; start after Bundle 1) | Files: src/config/mod.rs
> STEPs: 7–8 | See: [bundle-2.md](bundle-2.md)

---

### Slice 2B: API Client (Stage: depth)

**Bundle 3** — API Client
> Stage: depth | Parallel: yes (file-disjoint — src/api/ only; start after Bundle 1) | Files: src/api/client.rs, src/api/mod.rs
> STEPs: 9–10 | See: [bundle-3.md](bundle-3.md)

---

### Slice 2C: Snapshot Store (Stage: depth)

**Bundle 4** — Snapshot Store
> Stage: depth | Parallel: yes (file-disjoint — src/cache/ only; start after Bundle 1) | Files: src/cache/snapshot.rs, src/cache/mod.rs
> STEPs: 11–13 | See: [bundle-4.md](bundle-4.md)

---

### Slice 2D-1: TUI Widgets (Stage: depth)

**Bundle 5** — TUI Widgets
> Stage: depth | Parallel: yes (file-disjoint — src/tui/widgets/ only; start after Bundle 1) | Files: src/tui/widgets/period_bar.rs, src/tui/widgets/stat_card.rs, src/tui/widgets/game_row.rs, src/tui/mod.rs, src/tui/widgets/mod.rs
> STEPs: 14–18 | See: [bundle-5.md](bundle-5.md)

---

### Slice 2D-2: TUI Core (Stage: depth)

**Bundle 6** — TUI Core
> Stage: depth | Parallel: yes (file-disjoint with Bundles 2/3/4; start after Bundle 5 completes) | Files: src/tui/dashboard.rs, src/tui/app.rs
> STEPs: 19–22 | See: [bundle-6.md](bundle-6.md)

---

### Slice 3A: App Wiring (Stage: integration)

**Bundle 7** — App Wiring + Achievement Fan-out
> Stage: integration | Parallel: yes (file-disjoint with Bundle 8; start after Bundles 2/3/4/6 complete) | Files: src/main.rs (create), src/tui/app.rs (modify)
> STEPs: 23–25 | See: [bundle-7.md](bundle-7.md)

---

### Slice 3B: Distribution (Stage: integration)

**Bundle 8** — Distribution Packaging
> Stage: integration | Parallel: yes (file-disjoint with Bundle 7; start after Bundle 1 completes) | Files: .github/workflows/release.yml, install.sh, scoop/steam-stats.json, README.md
> STEPs: 26–27 | See: [bundle-8.md](bundle-8.md)

---

### Slice 3C: Cleanup (Stage: integration)

**Bundle 9** — iOS Cleanup
> Stage: integration | Parallel: no (depends on Bundle 8 — README.md must exist before SETUP.md deleted) | Files: deletes SteamStats_iOS/, SteamStats.xcodeproj/, project.yml, PROJECT_SUMMARY.md, SETUP.md; modifies CLAUDE.md
> STEPs: 28 | See: [bundle-9.md](bundle-9.md)

---

## Conflict Analysis

> Note: Covers explicitly declared file paths only. Implicit touches (Cargo.lock on any dependency addition, barrel files) may require manual sequencing during execution.

| Hot File | Touched By | Strategy |
|----------|------------|----------|
| `src/tui/app.rs` | STEP-21 create (Bundle 6), STEP-24 modify (Bundle 7) | Sequential — Bundle 7 must run after Bundle 6 ✓ |
| `src/main.rs` | STEP-23 create (Bundle 7), STEP-24 modify (Bundle 7) | Same bundle — no conflict |

**Implicit touches to watch**:
- `Cargo.lock` — generated by first `cargo build`; STEP-26 uses `--locked` which requires Cargo.lock to exist. Run `cargo build` after Bundle 1 before pushing a release tag.

---

## Execution Topology (Max Parallelism)

```
Bundle 1 (sequential)
    │
    ├── Bundle 2 (parallel) ──┐
    ├── Bundle 3 (parallel) ──┤
    ├── Bundle 4 (parallel) ──┤
    └── Bundle 5 (parallel) ──┤
                 │             │
           Bundle 6 ──────────┤  (after Bundle 5; file-disjoint with 2/3/4)
                               │
              ┌────────────────┘
              │  (after Bundles 2/3/4/6)
    ┌─────────┴──────────┐
    │                    │
Bundle 7 (parallel)  Bundle 8 (parallel)
    │                    │
    └────────────────────┘
                │  (after Bundle 8)
           Bundle 9
```

**Notes**:
- Bundles 2, 3, 4, 5 can all start simultaneously after Bundle 1 completes.
- Bundle 6 can start as soon as Bundle 5 completes, regardless of Bundles 2/3/4 state.
- Bundle 7 requires Bundles 2, 3, 4, AND 6 to all be done (wires config + api + cache + tui).
- Bundle 8 requires only Bundle 1 (Cargo.toml) and is otherwise independent.
- Bundle 9 requires Bundle 8 (README.md must exist).

---

## Architecture Decisions

See: [spec-driven/steam-stats-tui/design.md](design.md)

Key decisions affecting decomposition:
- **AD-1**: rally-tui stack (ratatui + tokio + reqwest) — informs all .rs STEPs
- **AD-2**: rally-tui event loop — informs Bundle 6 (app.rs) and Bundle 7 (main.rs)
- **AD-4**: NDJSON snapshot store — informs Bundle 4 (all cache STEPs)
- **AD-5**: Smart-incremental achievement fetch — informs Bundle 7 STEP-24
- **AD-7**: Distribution from main repo — informs Bundle 8 (no separate tap repo)
- **AD-8**: iOS cleanup last — informs Bundle 9 ordering

---

## File Structure

`tasks.md` is always an index-only document (bundle headers only, no STEP entries). All STEP detail lives in the bundle files:

    spec-driven/steam-stats-tui/tasks.md              — This file (index only)
    spec-driven/steam-stats-tui/bundle-1.md           — STEPs 1-6 (Shared Infrastructure)
    spec-driven/steam-stats-tui/bundle-2.md           — STEPs 7-8 (Config)
    spec-driven/steam-stats-tui/bundle-3.md           — STEPs 9-10 (API Client)
    spec-driven/steam-stats-tui/bundle-4.md           — STEPs 11-13 (Snapshot Store)
    spec-driven/steam-stats-tui/bundle-5.md           — STEPs 14-18 (TUI Widgets)
    spec-driven/steam-stats-tui/bundle-6.md           — STEPs 19-22 (TUI Core)
    spec-driven/steam-stats-tui/bundle-7.md           — STEPs 23-25 (App Wiring + Achievement)
    spec-driven/steam-stats-tui/bundle-8.md           — STEPs 26-27 (Distribution)
    spec-driven/steam-stats-tui/bundle-9.md           — STEP 28 (iOS Cleanup)

Progress files (executor writes only):

    spec-driven/steam-stats-tui/progress-bundle-1.md  — through progress-bundle-9.md
