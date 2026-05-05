use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{LineGauge, Paragraph, Widget};

/// One row in the games list: rank, name, percent share, progress bar, hours.
///
/// `percent` is a value 0–100. Values outside this range are clamped before
/// rendering to prevent `LineGauge` from panicking.
pub struct GameRow {
    pub rank: usize,
    pub name: String,
    pub percent: f64,
    pub hours: f64,
}

/// Truncate a game name to fit within `max_width` characters.
/// If truncation is needed, appends `'…'` (1 char) so total length == max_width.
fn truncate_name(name: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let char_count = name.chars().count();
    if char_count <= max_width {
        name.to_string()
    } else if max_width == 1 {
        "…".to_string()
    } else {
        // Take max_width-1 chars, then append ellipsis
        let truncated: String = name.chars().take(max_width - 1).collect();
        format!("{truncated}…")
    }
}

impl Widget for GameRow {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Column widths: rank=4, percent=7, hours=8 are fixed.
        // gauge gets the flex space between name and the right-side columns.
        // Layout: [rank:4] [name:flex] [percent:7] [gauge:flex] [hours:8]
        // The spec says "gauge (remaining - 12)". We model that as Min(0) flex.
        let constraints = [
            Constraint::Length(4),  // rank
            Constraint::Min(8),     // name (flexible, at least 8)
            Constraint::Length(7),  // percent
            Constraint::Min(4),     // gauge (flexible)
            Constraint::Length(8),  // hours
        ];
        let cols = Layout::horizontal(constraints).split(area);

        // Rank
        let rank_str = format!("{}", self.rank);
        Paragraph::new(Line::from(Span::raw(rank_str))).render(cols[0], buf);

        // Name — truncate to fit the name column
        let name_width = cols[1].width as usize;
        let display_name = truncate_name(&self.name, name_width);
        Paragraph::new(Line::from(Span::raw(display_name))).render(cols[1], buf);

        // Percent — e.g. "45.5%"
        let pct_str = format!("{:.1}%", self.percent.clamp(0.0, 100.0));
        Paragraph::new(Line::from(Span::raw(pct_str))).render(cols[2], buf);

        // Gauge — clamp ratio to [0.0, 1.0] to prevent LineGauge panic
        let ratio = (self.percent / 100.0).clamp(0.0, 1.0);
        LineGauge::default()
            .ratio(ratio)
            .filled_style(Style::default())
            .render(cols[3], buf);

        // Hours
        let hours_str = format!("{:.1}h", self.hours);
        Paragraph::new(Line::from(Span::raw(hours_str))).render(cols[4], buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_game_row(row: GameRow) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(row, frame.area());
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buf_string(buf: &ratatui::buffer::Buffer) -> String {
        buf.content.iter().map(|c| c.symbol().to_string()).collect()
    }

    #[test]
    fn normal_render_contains_expected_fields() {
        let buf = render_game_row(GameRow {
            rank: 1,
            name: "Counter-Strike 2".to_string(),
            percent: 45.5,
            hours: 123.4,
        });
        let content = buf_string(&buf);
        assert!(content.contains("1"), "rank should appear");
        assert!(content.contains("45.5"), "percent should appear");
        assert!(content.contains("123.4h"), "hours should appear");
    }

    #[test]
    fn percent_over_100_does_not_panic() {
        // This must NOT panic — ratio is clamped to 1.0
        render_game_row(GameRow {
            rank: 1,
            name: "Some Game".to_string(),
            percent: 101.0,
            hours: 10.0,
        });
    }

    #[test]
    fn percent_negative_does_not_panic() {
        render_game_row(GameRow {
            rank: 1,
            name: "Some Game".to_string(),
            percent: -5.0,
            hours: 10.0,
        });
    }

    #[test]
    fn two_hundred_char_name_does_not_panic() {
        let long_name = "A".repeat(200);
        render_game_row(GameRow {
            rank: 1,
            name: long_name,
            percent: 50.0,
            hours: 20.0,
        });
    }

    #[test]
    fn truncate_name_adds_ellipsis() {
        let result = truncate_name("Counter-Strike 2", 8);
        assert_eq!(result.chars().count(), 8);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn truncate_name_no_truncation_needed() {
        let result = truncate_name("Short", 10);
        assert_eq!(result, "Short");
    }

    #[test]
    fn truncate_name_zero_width_is_empty() {
        let result = truncate_name("Game", 0);
        assert_eq!(result, "");
    }
}
