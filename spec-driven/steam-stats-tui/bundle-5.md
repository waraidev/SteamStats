### Bundle 5: TUI Widgets
> Stage: depth | Parallel: yes (file-disjoint — src/tui/widgets/ only; start after Bundle 1) | Files: src/tui/widgets/period_bar.rs, src/tui/widgets/stat_card.rs, src/tui/widgets/game_row.rs, src/tui/mod.rs, src/tui/widgets/mod.rs

**Bundle Verify**: All TUI widgets render correctly on TestBackend without panics on boundary inputs.
- **Level**: unit
- **Given**: TestBackend (80×24) + constructed widget instances with known test data
- **Action**: `cargo test tui::widgets::`
- **Outcome**: All widget render tests pass; GameRow with percent=101.0 does not panic; StatCard shows "Est. Sessions" label; PeriodBar highlights selected period

> **Context**
>
> **Applicable ACs**
> - **AC-1.1**: Given: App has loaded data / When: Dashboard renders / Then: Four stat blocks shown (Games Played, Est. Sessions, Achievements, New Games) with number + label
> - **AC-1.2**: Given: At least one game has playtime / When: Dashboard renders / Then: Games listed with rank, name, %, progress bar, hours
> - **AC-2.1**: Given: Dashboard visible / When: Any state / Then: Period selector shown at top with current period highlighted
> - **AC-2.2**: Given: Dashboard loaded / When: Left/right arrows or Tab/Shift-Tab / Then: Selected period changes
>
> **Architecture Decisions**
> - **AD-3: Single-page dashboard layout with period tab bar** — Period selector via Tab/Left/Right; ratatui's LineGauge for progress bars. Stateless widgets — they receive pre-computed values as parameters and render accordingly.
>
> **Findings**
> - **F-1: rally-tui exact stack** — ratatui 0.29 widget API: `Widget` trait with `render(area: Rect, buf: &mut Buffer)`. `Layout::horizontal/vertical` for splitting. `Span`, `Line`, `Style` for text styling.
>
> **Standards**
> - **S-7**: TUI rendering tests: use ratatui::backend::TestBackend (Domain: testing | File Type: .rs)
>
> **Constraints**
> - Session count must be displayed as "Est. Sessions" (not "Sessions") — use the `LABEL_EST_SESSIONS` constant from models (Category: other | Source: spec Constraints)

---

#### STEP-14: Create src/tui/widgets/period_bar.rs
[FR-2 -> AC-2.1, AC-2.2] | create `src/tui/widgets/period_bar.rs` | Effort: S

> **Intent**: The period bar is stateless — it receives `selected: Period` as a parameter and renders the 4-label row with the selected one highlighted. ratatui widgets do not store state; state lives in `App`. The selected period highlight MUST be visually distinct (bold or color) — without styling, the user cannot tell which period is active (AC-2.1 "current period highlighted").

- `pub struct PeriodBar { pub selected: Period }` implementing `Widget`
- `fn render(self, area: Rect, buf: &mut Buffer)` — split area into 4 equal horizontal cells using `Layout::horizontal`
- Each cell renders its label with: `Style::default().bold()` for `selected`, `Style::default()` for others
- Labels from `Period::label()`: "4 Weeks", "6 Months", "This Year", "Lifetime"
- Center the label text within each cell using padding

**Pattern reference**: `../rally-tui/src/tui/`

**Verify**:
- Level: unit | Given: TestBackend 80×3, PeriodBar { selected: Period::FourWeeks } | Action: render and extract buffer content | Outcome: "4 Weeks" appears in buffer; its cell has bold modifier; other labels have no bold modifier

> **Standards**:
> - S-7: TestBackend for rendering tests

> Depends on: STEP-4, STEP-1 | Enables: STEP-17, STEP-18, STEP-19 | Parallel with: STEP-15, STEP-16

---

#### STEP-15: Create src/tui/widgets/stat_card.rs
[FR-1 -> AC-1.1] | create `src/tui/widgets/stat_card.rs` | Effort: S

> **Intent**: The sessions stat card MUST display "Est. Sessions" as its label — use `LABEL_EST_SESSIONS` from `crate::models::stats`, never a free string. A code reviewer changing "Est. Sessions" to "Sessions" would break the spec constraint without the compiler catching it. The `value: String` field handles the special case of achievements being `None` — callers pass `"—"` as the string, not `None`. This keeps the widget display-only with no business logic.

