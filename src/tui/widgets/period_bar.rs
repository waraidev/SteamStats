use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::models::stats::Period;

/// Tab bar widget that highlights the currently selected time period.
///
/// Stateless — receives `selected: Period` and renders accordingly.
/// State lives in `App`, not here.
pub struct PeriodBar {
    pub selected: Period,
}

impl Widget for PeriodBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let periods = [
            Period::FourWeeks,
            Period::SixMonths,
            Period::ThisYear,
            Period::Lifetime,
        ];

        let constraints: Vec<Constraint> = periods
            .iter()
            .map(|_| Constraint::Ratio(1, periods.len() as u32))
            .collect();

        let cells = Layout::horizontal(constraints).split(area);

        for (period, cell) in periods.iter().zip(cells.iter()) {
            let label = period.label();
            let style = if *period == self.selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Center the label by padding with spaces
            let cell_width = cell.width as usize;
            let label_len = label.len();
            let padding = if cell_width > label_len {
                (cell_width - label_len) / 2
            } else {
                0
            };
            let padded = format!("{:>width$}", label, width = label_len + padding);

            let line = Line::from(Span::styled(padded, style));
            Paragraph::new(line).render(*cell, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_period_bar(selected: Period, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(PeriodBar { selected }, frame.area());
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
        buf.content.iter().map(|c| c.symbol().to_string()).collect()
    }

    #[test]
    fn four_weeks_label_appears_in_buffer() {
        let buf = render_period_bar(Period::FourWeeks, 80, 3);
        let content = buffer_to_string(&buf);
        assert!(
            content.contains("4 Weeks"),
            "buffer should contain '4 Weeks'"
        );
    }

    #[test]
    fn selected_cell_has_bold_modifier() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(
                    PeriodBar {
                        selected: Period::FourWeeks,
                    },
                    frame.area(),
                );
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();

        // Each cell is 80/4 = 20 wide. The label "4 Weeks" (7 chars) is centered.
        // padding = (20 - 7) / 2 = 6, so label starts at x = 6
        // Search for any cell in first row (y=0) with BOLD set
        let has_bold = (0..80u16).any(|x| buf[(x, 0)].modifier.contains(Modifier::BOLD));
        assert!(has_bold, "selected period cell should have BOLD modifier");
    }

    #[test]
    fn non_selected_cells_have_no_bold() {
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(
                    PeriodBar {
                        selected: Period::FourWeeks,
                    },
                    frame.area(),
                );
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();

        // Cells 1-3 (x=20..79) should NOT have bold
        let has_bold_outside = (20..80u16).any(|x| buf[(x, 0)].modifier.contains(Modifier::BOLD));
        assert!(
            !has_bold_outside,
            "non-selected period cells should not be bold"
        );
    }

    #[test]
    fn all_period_labels_appear() {
        for period in [
            Period::FourWeeks,
            Period::SixMonths,
            Period::ThisYear,
            Period::Lifetime,
        ] {
            let buf = render_period_bar(period, 80, 3);
            let content = buffer_to_string(&buf);
            assert!(
                content.contains(period.label()),
                "buffer should contain '{}'",
                period.label()
            );
        }
    }
}
