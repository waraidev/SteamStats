use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

// Import LABEL_EST_SESSIONS to enforce compiler-checked reference in callers.
// The widget itself renders whatever label it receives — see dashboard.rs for usage.
#[allow(unused_imports)]
pub use crate::models::stats::LABEL_EST_SESSIONS;

/// A stat card showing a label (dim, top half) then a value (bold cyan, bottom half).
///
/// Label-on-top ordering prevents visual confusion with the period bar directly above,
/// which would otherwise make "4 Weeks → 87" read as a period-specific count.
///
/// Callers must pass `LABEL_EST_SESSIONS` as the label for the sessions stat.
/// The `value` field is a `String` so callers can pass `"—"` for unavailable data.
pub struct StatCard<'a> {
    pub label: &'a str,
    pub value: String,
}

impl<'a> Widget for StatCard<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Label on top (dim), value on bottom (bold cyan) — reads as "Games Played: 87"
        let rows = Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);

        let label_line = Line::from(Span::styled(
            self.label,
            Style::default().add_modifier(Modifier::DIM),
        ));
        Paragraph::new(label_line)
            .alignment(Alignment::Center)
            .render(rows[0], buf);

        let value_line = Line::from(Span::styled(
            self.value.clone(),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ));
        Paragraph::new(value_line)
            .alignment(Alignment::Center)
            .render(rows[1], buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn buf_string(buf: &ratatui::buffer::Buffer) -> String {
        buf.content.iter().map(|c| c.symbol().to_string()).collect()
    }

    #[test]
    fn value_and_label_appear_in_buffer() {
        let backend = TestBackend::new(20, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(
                    StatCard {
                        label: LABEL_EST_SESSIONS,
                        value: "42".to_string(),
                    },
                    frame.area(),
                );
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let content = buf_string(&buf);
        assert!(content.contains("42"), "buffer should contain '42'");
        assert!(content.contains("Est. Sessions"), "buffer should contain 'Est. Sessions'");
    }

    #[test]
    fn empty_value_renders_without_panic() {
        let backend = TestBackend::new(20, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(
                    StatCard {
                        label: "Games Played",
                        value: String::new(),
                    },
                    frame.area(),
                );
            })
            .unwrap();
        // If we reach here without panic, the test passes
    }

    #[test]
    fn dash_value_renders_without_panic() {
        let backend = TestBackend::new(20, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(
                    StatCard {
                        label: LABEL_EST_SESSIONS,
                        value: "—".to_string(),
                    },
                    frame.area(),
                );
            })
            .unwrap();
    }
}