- `pub struct StatCard<'a> { pub label: &'a str, pub value: String }` implementing `Widget`
- `fn render(self, area: Rect, buf: &mut Buffer)` — render `value` (large, bold) on top half, `label` (smaller) on bottom half of the card area
- Import `LABEL_EST_SESSIONS` from `crate::models::stats` — use it in callers (dashboard.rs), not in this widget directly

**Verify**:
- Level: unit | Given: TestBackend 20×4 + StatCard { label: "Est. Sessions", value: "42" } | Action: render | Outcome: Buffer contains "Est. Sessions" and "42"
- Level: inspection | Given: Codebase | Action: `rg "\"Sessions\"" src/` | Outcome: Only "Est. Sessions" appears; bare "Sessions" label is absent

> **Standards**:
> - S-7: TestBackend

> Depends on: STEP-4, STEP-1 | Enables: STEP-17, STEP-18, STEP-19 | Parallel with: STEP-14, STEP-16

---

#### STEP-16: Create src/tui/widgets/game_row.rs
[FR-1 -> AC-1.2] | create `src/tui/widgets/game_row.rs` | Effort: S

> **Intent**: The `LineGauge` ratio must be clamped to `[0.0, 1.0]` — ratatui panics with an assertion failure if ratio > 1.0 or < 0.0. This can happen if `playtime_delta / total_playtime` rounds above 1.0 due to floating-point arithmetic. Long game names (100+ chars) must be truncated — `truncate_name(name, max_width)` prevents the name from overflowing into adjacent columns and causing visual corruption.

- `pub struct GameRow { pub rank: usize, pub name: String, pub percent: f64, pub hours: f64 }` implementing `Widget`
- `fn render(self, area: Rect, buf: &mut Buffer)` — horizontal layout: rank (4), name (flexible), percent (7), gauge (remaining - 12), hours (8)
- `LineGauge::default().ratio(self.percent.clamp(0.0, 100.0) / 100.0)` — clamp before dividing
- Truncate `name` to `name_width - 1` chars if longer, appending `'…'`
- Hours: `format!("{:.1}h", self.hours)` (1 decimal place)

**Verify**:
- Level: unit | Given: TestBackend 80×3 + GameRow { rank: 1, name: "Counter-Strike 2", percent: 45.5, hours: 123.4 } | Action: render | Outcome: Buffer contains "1", "45.5", "123.4h", and a gauge character
- Level: unit | Given: GameRow with percent: 101.0 | Action: render | Outcome: Does not panic (ratio clamped to 1.0)

> **Standards**:
> - S-7: TestBackend

> Depends on: STEP-1 | Enables: STEP-17, STEP-18, STEP-19 | Parallel with: STEP-14, STEP-15

---

#### STEP-17: Create src/tui/mod.rs + src/tui/widgets/mod.rs
MANUAL -> Structural re-exports | create `src/tui/mod.rs`, create `src/tui/widgets/mod.rs` | Effort: XS

> **Intent**: N/A — structural step.

- `src/tui/widgets/mod.rs`: `pub mod period_bar; pub mod stat_card; pub mod game_row;`
- `src/tui/mod.rs`: `pub mod app; pub mod dashboard; pub mod widgets;`; `pub use app::run_tui;`

**Verify**:
- Level: inspection | Given: Both mod files created | Action: `cargo check` | Outcome: No import errors

> Depends on: STEP-16 | Enables: STEP-18, STEP-19, STEP-21 | Parallel with: —

---

#### STEP-18: Test TUI Widgets
MANUAL -> Test for STEPs 14-16 (test-after) | modify widget files to add test mods | Effort: S

> **Intent**: Widget tests catch layout regressions — a change to column widths that pushes content out of bounds only shows in TestBackend buffer content. Long game name input (200-char) tests that truncation prevents panic; out-of-range percent (>100) tests that clamping prevents panic.

- Add `#[cfg(test)] mod tests` to each widget file (period_bar.rs, stat_card.rs, game_row.rs)
- `PeriodBar`: render each period as selected; verify that `buffer` cell at the selected label position has bold modifier set
- `StatCard`: test value and label appear; test empty string value renders without panic
- `GameRow`: test normal render; test percent=101.0 no-panic; test 200-char name truncates to fit without panic

**Verify**:
- Level: unit | Given: TestBackend instances for each widget with boundary inputs | Action: `cargo test tui::widgets::` | Outcome: All render tests pass; no panics on 200-char name, percent > 100, or percent < 0

> **Standards**:
> - S-7: TestBackend

> Depends on: STEP-14, STEP-15, STEP-16, STEP-17 | Enables: — | Parallel with: STEP-8, STEP-10, STEP-13
